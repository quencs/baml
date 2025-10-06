use std::path::PathBuf;

use lsp_types::{
    request, CompletionItem, CompletionItemKind, CompletionList, CompletionParams,
    CompletionResponse,
};

use crate::{
    baml_project::{
        position_utils::{get_symbol_before_position, get_word_at_position},
        trim_line,
    },
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
pub(crate) struct Completion;

impl RequestHandler for Completion {
    type RequestType = request::Completion;
}

impl SyncRequestHandler for Completion {
    fn run(
        session: &mut Session,
        _notifier: Notifier,
        _requester: &mut Requester,
        params: CompletionParams,
    ) -> Result<Option<lsp_types::CompletionResponse>> {
        let url = params.text_document_position.text_document.uri;
        let path = url
            .to_file_path()
            .internal_error_msg("Could not convert URL to path")?;
        let Ok(project) = session.get_or_create_project(&path) else {
            return Ok(None);
        };

        let guard = project.lock();
        let document_key =
            DocumentKey::from_url(&PathBuf::from(guard.root_path()), &url).internal_error()?;
        let doc = guard
            .baml_project
            .files
            .get(&document_key)
            .ok_or(anyhow::anyhow!(
                "File {} was not present in the project",
                document_key
            ))
            .internal_error()?;

        let pos = params.text_document_position.position;
        let word = get_word_at_position(&doc.contents, &pos);
        let cleaned_word = trim_line(&word);

        // Simple context dispatch
        let mut items: Vec<CompletionItem> = Vec::new();

        // Attributes: detect using preceding characters
        let prev = get_symbol_before_position(&doc.contents, &pos);
        if prev == "@" {
            // Check if it's a block attribute (preceded by another @)
            let prev2 = if pos.character > 0 {
                let mut prev_pos = pos.clone();
                prev_pos.character = prev_pos.character.saturating_sub(1);
                get_symbol_before_position(&doc.contents, &prev_pos)
            } else {
                String::new()
            };

            if prev2 == "@" {
                items.extend(block_attribute_items(cleaned_word.as_str()));
            } else {
                items.extend(field_attribute_items(cleaned_word.as_str()));
            }
        }

        // Prompt helpers: _.role("...") and ctx.*
        if cleaned_word == "_." || cleaned_word.ends_with("_.") {
            items.extend(role_function_items());
        }
        if cleaned_word == "ctx." || cleaned_word.ends_with("ctx.") {
            items.extend(ctx_items());
        }
        if cleaned_word == "ctx.client." || cleaned_word.ends_with("ctx.client.") {
            items.extend(ctx_client_items());
        }

        // Top-level keywords (coarse heuristic: empty/short word)
        if items.is_empty() && cleaned_word.is_empty() {
            items.extend(top_level_keywords());
        }

        // IR-driven symbols as a fallback enhancement
        if items.is_empty() {
            if let Ok(rt) = guard.runtime() {
                items.extend(ir_symbol_items(rt));
            }
        }

        if items.is_empty() {
            return Ok(None);
        }

        let completion_list = CompletionList {
            is_incomplete: false,
            items,
            ..CompletionList::default()
        };
        Ok(Some(CompletionResponse::List(completion_list)))
    }
}

fn mk_item(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        ..CompletionItem::default()
    }
}

fn field_attribute_items(_prefix: &str) -> Vec<CompletionItem> {
    vec![
        mk_item("@alias", CompletionItemKind::KEYWORD, "attribute"),
        mk_item("@description", CompletionItemKind::KEYWORD, "attribute"),
        mk_item("@check", CompletionItemKind::KEYWORD, "attribute"),
        mk_item("@assert", CompletionItemKind::KEYWORD, "attribute"),
        mk_item("@stream.done", CompletionItemKind::KEYWORD, "attribute"),
        mk_item("@stream.not_null", CompletionItemKind::KEYWORD, "attribute"),
        mk_item(
            "@stream.with_state",
            CompletionItemKind::KEYWORD,
            "attribute",
        ),
    ]
}

fn block_attribute_items(_prefix: &str) -> Vec<CompletionItem> {
    vec![
        mk_item("@@dynamic", CompletionItemKind::KEYWORD, "block attribute"),
        mk_item("@@alias", CompletionItemKind::KEYWORD, "block attribute"),
        mk_item("@@assert", CompletionItemKind::KEYWORD, "block attribute"),
    ]
}

fn role_function_items() -> Vec<CompletionItem> {
    vec![
        mk_item(
            "_.role(\"system\")",
            CompletionItemKind::FUNCTION,
            "prompt helper",
        ),
        mk_item(
            "_.role(\"assistant\")",
            CompletionItemKind::FUNCTION,
            "prompt helper",
        ),
        mk_item(
            "_.role(\"user\")",
            CompletionItemKind::FUNCTION,
            "prompt helper",
        ),
    ]
}

fn ctx_items() -> Vec<CompletionItem> {
    vec![
        mk_item(
            "ctx.output_format",
            CompletionItemKind::PROPERTY,
            "prompt context",
        ),
        mk_item("ctx.client", CompletionItemKind::PROPERTY, "prompt context"),
    ]
}

fn ctx_client_items() -> Vec<CompletionItem> {
    vec![
        mk_item(
            "ctx.client.name",
            CompletionItemKind::PROPERTY,
            "prompt context",
        ),
        mk_item(
            "ctx.client.provider",
            CompletionItemKind::PROPERTY,
            "prompt context",
        ),
    ]
}

fn top_level_keywords() -> Vec<CompletionItem> {
    vec![
        mk_item("function", CompletionItemKind::KEYWORD, "declaration"),
        mk_item("class", CompletionItemKind::KEYWORD, "declaration"),
        mk_item("enum", CompletionItemKind::KEYWORD, "declaration"),
        mk_item("client", CompletionItemKind::KEYWORD, "declaration"),
        mk_item("generator", CompletionItemKind::KEYWORD, "declaration"),
        mk_item("retry_policy", CompletionItemKind::KEYWORD, "declaration"),
        mk_item(
            "template_string",
            CompletionItemKind::KEYWORD,
            "declaration",
        ),
        mk_item("type", CompletionItemKind::KEYWORD, "declaration"),
    ]
}

fn ir_symbol_items(rt: &baml_runtime::BamlRuntime) -> Vec<CompletionItem> {
    use crate::baml_project::BamlRuntimeExt;
    let mut items: Vec<CompletionItem> = Vec::new();

    // functions
    for f in rt.list_functions() {
        items.push(mk_item(
            &f.name,
            CompletionItemKind::FUNCTION,
            "BAML function",
        ));
    }

    // classes
    for c in rt.inner.ir.walk_classes() {
        items.push(mk_item(c.name(), CompletionItemKind::CLASS, "BAML class"));
    }

    // enums
    for e in rt.inner.ir.walk_enums() {
        items.push(mk_item(e.name(), CompletionItemKind::ENUM, "BAML enum"));
    }

    // type aliases
    for t in rt.inner.ir.walk_type_aliases() {
        items.push(mk_item(
            t.name(),
            CompletionItemKind::TYPE_PARAMETER,
            "BAML type alias",
        ));
    }

    items
}
