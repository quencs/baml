use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::{path::Path, sync::Arc};
use tokio::sync::broadcast;

pub struct FileWatcher {
    tx: broadcast::Sender<String>,
}

impl FileWatcher {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        Self { tx }
    }

    pub fn watch(&self, path: &Path) -> notify::Result<()> {
        let tx = self.tx.clone();
        let path = Arc::new(path.to_path_buf());

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, _>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        if !path.extension().map_or(false, |ext| ext == "baml") {
                            continue;
                        }

                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let parent =
                                        path.parent().and_then(|p| p.to_str()).unwrap_or(".");

                                    let filename = path
                                        .file_name()
                                        .and_then(|f| f.to_str())
                                        .unwrap_or("unknown.baml");

                                    let payload = json!({
                                        "command": "add_project",
                                        "content": {
                                            "root_path": parent,
                                            "files": {
                                                filename: content
                                            }
                                        }
                                    });

                                    let _ = tx.send(payload.to_string());
                                }
                            }
                            EventKind::Remove(_) => {
                                let parent = path.parent().and_then(|p| p.to_str()).unwrap_or(".");

                                let payload = json!({
                                    "command": "remove_project",
                                    "content": {
                                        "root_path": parent
                                    }
                                });

                                let _ = tx.send(payload.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            },
            Config::default().with_poll_interval(std::time::Duration::from_secs(1)),
        )?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
        Ok(())
    }
}
