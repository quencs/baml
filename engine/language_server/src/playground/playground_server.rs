use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

/// Script that runs the playground server.
/// On the input port
use crate::playground::definitions::PlaygroundState;
use crate::{
    playground::{playground_server_helpers::create_server_routes, proxy::ProxyServer},
    session::Session,
};

#[derive(Debug, Clone)]
pub struct PlaygroundServer {
    state: Arc<RwLock<PlaygroundState>>,
    session: Arc<Session>,
}

impl PlaygroundServer {
    pub fn new(state: Arc<RwLock<PlaygroundState>>, session: Arc<Session>) -> Self {
        Self { state, session }
    }

    pub async fn run(self, port: u16) -> Result<()> {
        let routes = create_server_routes(self.state, self.session);

        // Start the proxy server on a different port
        let proxy_port = port + 1; // Use playground port + 1 for proxy
        let proxy_server = ProxyServer::new(proxy_port);

        // Spawn the proxy server in a separate task
        let proxy_handle = tokio::spawn(async move {
            if let Err(e) = proxy_server.start().await {
                tracing::error!("Proxy server failed: {}", e);
            }
        });

        // Start the main playground server
        tracing::info!("Starting main playground server on port {}", port);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;

        // If we get here, the main server has stopped
        tracing::info!("Main playground server stopped");

        Ok(())
    }
}
