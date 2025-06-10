use crate::file_watcher::FileWatcher;
use futures_util::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use warp::{Filter, http::Response, ws::Message};

mod file_watcher;

/// Embed at compile time everything in dist/
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/dist");
static ROOT_PATH: &str = ".";

#[derive(Serialize, Deserialize, Debug)]
struct Span {
    file_path: String,
    start_line: u32,
    start: u32,
    end_line: u32,
    end: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct CursorPosition {
    file_name: String,
    file_text: String,
    line: u32,
    column: u32,
}

// Note: the name add_project should match exactly to the
// EventListener.tsx command definitions due to how serde serializes these into json
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "content")]
enum FrontendMessage {
    add_project {
        root_path: String,
        files: HashMap<String, String>,
    },
    remove_project {
        root_path: String,
    },
    set_flashing_regions {
        spans: Vec<Span>,
    },
    select_function {
        root_path: String,
        function_name: String,
    },
    update_cursor {
        cursor: CursorPosition,
    },
    baml_settings_updated {
        settings: HashMap<String, String>,
    },
    run_test {
        test_name: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Diagnostic {
    file_path: String,
    start_line: u32,
    start: u32,
    end_line: u32,
    end: u32,
    message: String,
    severity: String,
}

struct BamlState {
    files: HashMap<String, String>,
    tx: broadcast::Sender<String>,
}

impl BamlState {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            files: HashMap::new(),
            tx,
        }
    }
}

async fn client_connection(ws: warp::ws::WebSocket, state: Arc<RwLock<BamlState>>) {
    let (mut ws_tx, _ws_rx) = ws.split();
    let mut rx = {
        let state = state.read().await;
        state.tx.subscribe()
    };

    // Send initial project state
    let state_read = state.read().await;
    let add_project_msg = FrontendMessage::add_project {
        root_path: ROOT_PATH.to_string(),
        files: state_read.files.clone(),
    };
    // Send the add project message to the client
    let _ = ws_tx
        .send(Message::text(
            serde_json::to_string(&add_project_msg).unwrap(),
        ))
        .await;

    // Forward broadcast messages to this client
    // Ensures realtime updates on the UI
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = ws_tx.send(Message::text(msg)).await;
        }
    });
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(BamlState::new()));

    // Set up a single file watcher for the baml_src directory
    if let Ok(watcher) = FileWatcher::new("baml_src") {
        let state_clone = state.clone();
        if let Err(e) = watcher.watch(move |path| {
            println!("BAML file changed: {}", path);
            // Reload the file and update state
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut state = state_clone.write().await;
                if let Ok(content) = fs::read_to_string(path) {
                    state.files.insert(path.to_string(), content);

                    let add_project_msg = FrontendMessage::add_project {
                        root_path: ROOT_PATH.to_string(),
                        files: state.files.clone(),
                    };
                    let msg_str = serde_json::to_string(&add_project_msg).unwrap();
                    let _ = state.tx.send(msg_str);
                }
            });
        }) {
            eprintln!("Failed to watch baml_src directory: {}", e);
        }
    }

    // WebSocket handler
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let state = state.clone();
            ws.on_upgrade(move |socket| client_connection(socket, state))
        });

    // Static file serving
    let spa =
        warp::path::full()
            .and(warp::get())
            .and_then(|full: warp::path::FullPath| async move {
                let path = full.as_str().trim_start_matches('/');
                let file = if path.is_empty() { "index.html" } else { path };
                match STATIC_DIR.get_file(file) {
                    Some(f) => {
                        let body = f.contents();
                        let mime = from_path(file).first_or_octet_stream();
                        Ok::<_, Infallible>(
                            Response::builder()
                                .header("content-type", mime.as_ref())
                                .body(body.to_vec()),
                        )
                    }
                    None => Ok(Response::builder().status(404).body(b"Not Found".to_vec())),
                }
            });

    let routes = ws_route.or(spa).with(warp::log("bundle-server"));

    println!("Listening on http://localhost:3030 …");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
