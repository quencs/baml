/// Script to run the playground server based of a specified directory.
/// Currently uses a custom filewatcher which detects changes to files in
/// the directory and refreshes the web-view.
use crate::playground::definitions::PlaygroundState;
use crate::playground::playground_server::create_routes;
use crate::session::Session;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PlaygroundServer {
    state: Arc<RwLock<PlaygroundState>>,
    session: Arc<Session>,
}

impl PlaygroundServer {
    pub fn new(state: Arc<RwLock<PlaygroundState>>, session: Arc<Session>) -> Self {
        Self { state, session }
    }

    pub async fn run(self, port: u16) -> Result<()> {
        let routes = create_routes(self.state, self.session);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
        Ok(())
    }
}
