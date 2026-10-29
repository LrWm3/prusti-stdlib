//! A module that invokes the verifier `prusti-viper`

use log::{debug, warn};
use prusti_interface::{data::VerificationTask, environment::Environment, specs::typed};
use prusti_utils::{config, report::user};

#[tracing::instrument(name = "prusti::verify", level = "debug", skip(env))]
pub fn verify<'tcx>(
    env: Environment<'tcx>,
    def_spec: typed::DefSpecificationMap,
    verification_task: VerificationTask<'tcx>,
) {
    if env.diagnostic.has_errors() {
        warn!("The compiler reported an error, so the program will not be verified.");
    } else {
        debug!("Verification task: {:?}", verification_task);
        user::message(format!(
            "{}erification of {} items...",
            if verification_task.selective {
                "Selective v"
            } else {
                "V"
            },
            verification_task.procedures.len()
        ));

        if config::print_collected_verification_items() {
            println!(
                "Collected verification items {}:",
                verification_task.procedures.len()
            );
            for procedure in &verification_task.procedures {
                println!(
                    "procedure: {} at {:?}",
                    env.name.get_item_def_path(*procedure),
                    env.query.get_def_span(procedure)
                );
            }
        }

        // encode the crate to a RequestWithContext
        // TODO: push RequestWithContext through (replace VerificationRequest
        //   which is constructed further inside `prusti_server`)
        let phase_started =
            std::env::var_os("FERRUM_BENCH_PHASES").map(|_| std::time::Instant::now());
        let request = prusti_encoder::test_entrypoint(
            env.tcx(),
            env.body,
            def_spec,
            if verification_task.selective {
                Some(verification_task.procedures)
            } else {
                None
            },
            &env.diagnostic,
        );
        if let Some(started) = phase_started {
            log::info!(
                "[benchmark-phase] Rust to VIR encoding {} ns",
                started.elapsed().as_nanos()
            );
        }

        // if we do not use encode-only to get the dumped viper program (e.g., just to see if encoding fails), we don't need to start a JVM
        if !config::verify_mode().is_encode_only() || config::dump_viper_program() {
            let program = request.program;

            prusti_server::verify_programs(&env.diagnostic, vec![program]);
        }
    }
}
