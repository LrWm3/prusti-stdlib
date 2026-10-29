//! Native Ferrum backend, invoked on the verification owner thread.
use crate::ViperBackendConfig;
use prusti_ferrum::silver::{
    ast::program::Program,
    reporter::reporter::NoopReporter,
    verifier::verification_result::{AbstractError, FrontendErrorKind, VerificationResult},
};
use std::sync::Arc;
use viper::{ConsistencyError, VerificationError, VerificationResultKind};

struct NativeSmtManager(viper::smt_manager::SmtManager);
impl NativeSmtManager {
    fn finish(&mut self) -> Result<(), String> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.0.try_stop_and_check()))
        {
            Ok(result) => result.map_err(|error| format!("SMT manager cleanup: {error}")),
            Err(error) => {
                let detail = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic");
                Err(format!("SMT manager cleanup panicked: {detail}"))
            }
        }
    }
}
impl Drop for NativeSmtManager {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn source_consistency(program: &Program) -> Result<Vec<ConsistencyError>, String> {
    use prusti_ferrum::silver::ast::{
        expression::{Exp, ExpKind},
        pretty::pretty_printer::{expression_to_string, pretty_node, show_type, PrintError},
        utility::{consistency::check_transitively, nodes::Node},
    };
    let render = |node: &Node| -> Result<String, PrintError> {
        match node {
            Node::Exp(exp) => expression_to_string(exp),
            Node::Type(typ) => show_type(typ),
            Node::LocalVar(var) => {
                expression_to_string(&Exp::new(ExpKind::LocalVar(var.clone()), var.meta.clone()))
            }
            Node::LocalDecl(decl) => Ok(format!("{}: {}", decl.name, show_type(&decl.typ)?)),
            Node::UnnamedLocalDecl(decl) => show_type(&decl.typ),
            Node::FieldAccess(access) => expression_to_string(&Exp::new(
                ExpKind::FieldAccess(access.clone()),
                access.meta.clone(),
            )),
            Node::PredicateAccess(access) => expression_to_string(&Exp::new(
                ExpKind::PredicateAccess(access.clone()),
                access.meta.clone(),
            )),
            Node::PredicateAccessPredicate(access) => expression_to_string(&Exp::new(
                ExpKind::PredicateAccessPredicate(access.clone()),
                access.meta.clone(),
            )),
            _ => pretty_node(node),
        }
    };
    let render_seq = |_: &[Node]| -> Result<String, PrintError> {
        Err(PrintError::Unsupported("consistency sequence rendering"))
    };
    let errors = check_transitively(program, &render, &render_seq)
        .map_err(|e| format!("native consistency check: {e:?}"))?;
    errors
        .into_iter()
        .map(|error| {
            Ok(ConsistencyError {
                pos_id: prusti_ferrum::position_id(&error.position),
                message: error
                    .readable_message()
                    .map_err(|e| format!("consistency diagnostic: {e:?}"))?,
            })
        })
        .collect()
}

pub(crate) fn verify(
    program: Program,
    config: ViperBackendConfig,
    procedures: prusti_rustc_interface::data_structures::fx::FxHashSet<String>,
    sender: std::sync::mpsc::Sender<crate::ServerMessage>,
) -> VerificationResultKind {
    verify_internal(
        program,
        config,
        procedures,
        sender,
        prusti_utils::config::report_viper_messages(),
    )
}

fn verify_internal(
    program: Program,
    config: ViperBackendConfig,
    procedures: prusti_rustc_interface::data_structures::fx::FxHashSet<String>,
    sender: std::sync::mpsc::Sender<crate::ServerMessage>,
    allow_reporting: bool,
) -> VerificationResultKind {
    fn run(
        program: Program,
        config: ViperBackendConfig,
        procedures: prusti_rustc_interface::data_structures::fx::FxHashSet<String>,
        sender: std::sync::mpsc::Sender<crate::ServerMessage>,
        allow_reporting: bool,
    ) -> Result<VerificationResultKind, String> {
        if !config.verify_mode.is_verify() && !config.verify_mode.is_consistency_check() {
            return Err("Ferrum requires verify or consistency-check mode".into());
        }
        let mut args = config.verifier_args;
        let solver = if prusti_utils::config::use_smt_wrapper() {
            let log_path = prusti_utils::config::log_dir()
                .join("smt")
                .join(prusti_utils::report::log::to_legal_file_name("program"));
            std::fs::create_dir_all(&log_path).map_err(|e| format!("SMT log directory: {e}"))?;
            std::env::set_var(
                "PRUSTI_ORIGINAL_SMT_SOLVER_PATH",
                prusti_utils::config::smt_solver_path(),
            );
            let solver = prusti_utils::config::smt_solver_wrapper_path();
            let manager = NativeSmtManager(viper::smt_manager::SmtManager::new(
                log_path,
                prusti_utils::config::preserve_smt_trace_files(),
                prusti_utils::config::write_smt_statistics(),
                prusti_utils::config::smt_qi_ignore_builtin(),
                prusti_utils::config::smt_qi_bound_global_kind(),
                prusti_utils::config::smt_qi_bound_trace(),
                prusti_utils::config::smt_qi_bound_trace_kind(),
                prusti_utils::config::smt_unique_triggers_bound(),
                prusti_utils::config::smt_unique_triggers_bound_total(),
            ));
            std::env::set_var(
                "PRUSTI_SMT_SOLVER_MANAGER_PORT",
                manager.0.port().to_string(),
            );
            if prusti_utils::config::log_smt_wrapper_interaction() {
                std::env::set_var("PRUSTI_LOG_SMT_INTERACTION", "true");
            }
            (solver, Some(manager))
        } else {
            (prusti_utils::config::smt_solver_path(), None)
        };
        if let Some(bound) = prusti_utils::config::smt_qi_bound_global() {
            std::env::set_var("PRUSTI_SMT_QI_BOUND_GLOBAL", bound.to_string());
        }
        let (solver, mut smt_manager) = solver;
        let outcome = (|| -> Result<VerificationResultKind, String> {
            args.extend(["--z3Exe".into(), solver]);
            let debug_dir = if prusti_utils::config::dump_debug_info() {
                let path = prusti_utils::config::log_dir()
                    .join("viper_tmp")
                    .join(prusti_utils::report::log::to_legal_file_name("program"));
                std::fs::create_dir_all(&path).map_err(|e| format!("debug directory: {e}"))?;
                let path_string = path.to_str().ok_or("debug directory is not UTF-8")?;
                args.extend([
                    "--tempDirectory".into(),
                    path_string.into(),
                    "--printMethodCFGs".into(),
                ]);
                Some(path)
            } else {
                None
            };
            let report = allow_reporting.then(|| {
                Arc::new(crate::native_reporter::NativeReporter {
                    procedures: procedures.into_iter().collect(),
                    sender,
                    errors: std::sync::Mutex::new(Default::default()),
                })
            });
            let progress: Arc<dyn prusti_ferrum::silver::reporter::reporter::Reporter> =
                match &report {
                    Some(report) => report.clone(),
                    None => Arc::new(NoopReporter),
                };
            let target: Arc<dyn prusti_ferrum::silver::reporter::reporter::Reporter> =
                if let Some(path) = debug_dir {
                    let csv = prusti_ferrum::silver::reporter::reporter::CsvReporter::open(
                        path.join("report.csv"),
                    )
                    .map_err(|e| format!("CSV reporter: {e}"))?;
                    Arc::new(crate::native_reporter::DebugReporter {
                        progress,
                        csv: Arc::new(csv),
                    })
                } else {
                    progress
                };
            let reporter: Arc<dyn prusti_ferrum::silver::reporter::reporter::Reporter> = Arc::new(
                prusti_ferrum::silver::reporter::reporter::PluginAwareReporter {
                    name: "polling_reporter".into(),
                    target,
                    mapper: std::sync::RwLock::new(None),
                },
            );
            let warnings = Arc::new(std::sync::Mutex::new(Vec::new()));
            let mut frontend = match prusti_ferrum::native_frontend_with_warning_sink(
                &args,
                reporter,
                warnings.clone(),
            ) {
                Ok(frontend) => frontend,
                Err(error) => {
                    for warning in
                        std::mem::take(&mut *warnings.lock().unwrap_or_else(|e| e.into_inner()))
                    {
                        log::warn!("Ferrum: {warning}");
                    }
                    return Err(format!("native frontend: {error}"));
                }
            };
            let checked = source_consistency(&program);
            if config.verify_mode.is_consistency_check()
                || !matches!(&checked, Ok(errors) if errors.is_empty())
            {
                let outcome = checked.map(|errors| {
                    if errors.is_empty() {
                        VerificationResultKind::Success
                    } else {
                        VerificationResultKind::ConsistencyErrors(errors)
                    }
                });
                for warning in frontend.take_warnings() {
                    log::warn!("Ferrum: {warning}");
                }
                let stop = frontend.stop().map_err(|e| format!("shutdown: {e}"));
                for warning in frontend.take_warnings() {
                    log::warn!("Ferrum: {warning}");
                }
                return match (outcome, stop) {
                    (Ok(result), Ok(())) => Ok(result),
                    (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
                    (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
                };
            }
            let result = frontend
                .verify(&program)
                .map_err(|e| format!("verification: {e}"));
            for warning in frontend.take_warnings() {
                log::warn!("Ferrum: {warning}");
            }
            let stop = frontend.stop().map_err(|e| format!("shutdown: {e}"));
            for warning in frontend.take_warnings() {
                log::warn!("Ferrum: {warning}");
            }
            let result = match (result, stop) {
                (Ok(result), Ok(())) => result,
                (Err(e), Ok(())) | (Ok(_), Err(e)) => return Err(e),
                (Err(e), Err(cleanup)) => return Err(format!("{e}; {cleanup}")),
            };
            match result {
                VerificationResult::Success => Ok(VerificationResultKind::Success),
                VerificationResult::Failure(errors) => {
                    let mut verification = Vec::new();
                    let mut had_verification = false;
                    let mut consistency = Vec::new();
                    for error in errors {
                        match error {
                            AbstractError::Verification(error) => {
                                had_verification = true;
                                let mapped = prusti_ferrum::verification_error(error)
                                    .map_err(|e| format!("diagnostic mapping: {e:?}"))?;
                                let mut error = VerificationError::new(
                                    mapped.full_id,
                                    mapped.pos_id,
                                    mapped.offending_pos_id,
                                    mapped.reason_pos_id,
                                    mapped.message,
                                    None,
                                );
                                if let Some(report) = &report {
                                    let mut hashes = report
                                        .errors
                                        .lock()
                                        .map_err(|_| "poisoned reporter error set")?;
                                    if !hashes.insert(
                                        crate::native_reporter::NativeReporter::error_hash(&error),
                                    ) {
                                        continue;
                                    }
                                }
                                if prusti_utils::config::counterexample() {
                                    error.counterexample =
                                        prusti_ferrum::counterexample::from_diagnostic(
                                            &mapped.diagnostic,
                                        )?;
                                }
                                verification.push(error);
                            }
                            AbstractError::Frontend(error) => {
                                if let FrontendErrorKind::TimeoutOccurred {
                                    duration,
                                    ref units,
                                } = error.kind
                                {
                                    if duration < 0 || units != "second(s)" {
                                        return Err(format!(
                                            "unsupported timeout duration: {duration} {units}"
                                        ));
                                    }
                                    return Ok(VerificationResultKind::Timeout(duration as u64));
                                }
                                consistency.push(ConsistencyError {
                                    pos_id: prusti_ferrum::position_id(&error.position),
                                    message: error
                                        .readable_message()
                                        .map_err(|e| format!("diagnostic mapping: {e:?}"))?,
                                });
                            }
                            AbstractError::Parse(error) => consistency.push(ConsistencyError {
                                pos_id: prusti_ferrum::position_id(error.pos()),
                                message: error.readable_message(),
                            }),
                        }
                    }
                    if had_verification && !consistency.is_empty() {
                        return Err("mixed frontend and verification errors".into());
                    }
                    if had_verification {
                        Ok(VerificationResultKind::Failure(verification))
                    } else {
                        Ok(VerificationResultKind::ConsistencyErrors(consistency))
                    }
                }
            }
        })();
        let cleanup = match smt_manager.as_mut() {
            Some(manager) => manager.finish(),
            None => Ok(()),
        };
        match (outcome, cleanup) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), Ok(())) | (Ok(VerificationResultKind::Success), Err(error)) => Err(error),
            (Err(primary), Err(cleanup)) => Err(format!("{primary}; {cleanup}")),
            (Ok(primary), Err(cleanup)) => {
                Err(format!("verification result: {primary:?}; {cleanup}"))
            }
        }
    }
    run(program, config, procedures, sender, allow_reporting)
        .unwrap_or_else(VerificationResultKind::BackendError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prusti_ferrum::silver::ast::{
        ast::Metadata,
        expression::{Exp, ExpKind},
        program::{Method, MethodData, Program},
        statement::{Stmt, StmtKind},
    };
    #[test]
    #[ignore = "requires real Z3, PRUSTI_DUMP_DEBUG_INFO=true and a temporary PRUSTI_LOG_DIR"]
    fn configured_debug_output_writes_report_and_cfg() {
        assert!(prusti_utils::config::dump_debug_info());
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::env::var_os("PRUSTI_LOG_DIR").expect("set a temporary PRUSTI_LOG_DIR");
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let m = Method::new(MethodData {
                    name: "debug_fixture".into(),
                    formal_args: vec![],
                    formal_returns: vec![],
                    pres: vec![],
                    posts: vec![],
                    body: Some(Stmt::new(
                        StmtKind::Assert(Exp::new(ExpKind::TrueLit, Metadata::default())),
                        Metadata::default(),
                    )),
                    meta: Metadata::default(),
                });
                let program = Program::new(
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    vec![m],
                    vec![],
                    Metadata::default(),
                );
                let config = ViperBackendConfig::new(
                    viper::VerificationBackend::Ferrum,
                    prusti_utils::config::VerifyMode::Verify,
                );
                let (sender, _receiver) = std::sync::mpsc::channel();
                let result = verify(program, config, Default::default(), sender);
                assert!(
                    matches!(result, VerificationResultKind::Success),
                    "{result:?}"
                );
                let dir = prusti_utils::config::log_dir().join("viper_tmp/program");
                assert!(dir.join("debug_fixture.dot").is_file());
                let report = std::fs::read_to_string(dir.join("report.csv")).unwrap();
                assert!(
                    report.contains("EntitySuccessMessage,debug_fixture"),
                    "{report}"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set Z3_EXE"]
    fn consistency_mode_checks_without_solving_assertions() {
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let method = |name: &str| {
                    Method::new(MethodData {
                        name: name.into(),
                        formal_args: vec![],
                        formal_returns: vec![],
                        pres: vec![],
                        posts: vec![],
                        body: Some(Stmt::new(
                            StmtKind::Assert(Exp::new(ExpKind::FalseLit, Metadata::default())),
                            Metadata::default(),
                        )),
                        meta: Metadata::default(),
                    })
                };
                let program = |methods| {
                    Program::new(
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        methods,
                        vec![],
                        Metadata::default(),
                    )
                };
                let config = ViperBackendConfig::new(
                    viper::VerificationBackend::Ferrum,
                    prusti_utils::config::VerifyMode::ConsistencyCheck,
                );
                let (sender, _receiver) = std::sync::mpsc::channel();
                let good = verify(
                    program(vec![method("m")]),
                    config.clone(),
                    Default::default(),
                    sender.clone(),
                );
                assert!(matches!(good, VerificationResultKind::Success), "{good:?}");
                let bad = verify(
                    program(vec![method("m"), method("m")]),
                    config,
                    Default::default(),
                    sender,
                );
                assert!(
                    matches!(bad, VerificationResultKind::ConsistencyErrors(e) if !e.is_empty()),
                    "duplicate names not rejected"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3 and PRUSTI_REPORT_VIPER_MESSAGES=true"]
    fn configured_reporter_emits_and_deduplicates_selected_method() {
        assert!(prusti_utils::config::report_viper_messages());
        assert!(prusti_utils::config::counterexample());
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            let m = Method::new(MethodData { name: "bad".into(), formal_args: vec![], formal_returns: vec![], pres: vec![], posts: vec![], body: Some(Stmt::new(StmtKind::Assert(Exp::new(ExpKind::FalseLit, Metadata::default())), Metadata::default())), meta: Metadata::default() });
            let program = Program::new(vec![],vec![],vec![],vec![],vec![m],vec![],Metadata::default());
            let config = ViperBackendConfig::new(viper::VerificationBackend::Ferrum, prusti_utils::config::VerifyMode::Verify);
            let (sender,receiver) = std::sync::mpsc::channel();
            let result = verify(program,config,["bad".to_string()].into_iter().collect(),sender);
            assert!(matches!(result, VerificationResultKind::Failure(e) if e.is_empty()));
            assert!(matches!(receiver.try_recv().unwrap(), crate::ServerMessage::MethodTermination { viper_method_name, result: VerificationResultKind::Failure(e), .. } if viper_method_name == "bad" && e.len()==1 && e[0].counterexample.is_some()));
        }).unwrap().join().unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set Z3_EXE"]
    fn report_path_emits_method_and_deduplicates_final_failure() {
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            let m = Method::new(MethodData { name: "bad".into(), formal_args: vec![], formal_returns: vec![], pres: vec![], posts: vec![], body: Some(Stmt::new(StmtKind::Assert(Exp::new(ExpKind::FalseLit, Metadata::default())), Metadata::default())), meta: Metadata::default() });
            let program = Program::new(vec![],vec![],vec![],vec![],vec![m],vec![],Metadata::default());
            let config = ViperBackendConfig::new(viper::VerificationBackend::Ferrum, prusti_utils::config::VerifyMode::Verify);
            let (sender,receiver) = std::sync::mpsc::channel();
            let result = verify_internal(program,config,["bad".to_string()].into_iter().collect(),sender,true);
            assert!(matches!(result, VerificationResultKind::Failure(e) if e.is_empty()), "final error was not deduplicated");
            assert!(matches!(receiver.try_recv().unwrap(), crate::ServerMessage::MethodTermination { viper_method_name, result: VerificationResultKind::Failure(e), .. } if viper_method_name == "bad" && e.len()==1));
            assert!(receiver.try_recv().is_err());
        }).unwrap().join().unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set Z3_EXE"]
    fn native_server_verifies_and_maps_failure() {
        std::env::var_os("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let make = |value| {
                    let assertion = Stmt::new(
                        StmtKind::Assert(Exp::new(
                            if value {
                                ExpKind::TrueLit
                            } else {
                                ExpKind::FalseLit
                            },
                            Metadata::default(),
                        )),
                        Metadata::default(),
                    );
                    let method = Method::new(MethodData {
                        name: "test".into(),
                        formal_args: vec![],
                        formal_returns: vec![],
                        pres: vec![],
                        posts: vec![],
                        body: Some(assertion),
                        meta: Metadata::default(),
                    });
                    Program::new(
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        vec![method],
                        vec![],
                        Metadata::default(),
                    )
                };
                let config = ViperBackendConfig::new(
                    viper::VerificationBackend::Ferrum,
                    prusti_utils::config::VerifyMode::Verify,
                );
                let (sender, _receiver) = std::sync::mpsc::channel();
                let good = verify(
                    make(true),
                    config.clone(),
                    Default::default(),
                    sender.clone(),
                );
                assert!(matches!(good, VerificationResultKind::Success), "{good:?}");
                let result = verify(make(false), config, Default::default(), sender);
                assert!(
                    matches!(result, VerificationResultKind::Failure(_)),
                    "{result:?}"
                );
                if prusti_utils::config::counterexample() {
                    let VerificationResultKind::Failure(errors) = result else {
                        unreachable!()
                    };
                    assert!(
                        errors.iter().any(|error| error.counterexample.is_some()),
                        "mapped counterexample missing"
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
