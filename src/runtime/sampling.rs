//! Monotonic activation: opening a page starts its sources for the rest of the session.
use rstop::app::View;
use std::sync::atomic::{AtomicU8, Ordering};
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
impl Sources {
    pub fn new(full: bool) -> Self {
        Self(AtomicU8::new(if full { ALL } else { 0 }))
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
    fn sources_stay_active_after_leaving_a_page_and_unopened_pages_stay_dormant() {
        let sources = Sources::new(false);
        sources.activate(View::Overview, false);
        assert!(!sources.needs().containers);
        assert!(!sources.needs().processes);
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
