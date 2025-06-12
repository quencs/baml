use std::time::Instant;

use lsp_types::notification::DidChangeTextDocument;
use lsp_types::{DidChangeTextDocumentParams, PublishDiagnosticsParams};
use std::collections::HashMap;

use crate::playground::broadcast_project_update;
use crate::server::api::diagnostics::publish_diagnostics;
use crate::server::api::traits::{NotificationHandler, SyncNotificationHandler};
use crate::server::api::ResultExt;
use crate::server::client::{Notifier, Requester};
use crate::server::Result;
use crate::session::Session;
use crate::DocumentKey;

pub(crate) struct DidChangeTextDocumentHandler;

impl NotificationHandler for DidChangeTextDocumentHandler {
    type NotificationType = DidChangeTextDocument;
}

impl SyncNotificationHandler for DidChangeTextDocumentHandler {
    fn run(
        session: &mut Session,
        notifier: Notifier,
        _requester: &mut Requester,
        params: DidChangeTextDocumentParams,
    ) -> Result<()> {
        tracing::info!("DidChangeTextDocumentHandler");
        let start_time_total = Instant::now();

        let url = params.text_document.uri;
        let path = url
            .to_file_path()
            .internal_error_msg("Could not convert URL to path")?;

        // Get or create the project using the unified method
        let project = session.get_or_create_project(&path);
        if project.is_none() {
            tracing::error!("Failed to get or create project for path: {:?}", path);
            show_err_msg!("Failed to get or create project for path: {:?}", path);
        }

        let project = project.unwrap();
        let document_key =
            DocumentKey::from_url(&project.lock().unwrap().root_path(), &url).internal_error()?;

        session
            .update_text_document(
                &document_key,
                params.content_changes,
                params.text_document.version,
                Some(notifier.clone()),
            )
            .internal_error()?;

        // Broadcast the update using the playground runtime
        if let Some(runtime) = &session.playground_runtime {
            let state = session.playground_state.clone();
            tracing::info!("Runtime init!!");
            tracing::info!("state: {:?}", state);
            if let Some(state) = state {
                tracing::info!("Broadcasting project update to play-ground!!!");
                let projects = session.baml_src_projects.lock().unwrap();
                for (root_path, project) in projects.iter() {
                    let project = project.lock().unwrap();
                    let files = project.baml_project.files.clone();
                    let root_path = root_path.to_string_lossy().to_string();
                    let files_map: HashMap<String, String> = files
                        .into_iter()
                        .map(|(path, doc)| {
                            (path.path().to_string_lossy().to_string(), doc.contents)
                        })
                        .collect();

                    tracing::info!("files_map: {:?}", files_map);

                    let state = state.clone();
                    runtime.spawn(async move {
                        if let Err(e) =
                            broadcast_project_update(&state, &root_path, files_map).await
                        {
                            tracing::error!("Failed to broadcast project update: {}", e);
                        }
                    });
                }
            }
        }

        tracing::info!("publishing diagnostics");

        publish_diagnostics(&notifier, project, Some(params.text_document.version))?;

        let elapsed = start_time_total.elapsed();
        tracing::info!("didchange total took {:?}ms", elapsed.as_millis());
        Ok(())
    }
}
