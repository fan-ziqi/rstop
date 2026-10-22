//! Each slow source publishes independently; an optional probe cannot hold up another page.
use super::sampling::{Needs, Sources};
use rstop::{STOP, config::Config, discovery, model::Inventory};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    Containers,
    Services,
    Extras,
    #[cfg(target_os = "macos")]
    Gpu,
}

impl Source {
    fn needed(self, needs: Needs) -> bool {
        match self {
            Self::Containers => needs.containers,
            Self::Services => needs.services,
            Self::Extras => needs.extras,
            #[cfg(target_os = "macos")]
            Self::Gpu => true,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Containers => "containers",
            Self::Services => "services",
            Self::Extras => "extras",
            #[cfg(target_os = "macos")]
            Self::Gpu => "platform_gpu",
        }
    }
}

#[derive(Default)]
pub(super) struct Cache {
    samples: BTreeMap<Source, Inventory>,
    pub revision: Arc<AtomicU64>,
}

impl Cache {
    fn publish(&mut self, source: Source, data: Inventory) {
        self.samples.insert(source, data);
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Inventory {
        let mut data = Inventory::default();
        for sample in self.samples.values() {
            data.updated_at = data.updated_at.max(sample.updated_at);
            data.containers.extend(sample.containers.iter().cloned());
            data.runtimes.extend(sample.runtimes.iter().cloned());
            data.services.extend(sample.services.iter().cloned());
            data.service_managers
                .extend(sample.service_managers.iter().cloned());
            data.listeners.extend(sample.listeners.iter().cloned());
            data.notes.extend(sample.notes.iter().cloned());
            data.extra.extend(sample.extra.clone());
        }
        data.extra.insert(
            "Discovery state".into(),
            serde_json::json!({
                "containers":self.samples.contains_key(&Source::Containers),
                "services":self.samples.contains_key(&Source::Services),
                "extras":self.samples.contains_key(&Source::Extras),
                "platform_gpu": !cfg!(target_os = "macos") || self.samples.keys().any(|s| s.name() == "platform_gpu"),
            }),
        );
        data
    }
}

pub(super) fn workers(
    cache: Arc<RwLock<Cache>>,
    preferences: Arc<RwLock<Config>>,
    sources: Arc<Sources>,
    live: bool,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    if !live {
        return Ok(Vec::new());
    }
    [
        Source::Containers,
        Source::Services,
        Source::Extras,
        #[cfg(target_os = "macos")]
        Source::Gpu,
    ]
    .into_iter()
    .map(|source| {
        let cache = cache.clone();
        let preferences = preferences.clone();
        let sources = sources.clone();
        thread::Builder::new()
            .name(format!("rstop-{}", source.name()))
            .spawn(move || {
                let mut rates = discovery::InventoryRates::default();
                while !STOP.load(Ordering::Relaxed) {
                    if !source.needed(sources.needs()) {
                        thread::park_timeout(Duration::from_secs(1));
                        continue;
                    }
                    let settings = preferences.read().map(|c| c.clone()).unwrap_or_default();
                    let first = cache.read().is_ok_and(|c| !c.samples.contains_key(&source));
                    let publish = |data: Inventory| {
                        if let Ok(mut current) = cache.write() {
                            current.publish(source, data);
                        }
                    };
                    let mut data = match source {
                        #[cfg(target_os = "macos")]
                        Source::Gpu => Inventory {
                            updated_at: rstop::collector::unix_time(),
                            extra: rstop::platform_gpu::discover(&settings),
                            ..Default::default()
                        },
                        _ => discovery::discover_background(
                            &settings,
                            source == Source::Containers,
                            source == Source::Services,
                            source == Source::Extras,
                            |partial| {
                                if first {
                                    publish(partial.clone());
                                }
                            },
                        ),
                    };
                    rates.apply(&mut data);
                    publish(data);
                    let delay = if source.name() == "platform_gpu" {
                        Duration::from_millis(settings.interval_ms)
                    } else {
                        Duration::from_secs(5)
                    };
                    let until = Instant::now() + delay;
                    while Instant::now() < until && !STOP.load(Ordering::Relaxed) {
                        thread::park_timeout(Duration::from_millis(50));
                    }
                }
            })
            .map_err(Into::into)
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_publications_keep_other_sources_and_their_loading_state() {
        let mut cache = Cache::default();
        cache.publish(
            Source::Services,
            Inventory {
                updated_at: 1,
                service_managers: vec!["launchd/user".into()],
                notes: vec!["service note".into()],
                ..Default::default()
            },
        );
        let data = cache.snapshot();
        assert_eq!(data.extra["Discovery state"]["services"], true);
        assert_eq!(data.extra["Discovery state"]["containers"], false);
        cache.publish(
            Source::Containers,
            Inventory {
                updated_at: 2,
                runtimes: vec!["docker".into()],
                ..Default::default()
            },
        );
        let data = cache.snapshot();
        assert_eq!(data.updated_at, 2);
        assert_eq!(data.runtimes, ["docker"]);
        assert_eq!(data.service_managers, ["launchd/user"]);
        assert_eq!(data.notes, ["service note"]);
        cache.publish(Source::Services, Inventory::default());
        let data = cache.snapshot();
        assert!(data.service_managers.is_empty());
        assert!(data.notes.is_empty());
        assert_eq!(data.runtimes, ["docker"]);
        assert_eq!(cache.revision.load(Ordering::Relaxed), 3);
    }
}
