//! Native Ferrum backend, invoked on the verification owner thread.
use crate::ViperBackendConfig;
use prusti_ferrum::silver::{
    ast::program::Program,
    reporter::reporter::NoopReporter,
    verifier::verification_result::{AbstractError, FrontendErrorKind, VerificationResult},
};
use std::sync::Arc;
use viper::{ConsistencyError, VerificationError, VerificationResultKind};

pub(crate) fn verify(program: Program, config: ViperBackendConfig) -> VerificationResultKind {
    fn run(program: Program, config: ViperBackendConfig) -> Result<VerificationResultKind, String> {
        if !config.verify_mode.is_verify() {
            return Err("Ferrum currently supports verify mode only".into());
        }
        if prusti_utils::config::counterexample() {
            return Err("Ferrum counterexample backtranslation is not yet implemented".into());
        }
        if prusti_utils::config::report_viper_messages()
            || prusti_utils::config::dump_viper_program()
            || prusti_utils::config::dump_debug_info()
        {
            return Err("Ferrum reporting or debug output mode is not yet implemented".into());
        }
        let mut args = config.verifier_args;
        args.extend(["--z3Exe".into(), prusti_utils::config::smt_solver_path()]);
        let warnings = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut frontend = match prusti_ferrum::native_frontend_with_warning_sink(
            &args,
            Arc::new(NoopReporter),
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
                let mut consistency = Vec::new();
                for error in errors {
                    match error {
                        AbstractError::Verification(error) => {
                            let error = prusti_ferrum::verification_error(error)
                                .map_err(|e| format!("diagnostic mapping: {e:?}"))?;
                            verification.push(VerificationError::new(
                                error.full_id,
                                error.pos_id,
                                error.offending_pos_id,
                                error.reason_pos_id,
                                error.message,
                                None,
                            ));
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
                if !verification.is_empty() && !consistency.is_empty() {
                    return Err("mixed frontend and verification errors".into());
                }
                if !verification.is_empty() {
                    Ok(VerificationResultKind::Failure(verification))
                } else {
                    Ok(VerificationResultKind::ConsistencyErrors(consistency))
                }
            }
        }
    }
    run(program, config).unwrap_or_else(VerificationResultKind::BackendError)
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
                let good = verify(make(true), config.clone());
                assert!(matches!(good, VerificationResultKind::Success), "{good:?}");
                let result = verify(make(false), config);
                assert!(
                    matches!(result, VerificationResultKind::Failure(_)),
                    "{result:?}"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
