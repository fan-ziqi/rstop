//! Basic processes are sampled from startup; optional page sources stay active once opened.
use rstop::app::View;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    thread,
    time::{Duration, Instant},
};
const PROCESSES: u8 = 1;
const GPU_PROCESSES: u8 = 2;
const CONNECTIONS: u8 = 4;
const CONTAINERS: u8 = 8;
const SERVICES: u8 = 16;
const EXTRAS: u8 = 32;
const ALL: u8 = 63;
pub(super) struct Sources(AtomicU8);
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Needs {
    pub processes: bool,
    pub gpu_processes: bool,
    pub connections: bool,
    pub containers: bool,
    pub services: bool,
    pub extras: bool,
}

pub(super) fn wait_for_sample(until: Instant, sources: &Sources, needed: Needs) {
    // Discovery (including macOS GPU polling) must not shorten the history
    // sample period. The next scheduled sample picks up the latest inventory.
    while Instant::now() < until
        && sources.needs() == needed
        && !rstop::STOP.load(Ordering::Relaxed)
    {
        thread::park_timeout(Duration::from_millis(20));
    }
}
impl Sources {
    pub fn new(full: bool) -> Self {
        Self(AtomicU8::new(if full { ALL } else { PROCESSES }))
    }
    pub fn activate(&self, view: View, process_table: bool) -> bool {
        let mut bits = match view {
            View::Processes | View::Gpu => PROCESSES | GPU_PROCESSES,
            View::Containers => CONTAINERS | PROCESSES | GPU_PROCESSES,
            View::Services => SERVICES | PROCESSES | GPU_PROCESSES,
            View::Network => CONNECTIONS | PROCESSES | EXTRAS,
            View::Storage | View::System | View::Explorer => EXTRAS,
            _ => 0,
        };
        if process_table {
            bits |= PROCESSES | GPU_PROCESSES;
        }
        self.0.fetch_or(bits, Ordering::Relaxed) & bits != bits
    }
    pub fn needs(&self) -> Needs {
        let bits = self.0.load(Ordering::Relaxed);
        Needs {
            processes: bits & PROCESSES != 0,
            gpu_processes: bits & GPU_PROCESSES != 0,
            connections: bits & CONNECTIONS != 0,
            containers: bits & CONTAINERS != 0,
            services: bits & SERVICES != 0,
            extras: bits & EXTRAS != 0,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_updates_do_not_advance_the_sample_deadline() {
        let sources = Sources::new(false);
        let deadline = Instant::now() + Duration::from_millis(60);
        thread::current().unpark();
        wait_for_sample(deadline, &sources, sources.needs());
        assert!(
            Instant::now() >= deadline,
            "Background GPU updates must not create extra history samples"
        );
    }

    #[test]
    fn basic_processes_start_immediately_and_optional_sources_stay_active_once_opened() {
        let sources = Sources::new(false);
        assert!(sources.needs().processes);
        sources.activate(View::Overview, false);
        assert!(!sources.needs().containers);
        assert!(sources.needs().processes);
        assert!(!sources.needs().gpu_processes);
        assert!(!sources.needs().extras);
        assert!(sources.activate(View::Processes, true));
        assert!(sources.needs().gpu_processes);
        sources.activate(View::Containers, false);
        sources.activate(View::Overview, false);
        assert!(sources.needs().containers);
        assert!(sources.needs().processes);
        assert!(!sources.needs().services);
        assert!(!sources.needs().connections);
        sources.activate(View::Services, false);
        sources.activate(View::Network, false);
        assert!(
            sources.needs().containers && sources.needs().services && sources.needs().connections
        );
    }
}
