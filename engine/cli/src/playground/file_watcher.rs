use notify_debouncer_full::{new_debouncer, notify::*};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

// code based on engine/baml-runtime/src/cli/dev.rs

pub struct FileWatcher {
    path: String,
}

impl FileWatcher {
    pub fn new(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            path: path.to_string(),
        })
    }

    pub fn watch<F>(&self, mut callback: F) -> std::io::Result<()>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let path = self.path.clone();
        std::thread::spawn(move || {
            let (tx, rx) = channel();
            // no specific tickrate, max debounce time 200ms
            let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx).unwrap();

            debouncer
                .watch(Path::new(&path), RecursiveMode::Recursive)
                .unwrap();

            for result in rx {
                match result {
                    Ok(events) => {
                        for event in events {
                            if let Some(paths) = event.paths.first() {
                                if let Some(path_str) = paths.to_str() {
                                    callback(path_str);
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        eprintln!("Error watching {}: {:?}", path, errors);
                    }
                }
            }
        });

        Ok(())
    }
}
