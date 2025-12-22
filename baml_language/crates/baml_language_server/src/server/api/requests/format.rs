use lsp_types::{DocumentFormattingParams, TextEdit, request};

use crate::{
    Session,
    baml_project::position_utils::full_document_range,
    server::{
        Result,
        api::{
            ResultExt,
            traits::{RequestHandler, SyncRequestHandler},
        },
        client::{Notifier, Requester},
    },
};

pub(crate) struct DocumentFormatting;

impl RequestHandler for DocumentFormatting {
    type RequestType = request::Formatting;
}

impl SyncRequestHandler for DocumentFormatting {
    fn run(
        session: &mut Session,
        _notifier: Notifier,
        _requester: &mut Requester,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<lsp_types::TextEdit>>> {
        let url = &params.text_document.uri;
        let path = url
            .to_file_path()
            .internal_error_msg("Could not convert URL to path")?;

        // Get the project
        let Ok(project) = session.get_or_create_project(&path) else {
            tracing::warn!(
                "DocumentFormatting: Could not get project for path: {:?}",
                path
            );
            return Ok(None);
        };

        let guard = project.lock();
        let lsp_db = guard.lsp_db();

        // Get the SourceFile for this path
        let Some(source_file) = lsp_db.get_file(&path) else {
            tracing::warn!(
                "DocumentFormatting: file not found in LspDatabase: {:?}",
                path
            );
            return Ok(None);
        };

        // Get the original content for the range
        let original_content = source_file.text(lsp_db.db());

        // Format the file using baml_format
        let formatted_content = match baml_format::format_file(lsp_db.db(), source_file) {
            Some(formatted) => formatted,
            None => {
                tracing::warn!("DocumentFormatting: formatter returned None (likely parse errors)");
                return Ok(None);
            }
        };

        // Return a TextEdit that replaces the entire document
        Ok(Some(vec![TextEdit {
            range: full_document_range(&original_content),
            new_text: formatted_content,
        }]))
    }
}
