// © 2021, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[cfg(feature = "jvm")]
use crate::Backend;
use crate::ServerMessage;
#[cfg(feature = "jvm")]
use log::info;
use prusti_rustc_interface::data_structures::fx::FxHashSet;
#[cfg(feature = "jvm")]
use prusti_utils::report::log::{report, to_legal_file_name};
use prusti_utils::{config, config::VerifyMode, Stopwatch};
use std::sync::{self, mpsc};
#[cfg(feature = "jvm")]
use std::{fs::create_dir_all, path::PathBuf, sync::OnceLock};
use viper::{self, VerificationBackend, VerificationResult, VerificationResultKind};
#[cfg(feature = "jvm")]
use viper::{smt_manager::SmtManager, Viper};

/// The JVM object should only instantiated once, so it is stored in a
/// global reference, shareable between threads and execution of different
/// requests when running the server. No Mutex is used because
/// the value should only ever be used through non mutable references.
#[cfg(feature = "jvm")]
pub(crate) static VIPER: OnceLock<Viper> = OnceLock::new();

/// Server requests that are sent between client and server
/// These are oblivious to the backend being used for verification
/// and could potentially be something other than verification.
pub(crate) enum ServerRequest {
    Verification(ServerVerificationRequest),
}

/// Server requests that are sent between threads of the verifying process.
/// Specifies the kind of backend to be used for verification and carries necessary data.
pub(crate) enum ServerVerificationRequest {
    // viper program, backend config, set of viper identifiers
    #[cfg(feature = "jvm")]
    JVMViperRequest(
        jni::objects::GlobalRef,
        ViperBackendConfig,
        FxHashSet<String>,
    ),
    #[cfg(feature = "ferrum")]
    FerrumRequest(
        Result<prusti_ferrum::silver::ast::program::Program, String>,
        ViperBackendConfig,
        FxHashSet<String>,
    ),
    #[cfg(not(all(feature = "jvm", feature = "ferrum")))]
    Unavailable(String),
}

impl ServerVerificationRequest {
    /// Process and consume the request
    // FIXME: can we do without the "program" strings?
    pub fn process<'v, 't: 'v>(self, sender: &mpsc::Sender<ServerMessage>) {
        #[allow(unused_mut)]
        let mut stopwatch = Stopwatch::start("prusti-server", "verifier startup");
        let mut result = VerificationResult {
            item_name: "program".to_string(),
            kind: VerificationResultKind::Success,
            cached: false,
            time_ms: 0,
        };

        match self {
            #[cfg(feature = "jvm")]
            ServerVerificationRequest::JVMViperRequest(
                viper_program_ref,
                backend_config,
                procedures,
            ) => {
                let viper = VIPER.get().expect("ServerVerificationRequest: Viper was not instantiated before processing a request");
                let verification_context = viper.attach_current_thread();
                let mut backend = match backend_config.backend {
                    VerificationBackend::Carbon | VerificationBackend::Silicon => Backend::Viper(
                        new_viper_verifier(
                            "program",
                            &verification_context,
                            backend_config.clone(),
                        ),
                        &verification_context,
                        viper_program_ref,
                    ),
                    VerificationBackend::Ferrum => {
                        result.kind = VerificationResultKind::BackendError(
                            "Ferrum backend received a JVM request".into(),
                        );
                        sender.send(ServerMessage::Termination(result)).unwrap();
                        return;
                    }
                };
                stopwatch.start_next("backend verification");
                result.kind = {
                    let _phase = crate::bench_phase::Phase::start("backend verification");
                    backend.verify(procedures, sender.clone())
                };
                result.time_ms = stopwatch.finish().as_millis();
            }
            #[cfg(feature = "ferrum")]
            ServerVerificationRequest::FerrumRequest(program, config, procedures) => {
                result.kind = match program {
                    Ok(program) => {
                        crate::native_backend::verify(program, config, procedures, sender.clone())
                    }
                    Err(error) => VerificationResultKind::BackendError(error),
                };
                result.time_ms = stopwatch.finish().as_millis();
            }
            #[cfg(not(all(feature = "jvm", feature = "ferrum")))]
            ServerVerificationRequest::Unavailable(error) => {
                result.kind = VerificationResultKind::BackendError(error);
            }
        }

        /*normalization_info.denormalize_result(&mut result.kind);*/
        sender.send(ServerMessage::Termination(result)).unwrap();
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationRequest {
    pub program: vir::ProgramRef,
    pub procedures: FxHashSet<String>,
    pub backend_config: ViperBackendConfig,
}

impl VerificationRequest {
    pub(crate) fn get_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.program.get_hash().hash(&mut hasher);
        self.backend_config.hash(&mut hasher);
        hasher.finish()
    }

    /// Builds a more specific request based on the backend configuration and sends it.
    /// This includes the vir-viper translation if the Viper backend is used.
    pub(crate) fn send(&self, mtx_tx_verreq: &sync::Mutex<mpsc::Sender<ServerRequest>>) {
        let request = self.build_request();

        #[cfg(feature = "ferrum")]
        if self.backend_config.verify_mode.is_encode_only() {
            if let ServerVerificationRequest::FerrumRequest(Err(error), ..) = &request {
                panic!("{error}");
            }
        }
        if !self.backend_config.verify_mode.is_encode_only() {
            mtx_tx_verreq
                .lock()
                .unwrap()
                .send(ServerRequest::Verification(request))
                .unwrap();
        }
    }

    fn build_request(&self) -> ServerVerificationRequest {
        match self.backend_config.backend {
            VerificationBackend::Ferrum => {
                #[cfg(feature = "ferrum")]
                {
                    let program = {
                        let _phase = crate::bench_phase::Phase::start("VIR conversion");
                        vir::with_vcx(|vcx| {
                            prusti_ferrum::program_to_silver(vcx.get_program(self.program))
                                .map_err(|e| format!("VIR conversion: {e:?}"))
                        })
                    }
                    .and_then(|program| {
                        if config::dump_viper_program() {
                            let rendered =
                                prusti_ferrum::silver::ast::pretty::pretty_printer::pretty_node(
                                    &prusti_ferrum::silver::ast::utility::nodes::Node::Program(
                                        program.clone(),
                                    ),
                                )
                                .map_err(|e| format!("Viper program printing: {e:?}"))?;
                            let namespace = "viper_program";
                            let filename =
                                format!("{}.vpr", self.program.get_name_with_check_mode());
                            log::info!("Dumping Viper program to '{namespace}/{filename}'");
                            prusti_utils::report::log::report(namespace, filename, rendered);
                        }
                        Ok(program)
                    });
                    ServerVerificationRequest::FerrumRequest(
                        program,
                        self.backend_config.clone(),
                        self.procedures.clone(),
                    )
                }
                #[cfg(not(feature = "ferrum"))]
                {
                    ServerVerificationRequest::Unavailable("Ferrum backend not built".into())
                }
            }
            #[cfg(not(feature = "jvm"))]
            VerificationBackend::Carbon | VerificationBackend::Silicon => {
                ServerVerificationRequest::Unavailable("JVM backend not built".into())
            }
            #[cfg(feature = "jvm")]
            VerificationBackend::Carbon | VerificationBackend::Silicon => {
                let mut stopwatch = Stopwatch::start("prusti-server backend", "JVM startup");
                let viper = VIPER.get_or_init(|| {
                    Viper::new_with_args(&config::viper_home(), config::extra_jvm_args())
                });
                stopwatch.start_next("attach current thread to the JVM");
                let context = viper.attach_current_thread();
                let ast_utils = context.new_ast_utils();

                stopwatch.start_next("construction of JVM objects");
                ast_utils.with_local_frame(16, || {
                    let ast_factory = context.new_ast_factory();

                    let viper_program = {
                        let _phase = crate::bench_phase::Phase::start("VIR conversion");
                        vir::with_vcx(|vcx| {
                            let program = vcx.get_program(self.program);
                            prusti_viper::program_to_viper(program, &ast_factory)
                        })
                    };

                    if config::dump_viper_program() {
                        stopwatch.start_next("dumping viper program");
                        dump_viper_program(
                            &ast_utils,
                            viper_program,
                            self.program.get_name_with_check_mode(),
                        );
                    }

                    let viper_program_ref = context
                        .env()
                        .new_global_ref(viper_program.to_jobject())
                        .unwrap();

                    ServerVerificationRequest::JVMViperRequest(
                        viper_program_ref,
                        self.backend_config.clone(),
                        self.procedures.clone(),
                    )
                })
            }
        }
    }
}

/// The configuration for the viper backend, (i.e. verifier).
/// Expresses which backend (silicon or carbon) should be used, and provides command-line arguments
/// to the viper verifier.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct ViperBackendConfig {
    pub backend: VerificationBackend,
    pub verifier_args: Vec<String>,
    pub verify_mode: VerifyMode,
}

impl ViperBackendConfig {
    pub fn new(backend: VerificationBackend, verify_mode: VerifyMode) -> Self {
        let mut verifier_args = config::extra_verifier_args();
        match backend {
            VerificationBackend::Silicon | VerificationBackend::Ferrum => {
                verifier_args.push(format!(
                    "--numberOfErrorsToReport={}",
                    config::num_errors_per_function()
                ));
                if config::use_more_complete_exhale() {
                    verifier_args.push("--enableMoreCompleteExhale".to_string());
                }
                if config::counterexample() {
                    verifier_args.push("--counterexample".to_string());
                    verifier_args.push("mapped".to_string());
                }
                if let Some(number) = config::number_of_parallel_verifiers() {
                    verifier_args.push("--numberOfParallelVerifiers".to_string());
                    verifier_args.push(number.to_string());
                }

                verifier_args.extend(vec![
                    "--disableTerminationPlugin".to_string(),
                    "--assertTimeout".to_string(),
                    config::assert_timeout().to_string(),
                    "--proverConfigArgs".to_string(),
                ]);
                // model.partial changes the default case of functions in counterexamples
                // to #unspecified
                // arith.solver=6 overrides Silicon's legacy arithmetic solver (2),
                // which is incomplete on the `mod` terms of checked arithmetic
                let mut prover_args = format!(
                    "smt.qi.eager_threshold={} model.partial={} smt.arith.solver=6",
                    config::smt_qi_eager_threshold(),
                    config::counterexample()
                );

                if let Some(smt_qi_profile) = config::smt_qi_profile() {
                    prover_args = format!("{prover_args} smt.qi.profile={smt_qi_profile}");
                }
                if let Some(smt_qi_profile_freq) = config::smt_qi_profile_freq() {
                    prover_args =
                        format!("{prover_args} smt.qi.profile_freq={smt_qi_profile_freq}");
                }

                verifier_args.push(prover_args);

                // Keep backend informational output quiet for both implementations.
                verifier_args.extend(vec!["--logLevel".to_string(), "ERROR".to_string()]);

                if let Some(check_timeout) = config::check_timeout() {
                    verifier_args.push("--checkTimeout".to_string());
                    verifier_args.push(check_timeout.to_string());
                }

                if config::report_block_messages() && config::report_viper_messages() {
                    verifier_args.push("--generateBlockMessages".to_string());
                }
            }
            VerificationBackend::Carbon => {
                verifier_args.extend(vec!["--disableAllocEncoding".to_string()]);
            }
        }
        Self {
            backend,
            verifier_args,
            verify_mode,
        }
    }
}

#[cfg(feature = "jvm")]
fn new_viper_verifier<'v, 't: 'v>(
    program_name: &str,
    verification_context: &'v viper::VerificationContext<'t>,
    backend_config: ViperBackendConfig,
) -> viper::Verifier<'v> {
    let mut verifier_args: Vec<String> = backend_config.verifier_args;
    let report_path: Option<PathBuf>;
    if config::dump_debug_info() {
        let log_path = config::log_dir()
            .join("viper_tmp")
            .join(to_legal_file_name(program_name));
        create_dir_all(&log_path).unwrap();
        report_path = Some(log_path.join("report.csv"));
        let log_dir_str = log_path.to_str().unwrap();
        match backend_config.backend {
            VerificationBackend::Silicon => {
                verifier_args.extend(vec![
                    "--tempDirectory".to_string(),
                    log_dir_str.to_string(),
                    "--printMethodCFGs".to_string(),
                    //"--printTranslatedProgram".to_string(),
                ])
            }
            VerificationBackend::Ferrum => unreachable!("native backend does not use JVM verifier"),
            VerificationBackend::Carbon => verifier_args.extend(vec![
                "--boogieOpt".to_string(),
                format!("/logPrefix {log_dir_str}"),
                //"--print".to_string(), "./log/boogie_program/program.bpl".to_string(),
            ]),
        }
    } else {
        report_path = None;
        if backend_config.backend == VerificationBackend::Silicon {
            // TODO: unknown option?
            // verifier_args.extend(vec!["--disableTempDirectory".to_string()]);
        }
    }
    let (smt_solver, smt_manager) = if config::use_smt_wrapper() {
        std::env::set_var("PRUSTI_ORIGINAL_SMT_SOLVER_PATH", config::smt_solver_path());
        let log_path = config::log_dir()
            .join("smt")
            .join(to_legal_file_name(program_name));
        create_dir_all(&log_path).unwrap();
        let smt_manager = SmtManager::new(
            log_path,
            config::preserve_smt_trace_files(),
            config::write_smt_statistics(),
            config::smt_qi_ignore_builtin(),
            config::smt_qi_bound_global_kind(),
            config::smt_qi_bound_trace(),
            config::smt_qi_bound_trace_kind(),
            config::smt_unique_triggers_bound(),
            config::smt_unique_triggers_bound_total(),
        );
        std::env::set_var(
            "PRUSTI_SMT_SOLVER_MANAGER_PORT",
            smt_manager.port().to_string(),
        );
        if config::log_smt_wrapper_interaction() {
            std::env::set_var("PRUSTI_LOG_SMT_INTERACTION", "true");
        }
        (config::smt_solver_wrapper_path(), smt_manager)
    } else {
        (config::smt_solver_path(), SmtManager::default())
    };
    let boogie_path = config::boogie_path();
    if let Some(bound) = config::smt_qi_bound_global() {
        // We need to set the environment variable to reach our Z3 wrapper.
        std::env::set_var("PRUSTI_SMT_QI_BOUND_GLOBAL", bound.to_string());
    }

    verification_context.new_verifier(
        backend_config.backend,
        verifier_args,
        report_path,
        smt_solver,
        boogie_path,
        smt_manager,
        backend_config.verify_mode,
    )
}

#[cfg(feature = "jvm")]
pub fn dump_viper_program(
    ast_utils: &viper::AstUtils,
    program: viper::Program,
    program_name: &str,
) {
    let namespace = "viper_program";
    let filename = format!("{program_name}.vpr");
    info!("Dumping Viper program to '{namespace}/{filename}'");
    report(namespace, filename, ast_utils.pretty_print(program));
}

#[cfg(all(test, feature = "ferrum"))]
mod native_tests {
    use super::*;
    #[test]
    #[ignore = "requires real Z3; set Z3_EXE"]
    fn vir_request_uses_native_queue_and_returns_failure() {
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let program = vir::with_vcx(|vcx| {
                    let assertion = vcx.mk_assert_stmt(vcx.mk_bool::<false>());
                    let label = vcx.alloc(vir::CfgBlockLabelData::End);
                    let end = vcx.alloc(vir::TerminatorStmtData::Exit);
                    let block = vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[assertion]), end);
                    let method = vcx.mk_method_unchecked(
                        "bad",
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(vcx.alloc_slice(&[block])),
                    );
                    vcx.mk_program(&[], &[], &[], &[], &[], vcx.alloc_slice(&[method]))
                        .to_ref()
                });
                let request = VerificationRequest {
                    program,
                    procedures: Default::default(),
                    backend_config: ViperBackendConfig::new(
                        VerificationBackend::Ferrum,
                        VerifyMode::Verify,
                    ),
                };
                let request = request.build_request();
                let (tx, rx) = std::sync::mpsc::channel();
                request.process(&tx);
                let ServerMessage::Termination(result) = rx.recv().unwrap() else {
                    panic!("expected termination");
                };
                assert!(
                    matches!(result.kind, VerificationResultKind::Failure(_)),
                    "{:?}",
                    result.kind
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
