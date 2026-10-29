//! Native Ferrum backend, invoked on the verification owner thread.
use crate::ViperBackendConfig;
use prusti_ferrum::silver::{
    ast::program::Program,
    reporter::reporter::NoopReporter,
    verifier::verification_result::{AbstractError, FrontendErrorKind, VerificationResult},
};
use std::sync::Arc;
use viper::{ConsistencyError, VerificationError, VerificationResultKind};

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
        if !config.verify_mode.is_verify() {
            return Err("Ferrum currently supports verify mode only".into());
        }
        if prusti_utils::config::dump_viper_program() || prusti_utils::config::dump_debug_info() {
            return Err("Ferrum reporting or debug output mode is not yet implemented".into());
        }
        let mut args = config.verifier_args;
        args.extend(["--z3Exe".into(), prusti_utils::config::smt_solver_path()]);
        let report = allow_reporting.then(|| {
            Arc::new(crate::native_reporter::NativeReporter {
                procedures: procedures.into_iter().collect(),
                sender,
                errors: std::sync::Mutex::new(Default::default()),
            })
        });
        let reporter: Arc<dyn prusti_ferrum::silver::reporter::reporter::Reporter> =
            if let Some(report) = &report {
                Arc::new(
                    prusti_ferrum::silver::reporter::reporter::PluginAwareReporter {
                        name: "prusti".into(),
                        target: report.clone(),
                        mapper: std::sync::RwLock::new(None),
                    },
                )
            } else {
                Arc::new(NoopReporter)
            };
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
