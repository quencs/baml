use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

// Note: the name add_project should match exactly to the
// EventListener.tsx command definitions due to how serde serializes these into json
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "content")]
pub enum FrontendMessage {
    add_project {
        root_path: String,
        files: HashMap<String, String>,
    },
    remove_project {
        root_path: String,
    },
    select_function {
        root_path: String,
        function_name: String,
    },
    baml_settings_updated {
        settings: HashMap<String, String>,
    },
    run_test {
        test_name: String,
    },
}

#[derive(Debug)]
pub struct PlaygroundState {
    pub tx: broadcast::Sender<String>,
    // Keep a reference to the receiver to prevent the channel from being closed
    _rx: broadcast::Receiver<String>,
    /// Key = root_path, value = last selected function for that project
    last_function: tokio::sync::RwLock<HashMap<String, String>>,
}

impl PlaygroundState {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(100);
        Self {
            tx,
            _rx: rx,
            last_function: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn broadcast_update(&self, msg: String) -> anyhow::Result<()> {
        let n = self.tx.send(msg)?;
        tracing::debug!("broadcast sent to {n} receivers");
        Ok(())
    }

    pub async fn set_last_function(&self, root: String, func: String) {
        self.last_function.write().await.insert(root, func);
    }

    pub async fn get_last_function(&self, root: &str) -> Option<String> {
        self.last_function.read().await.get(root).cloned()
    }

    pub async fn get_all_root_paths_with_functions(&self) -> Vec<String> {
        self.last_function.read().await.keys().cloned().collect()
    }
}
