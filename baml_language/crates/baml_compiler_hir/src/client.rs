//! Client validation logic for HIR lowering.
//!
//! This module contains validation for client definitions, including:
//! - `client_response_type` validation
//! - `http` configuration block validation

use baml_base::Name;
use baml_compiler_diagnostics::HirDiagnostic;
use baml_compiler_syntax::ast::{ClientDef, ConfigBlock};
use rowan::ast::AstNode;

use crate::{LoweringContext, item_tree::Client};

/// Valid values for `client_response_type`.
pub(crate) const VALID_RESPONSE_TYPES: &[&str] = &[
    "openai",
    "openai-responses",
    "anthropic",
    "google",
    "vertex",
    "openrouter",
];

/// Valid http config fields for regular (non-composite) clients.
pub(crate) const REGULAR_HTTP_FIELDS: &[&str] = &[
    "connect_timeout_ms",
    "request_timeout_ms",
    "time_to_first_token_timeout_ms",
    "idle_timeout_ms",
];

/// Valid http config fields for composite clients (fallback/round-robin).
pub(crate) const COMPOSITE_HTTP_FIELDS: &[&str] = &["total_timeout_ms"];

/// Composite client providers.
pub(crate) const COMPOSITE_PROVIDERS: &[&str] = &["fallback", "round-robin"];

/// Extract client configuration from CST with validation.
pub(crate) fn lower_client(
    node: &baml_compiler_syntax::SyntaxNode,
    ctx: &mut LoweringContext,
) -> Option<Client> {
    let client_def = ClientDef::cast(node.clone())?;

    // Extract name using AST accessor
    let name = client_def
        .name()
        .map(|t| Name::new(t.text()))
        .unwrap_or_else(|| Name::new("UnnamedClient"));
    let client_name = name.to_string();

    // Extract provider from config block using AST accessors
    let provider_item = client_def
        .config_block()
        .and_then(|block| block.items().find(|item| item.matches_key("provider")));

    let provider = provider_item
        .as_ref()
        .and_then(baml_compiler_syntax::ast::ConfigItem::value_word)
        .map(|t| Name::new(t.text()))
        .unwrap_or_else(|| Name::new("unknown"));

    // Validate that provider field exists
    if provider_item.is_none() {
        // Get the span for the entire client definition
        ctx.push_diagnostic(HirDiagnostic::MissingProvider {
            client_name: client_name.clone(),
            span: ctx.span(node.text_range()),
        });
    }

    let is_composite = COMPOSITE_PROVIDERS.contains(&provider.as_str());

    // Validate config block fields
    if let Some(config_block) = client_def.config_block() {
        // Check for unknown properties in client config block
        for item in config_block.items() {
            if let Some(key) = item.key() {
                let key_text = key.text();
                if key_text != "provider" && key_text != "options" {
                    ctx.push_diagnostic(HirDiagnostic::UnknownClientProperty {
                        client_name: client_name.clone(),
                        field_name: key_text.to_string(),
                        span: ctx.span(key.text_range()),
                    });
                }
            }
        }

        // Find the options block for further validation
        if let Some(options_item) = config_block
            .items()
            .find(|item| item.matches_key("options"))
        {
            if let Some(options_block) = options_item.nested_block() {
                // Validate client_response_type
                validate_client_response_type(ctx, &client_name, &options_block);

                // Validate http block
                validate_http_block(ctx, &client_name, &options_block, is_composite);
            }
        }
    }

    Some(Client { name, provider })
}

/// Validate `client_response_type` field.
fn validate_client_response_type(
    ctx: &mut LoweringContext,
    client_name: &str,
    options_block: &ConfigBlock,
) {
    if let Some(response_type_item) = options_block
        .items()
        .find(|item| item.matches_key("client_response_type"))
    {
        if let Some(value) = response_type_item.value_str() {
            if !VALID_RESPONSE_TYPES.contains(&value.as_str()) {
                if let Some(value_range) = response_type_item.value_text_range() {
                    ctx.push_diagnostic(HirDiagnostic::InvalidClientResponseType {
                        client_name: client_name.to_string(),
                        value,
                        span: ctx.span(value_range),
                        valid_values: VALID_RESPONSE_TYPES.to_vec(),
                    });
                }
            }
        }
    }
}

/// Validate http configuration block.
fn validate_http_block(
    ctx: &mut LoweringContext,
    client_name: &str,
    options_block: &ConfigBlock,
    is_composite: bool,
) {
    if let Some(http_item) = options_block.items().find(|item| item.matches_key("http")) {
        // Check if http has a nested block or a scalar value
        if let Some(http_block) = http_item.nested_block() {
            // Validate http config fields
            validate_http_config_fields(ctx, client_name, &http_block, is_composite);
        } else if http_item.has_value() {
            // http is a scalar value, not a block - this is an error
            if let Some(key_token) = http_item.key() {
                ctx.push_diagnostic(HirDiagnostic::HttpConfigNotBlock {
                    client_name: client_name.to_string(),
                    span: ctx.span(key_token.text_range()),
                });
            }
        }
    }
}

/// Validate http configuration block fields.
fn validate_http_config_fields(
    ctx: &mut LoweringContext,
    client_name: &str,
    http_block: &ConfigBlock,
    is_composite: bool,
) {
    let valid_fields = if is_composite {
        COMPOSITE_HTTP_FIELDS
    } else {
        REGULAR_HTTP_FIELDS
    };

    for item in http_block.items() {
        let Some(key_token) = item.key() else {
            continue;
        };
        let field_name = key_token.text();
        let field_span = ctx.span(key_token.text_range());

        // Check if field is valid
        if !valid_fields.contains(&field_name) {
            // For composite clients, check if they're trying to use regular fields
            // For regular clients, check if they're trying to use composite fields
            let suggestion = find_similar_field(field_name, valid_fields);

            ctx.push_diagnostic(HirDiagnostic::UnknownHttpConfigField {
                client_name: client_name.to_string(),
                field_name: field_name.to_string(),
                span: field_span,
                suggestion,
                is_composite,
            });
            continue;
        }

        // Validate timeout values are non-negative
        if field_name.ends_with("_ms") {
            // Check if negative (has minus sign)
            if item.is_negative() {
                if let Some(value) = item.value_int() {
                    ctx.push_diagnostic(HirDiagnostic::NegativeTimeout {
                        client_name: client_name.to_string(),
                        field_name: field_name.to_string(),
                        value: -value.abs(), // Make sure it's negative for display
                        span: field_span,
                    });
                }
            }
        }
    }
}

/// Find a similar field name for suggestions using edit distance.
fn find_similar_field(field_name: &str, valid_fields: &[&str]) -> Option<String> {
    // If there's only one valid field, always suggest it
    if valid_fields.len() == 1 {
        return Some(valid_fields[0].to_string());
    }

    baml_base::find_similar_names(field_name, valid_fields.iter().copied(), 1)
        .into_iter()
        .next()
}
