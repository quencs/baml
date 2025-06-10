use notify::event::DataChange;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct FileWatcher {
    path: String,
}

impl FileWatcher {
    pub fn new(path: &str) -> Result<Self, notify::Error> {
        Ok(Self {
            path: path.to_string(),
        })
    }

    pub fn watch<F>(&self, mut callback: F) -> Result<(), notify::Error>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let path = self.path.clone();
        std::thread::spawn(move || {
            let (tx, rx) = channel();
            let mut watcher = match RecommendedWatcher::new(
                move |res: Result<Event, _>| {
                    if let Ok(event) = res {
                        tx.send(event).unwrap();
                    }
                },
                Config::default()
                    .with_poll_interval(Duration::from_secs(1))
                    .with_compare_contents(true),
            ) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Failed to create watcher for {}: {}", path, e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(Path::new(&path), RecursiveMode::Recursive) {
                eprintln!("Failed to watch {}: {}", path, e);
                return;
            }

            for event in rx {
                // Only trigger on actual content changes, not metadata changes
                if let EventKind::Modify(kind) = event.kind {
                    if kind == notify::event::ModifyKind::Data(DataChange::Content) {
                        // Get the full path of the changed file
                        if let Some(paths) = event.paths.first() {
                            if let Some(path_str) = paths.to_str() {
                                callback(path_str);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
