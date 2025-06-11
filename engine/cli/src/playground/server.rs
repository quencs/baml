use crate::playground::{BamlState, FileWatcher, FrontendMessage};
use anyhow::Result;
// use baml_log::bdebug;
use futures_util::{SinkExt, StreamExt};
use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{http::Response, ws::Message, Filter};

/// Embed at compile time everything in dist/
/// This embeds the entire frontend code into the binary which includes the web-view and playground-common
/// NOTE: requires web-panel for vscode to be built
static STATIC_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../typescript/vscode-ext/packages/web-panel/dist");
// Does not matter what this is, it is not currently used in the playground
static ROOT_PATH: &str = ".";

pub async fn client_connection(ws: warp::ws::WebSocket, state: Arc<RwLock<BamlState>>) {
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

pub async fn initialize_baml_files(state: Arc<RwLock<BamlState>>, baml_src: &Path) {
    let mut state = state.write().await;
    for path in get_baml_files(baml_src) {
        if let Ok(content) = fs::read_to_string(&path) {
            state.files.insert(path, content);
        }
    }
}

// method that retrieves all baml files in the baml_src directory
pub fn get_baml_files(baml_src: &Path) -> Vec<String> {
    let mut files = Vec::new();

    fn search_dir(dir_path: &std::path::Path, files: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    search_dir(&path, files);
                } else if path.is_file() && path.extension().map_or(false, |ext| ext == "baml") {
                    if let Ok(absolute) = fs::canonicalize(&path) {
                        if let Some(path_str) = absolute.to_str() {
                            files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }

    search_dir(baml_src, &mut files);
    files
}

pub fn setup_file_watcher(state: Arc<RwLock<BamlState>>, baml_src: &Path) -> Result<()> {
    if let Ok(watcher) = FileWatcher::new(baml_src.to_str().unwrap()) {
        let state_clone = state.clone();
        if let Err(e) = watcher.watch(move |path| {
            // bdebug!("BAML file changed: {}", path);
            // Reload the file and update state
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut state = state_clone.write().await;
                // Remove the modified file from state
                state.files.remove(path);
                // Re-add it if it still exists
                if let Ok(content) = fs::read_to_string(path) {
                    state.files.insert(path.to_string(), content);
                }
                // bdebug!("files: {:?}", state.files.clone());

                let add_project_msg = FrontendMessage::add_project {
                    root_path: ROOT_PATH.to_string(),
                    files: state.files.clone(),
                };
                let msg_str = serde_json::to_string(&add_project_msg).unwrap();
                let _ = state.tx.send(msg_str);
            });
        }) {
            eprintln!("Failed to watch baml_src directory: {}", e);
        }
    }
    Ok(())
}

pub fn create_routes(
    state: Arc<RwLock<BamlState>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // WebSocket handler
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let state = state.clone();
            ws.on_upgrade(move |socket| client_connection(socket, state))
        });

    // Static file serving needed to serve the frontend files
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
                        Ok::<_, warp::Rejection>(
                            Response::builder()
                                .header("content-type", mime.as_ref())
                                .body(body.to_vec()),
                        )
                    }
                    None => Ok::<_, warp::Rejection>(
                        Response::builder().status(404).body(b"Not Found".to_vec()),
                    ),
                }
            });

    ws_route.or(spa).with(warp::log("bundle-server"))
}
