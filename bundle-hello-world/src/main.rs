// src/main.rs
use futures_util::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::Serialize;
use std::{convert::Infallible, ops::Range};
use tokio::sync::broadcast;
use warp::{
    Filter,
    http::Response,
    ws::{Message, WebSocket},
};

/// Embed at compile time everything in dist/
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/dist");

#[derive(Serialize)]
struct Hello {
    message: &'static str,
}

/// Handle a new WebSocket client: forward every broadcasted String as a text frame.
async fn client_connection(ws: WebSocket, mut rx: broadcast::Receiver<String>) {
    let (mut tx, mut _rx) = ws.split();
    // Spawn a task that listens on the broadcast channel...
    tokio::spawn(async move {
        while let Ok(json_msg) = rx.recv().await {
            if tx.send(Message::text(json_msg)).await.is_err() {
                // client disconnected
                break;
            }
        }
    });
    // (Optionally) you can also read from `_rx` if clients send you messages.
}

#[tokio::main]
async fn main() {
    // 1) Create a broadcast channel for JSON‐payloads
    let (tx, _rx) = broadcast::channel::<String>(16);

    // 2) Simple API route
    let api = warp::path!("api" / "hello").map(|| {
        warp::reply::json(&Hello {
            message: "Hello from Rust!",
        })
    });

    // 3) Static‐file SPA handler
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

    // 4) WebSocket route at /ws
    let tx_ws = tx.clone();
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let rx = tx_ws.subscribe();
            ws.on_upgrade(move |socket| client_connection(socket, rx))
        });

    // 5) Combine all routes
    let routes = ws_route.or(api).or(spa).with(warp::log("bundle-server"));

    // 6) Example: after 5 seconds, broadcast a demo message
    let demo_tx = tx.clone();
    tokio::spawn(async move {
        for _i in 0..100 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let payload = serde_json::json!({
                "command": "add_project",
                "content": {
                    "root_path": "/foo/bar",
                    "files": { "main.baml": "class Receipt {}" }
                }
            });
            let _ = demo_tx.send(payload.to_string());
        }
    });

    // 7) Launch the server
    println!("Listening on http://localhost:3030 …");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
