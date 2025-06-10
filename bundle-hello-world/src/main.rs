use futures_util::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
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

async fn client_connection(ws: warp::ws::WebSocket, state: Arc<RwLock<BamlState>>) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Load BAML files and send initial state
    {
        let mut state = state.write().await;
        if let Err(e) = state.load_baml_files() {
            eprintln!("Error loading BAML files: {}", e);
        }
    }

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
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(BamlState::new()));

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
