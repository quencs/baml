use std::time::Instant;

use lsp_types::notification::DidChangeTextDocument;
use lsp_types::{DidChangeTextDocumentParams, PublishDiagnosticsParams};

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
        if !url.to_string().contains("baml_src") {
            return Ok(());
        }

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
        let document_key = {
            let project_guard = project.lock().unwrap();
            DocumentKey::from_url(&project_guard.root_path(), &url).internal_error()?
        };

        // Try to update the text document
        let update_result = session.update_text_document(
            &document_key,
            params.content_changes.clone(),
            params.text_document.version,
            Some(notifier.clone()),
        );

        // If update failed, it might be due to moved files - try to reload and retry
        if let Err(e) = update_result {
            tracing::warn!("Initial text document update failed: {}. Attempting reload...", e);
            
            // Try to reload session to handle moved files
            if let Err(reload_err) = session.reload(Some(notifier.clone())) {
                tracing::error!("Failed to reload session during did_change: {}", reload_err);
                return Err(reload_err).internal_error();
            }
            
            // Get fresh project and document key after reload
            let reloaded_project = session.get_or_create_project(&path);
            if let Some(reloaded_project) = reloaded_project {
                let new_document_key = {
                    let project_guard = reloaded_project.lock().unwrap();
                    DocumentKey::from_url(&project_guard.root_path(), &url).internal_error()?
                };
                
                // Try the update again with new document key
                session
                    .update_text_document(
                        &new_document_key,
                        params.content_changes,
                        params.text_document.version,
                        Some(notifier.clone()),
                    )
                    .internal_error()?;
                    
                tracing::info!("Successfully updated text document after reload");
            } else {
                return Err(anyhow::anyhow!("Could not find or create project after reload")).internal_error();
            }
        }

        tracing::info!("publishing diagnostics");

        publish_diagnostics(&notifier, project, Some(params.text_document.version))?;

        let elapsed = start_time_total.elapsed();
        tracing::info!("didchange total took {:?}ms", elapsed.as_millis());
        Ok(())
    }
}
