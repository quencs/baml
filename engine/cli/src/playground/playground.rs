/// Script to run the playground server based of a specified directory.
/// Currently uses a custom filewatcher which detects changes to files in
/// the directory and refreshes the web-view.
use crate::playground::{create_routes, initialize_baml_files, setup_file_watcher, BamlState};
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Args, Debug, Clone)]
pub struct PlaygroundArgs {
    #[arg(long, help = "path/to/baml_src", default_value = "./baml_src")]
    pub from: PathBuf,
    #[arg(long, help = "port to expose playground on", default_value = "3030")]
    port: u16,
}

impl PlaygroundArgs {
    pub fn run(&self) -> Result<()> {
        let state = Arc::new(RwLock::new(BamlState::new()));

        // Initialize all BAML files from baml_src directory
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(initialize_baml_files(state.clone(), &self.from));

        // Set up file watcher
        setup_file_watcher(state.clone(), &self.from)?;

        // Sets up the connections for the frontend server
        let routes = create_routes(state);

        println!("Hosted playground at http://localhost:{}...", self.port);
        rt.block_on(warp::serve(routes).run(([127, 0, 0, 1], self.port)));

        Ok(())
    }
}
