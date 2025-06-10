// src/main.rs
use futures_util::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::{convert::Infallible, ops::Range};
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use warp::{
    Filter,
    http::Response,
    ws::{Message, WebSocket},
};

/// Embed at compile time everything in dist/
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/dist");

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "content")]
enum FrontendMessage {
    AddProject {
        root_path: String,
        files: HashMap<String, String>,
    },
    ModifyFile {
        root_path: String,
        name: String,
        content: Option<String>,
    },
    RemoveProject {
        root_path: String,
    },
    SetFlashingRegions {
        spans: Vec<Span>,
    },
    SelectFunction {
        root_path: String,
        function_name: String,
    },
    UpdateCursor {
        cursor: CursorPosition,
    },
    BamlSettingsUpdated {
        // Add BAML settings fields as needed
        settings: HashMap<String, String>,
    },
    RunTest {
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
    current_function: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl BamlState {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
            current_function: None,
            diagnostics: Vec::new(),
        }
    }

    fn load_baml_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load receipt.baml
        let receipt_content = fs::read_to_string("receipt.baml")?;
        self.files
            .insert("receipt.baml".to_string(), receipt_content);

        // Load clients.baml
        let clients_content = fs::read_to_string("clients.baml")?;
        self.files
            .insert("clients.baml".to_string(), clients_content);

        Ok(())
    }
}

async fn handle_ws_message(
    msg: String,
    state: Arc<RwLock<BamlState>>,
    tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let message: FrontendMessage = serde_json::from_str(&msg)?;

    match message {
        FrontendMessage::AddProject { root_path, files } => {
            let mut state = state.write().await;
            state.files = files;

            // Process BAML files
            if let Some(baml_content) = state.files.get("receipt.baml") {
                // TODO: Parse BAML and validate
                // TODO: Update diagnostics
                // TODO: Send updates to frontend
            }
        }
        FrontendMessage::ModifyFile {
            root_path,
            name,
            content,
        } => {
            let mut state = state.write().await;
            if let Some(content) = content {
                state.files.insert(name, content);
                // TODO: Revalidate BAML
                // TODO: Update diagnostics
                // TODO: Send updates to frontend
            }
        }
        FrontendMessage::RemoveProject { root_path } => {
            let mut state = state.write().await;
            state.files.clear();
            state.current_function = None;
            state.diagnostics.clear();
        }
        FrontendMessage::SetFlashingRegions { spans } => {
            // TODO: Handle flashing regions
        }
        FrontendMessage::SelectFunction {
            root_path,
            function_name,
        } => {
            let mut state = state.write().await;
            state.current_function = Some(function_name);
        }
        FrontendMessage::UpdateCursor { cursor } => {
            // TODO: Handle cursor updates
        }
        FrontendMessage::BamlSettingsUpdated { settings } => {
            // TODO: Handle settings updates
        }
        FrontendMessage::RunTest { test_name } => {
            // TODO: Handle test execution
        }
    }

    Ok(())
}

async fn client_connection(
    ws: warp::ws::WebSocket,
    state: Arc<RwLock<BamlState>>,
    tx: broadcast::Sender<String>,
) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut rx = tx.subscribe();

    // Load BAML files and send initial state
    {
        let mut state = state.write().await;
        if let Err(e) = state.load_baml_files() {
            eprintln!("Error loading BAML files: {}", e);
        }
    }

    // Send initial port number
    let port_msg = serde_json::json!({
        "command": "port_number",
        "content": {
            "port": 3030
        }
    });
    let _ = ws_tx.send(Message::text(port_msg.to_string())).await;

    // Send initial project state
    let state_read = state.read().await;
    let add_project_msg = serde_json::json!({
        "command": "add_project",
        "content": {
            "root_path": ".",
            "files": state_read.files
        }
    });
    let _ = ws_tx.send(Message::text(add_project_msg.to_string())).await;

    // Handle incoming messages
    let state_clone = state.clone();
    let tx_clone = tx.clone();
    let handle_incoming = async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_str() {
                        if let Err(e) = handle_ws_message(
                            text.to_string(),
                            state_clone.clone(),
                            tx_clone.clone(),
                        )
                        .await
                        {
                            eprintln!("Error handling message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    // Handle outgoing messages
    let handle_outgoing = async move {
        while let Ok(msg) = rx.recv().await {
            if ws_tx.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = handle_incoming => {},
        _ = handle_outgoing => {},
    }
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<String>(16);
    let state = Arc::new(RwLock::new(BamlState::new()));

    // WebSocket handler
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let tx = tx.clone();
            let state = state.clone();
            ws.on_upgrade(move |socket| client_connection(socket, state, tx))
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
