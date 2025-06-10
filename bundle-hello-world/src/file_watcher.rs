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
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, _>| {
                    if let Ok(event) = res {
                        tx.send(event).unwrap();
                    }
                },
                Config::default()
                    .with_poll_interval(Duration::from_secs(1))
                    .with_compare_contents(true),
            )
            .unwrap();

            watcher
                .watch(Path::new(&path), RecursiveMode::NonRecursive)
                .unwrap();

            for event in rx {
                // Only trigger on actual content changes, not metadata changes
                if let EventKind::Modify(kind) = event.kind {
                    if kind == notify::event::ModifyKind::Data(DataChange::Content) {
                        callback(&path);
                    }
                }
            }
        });

        Ok(())
    }
}
