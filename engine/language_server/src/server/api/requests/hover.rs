use std::collections::HashMap;

use lsp_types::{self as types, request as req, HoverParams, TextDocumentItem};

use crate::{
    server::{
        api::{
            traits::{RequestHandler, SyncRequestHandler},
            ResultExt,
        },
        client::{Notifier, Requester},
        Result,
    },
    DocumentKey, Session,
};

pub(crate) struct Hover;

impl RequestHandler for Hover {
    type RequestType = req::HoverRequest;
}

impl SyncRequestHandler for Hover {
    fn run(
        session: &mut Session,
        notifier: Notifier,
        _requester: &mut Requester,
        params: HoverParams,
    ) -> Result<Option<types::Hover>> {
        let url = &params.text_document_position_params.text_document.uri;
        let path = url
            .to_file_path()
            .internal_error_msg("Could not convert URL to path")?;
        let position = params.text_document_position_params.position;

        // Check runtime feature flag for new LSP implementation
        if session.baml_settings.use_new_lsp() {
            return run_new_lsp(session, &path, url, &position);
        }

        // Existing implementation (default)
        run_old_lsp(session, notifier, &path, url, &position)
    }
}

/// New Salsa-based hover implementation (when --features beta is enabled)
fn run_new_lsp(
    session: &Session,
    path: &std::path::Path,
    url: &lsp_types::Url,
    position: &lsp_types::Position,
) -> Result<Option<types::Hover>> {
    let Ok(lsp_db) = session.get_or_create_lsp_database(path) else {
        return Ok(None);
    };

    let mut db = lsp_db.lock();

    // Try to get file content from the old project system (for now)
    // This ensures we have the file content even if it hasn't been added to lsp_db yet
    if db.get_file(path).is_none() {
        if let Ok(project) = session.get_or_create_project(path) {
            let project_guard = project.lock();
            let root = project_guard.root_path();
            if let Ok(doc_key) = DocumentKey::from_url(root, url) {
                if let Some(text_doc) = project_guard.baml_project.files.get(&doc_key) {
                    db.add_or_update_file(path, &text_doc.contents);
                }
            }
        }
    }

    // Get the SourceFile for this path
    let Some(source_file) = db.get_file(path) else {
        tracing::warn!("*** HOVER (new): File not found in LspDatabase: {:?}", path);
        return Ok(None);
    };

    // Find symbol at position
    let Some(symbol) = db.symbol_at_position(source_file, position) else {
        return Ok(None);
    };

    // Generate hover text
    let hover_text = db.get_hover_text(&symbol);

    Ok(Some(types::Hover {
        contents: types::HoverContents::Markup(types::MarkupContent {
            kind: types::MarkupKind::Markdown,
            value: format!("```baml\n{}\n```", hover_text),
        }),
        range: None,
    }))
}

/// Existing Pest-based hover implementation (default)
fn run_old_lsp(
    session: &Session,
    notifier: Notifier,
    path: &std::path::Path,
    url: &lsp_types::Url,
    position: &lsp_types::Position,
) -> Result<Option<types::Hover>> {
    let Ok(project) = session.get_or_create_project(path) else {
        return Ok(None);
    };

    let document_key =
        DocumentKey::from_url(project.lock().root_path(), url).internal_error()?;

    let text_document_item = match project.lock().baml_project.files.get(&document_key) {
        None => {
            tracing::warn!("*** HOVER: Failed to find doc {:?}", url);
            Err(anyhow::anyhow!(
                "File {} was not present in the project",
                url
            ))
        }
        Some(text_document) => Ok(TextDocumentItem {
            uri: url.clone(),
            language_id: "BAML".to_string(),
            text: text_document.contents.clone(),
            version: 1,
        }),
    }
    .internal_error()?;

    // Just swallow the error here, we dont want hover failures to show error notifs for a user.
    let default_flags = vec!["beta".to_string()];
    let hover = match project.lock().handle_hover_request(
        &text_document_item,
        position,
        notifier,
        session
            .baml_settings
            .feature_flags
            .as_ref()
            .unwrap_or(&default_flags),
    ) {
        Ok(hover) => hover,
        Err(e) => {
            tracing::error!("Error handling hover request: {}", e);
            None
        }
    };

    Ok(hover)
}
