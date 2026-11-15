//! Opt-in backend phase observations for paired performance experiments.
use std::time::Instant;

pub(crate) struct Phase {
    name: &'static str,
    started: Option<Instant>,
}

impl Phase {
    pub(crate) fn start(name: &'static str) -> Self {
        Self {
            name,
            started: std::env::var_os("FERRUM_BENCH_PHASES").map(|_| Instant::now()),
        }
    }
}

impl Drop for Phase {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            log::info!(
                "[benchmark-phase] {} {} ns",
                self.name,
                started.elapsed().as_nanos()
            );
        }
    }
}
