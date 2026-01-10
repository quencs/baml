//! Jinja runtime for BAML.
//!
//! This crate provides:
//! - `render_prompt` - Render a Jinja template with BAML values
//! - `OutputFormatContent` - Schema information for parsing LLM responses
//! - `RenderedPrompt` - The result of rendering a prompt template

mod baml_value_to_jinja;
mod chat_message_part;
mod output_format;

pub use baml_value_to_jinja::IntoMiniJinjaValue;
pub use chat_message_part::ChatMessagePart;
pub use output_format::{
    Class, ClassField, Enum, EnumVariant, Name, OutputFormat, OutputFormatBuilder,
    OutputFormatContent, RenderOptions,
};

use baml_value_to_jinja::MAGIC_MEDIA_DELIMITER;

use std::collections::HashMap;

use ir_stub::{BamlMedia, BamlValue};
use indexmap::IndexMap;
use minijinja::{context, value::Kwargs, ErrorKind};
use serde::{Deserialize, Serialize};

const MAGIC_CHAT_ROLE_DELIMITER: &str = "BAML_CHAT_ROLE_MAGIC_STRING_DELIMITER";

// ============================================================================
// Render Context
// ============================================================================

/// Client configuration for rendering.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize)]
pub struct RenderContext_Client {
    /// The name of the client.
    pub name: String,
    /// The provider (e.g., "openai", "anthropic").
    pub provider: String,
    /// Default role for messages without explicit role.
    pub default_role: String,
    /// Allowed roles for this client.
    pub allowed_roles: Vec<String>,
    /// Role remapping (e.g., "user" -> "human" for Anthropic).
    pub remap_role: HashMap<String, String>,
    /// Additional client options.
    pub options: IndexMap<String, serde_json::Value>,
}

impl Default for RenderContext_Client {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            provider: "openai".to_string(),
            default_role: "system".to_string(),
            allowed_roles: vec![
                "system".to_string(),
                "user".to_string(),
                "assistant".to_string(),
            ],
            remap_role: HashMap::new(),
            options: IndexMap::new(),
        }
    }
}

/// Context for rendering a prompt.
#[derive(Debug)]
pub struct RenderContext {
    /// Client configuration.
    pub client: RenderContext_Client,
    /// Output format schema.
    pub output_format: OutputFormatContent,
    /// Tags available in the template.
    pub tags: HashMap<String, BamlValue>,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            client: RenderContext_Client::default(),
            output_format: OutputFormatContent::empty(),
            tags: HashMap::new(),
        }
    }
}

// ============================================================================
// Rendered Prompt
// ============================================================================

/// A rendered chat message.
#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct RenderedChatMessage {
    /// The role of this message.
    pub role: String,
    /// Whether duplicate roles are allowed.
    pub allow_duplicate_role: bool,
    /// The message parts.
    pub parts: Vec<ChatMessagePart>,
}

/// The result of rendering a prompt template.
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum RenderedPrompt {
    /// A completion prompt (single string).
    Completion(String),
    /// A chat prompt (multiple messages with roles).
    Chat(Vec<RenderedChatMessage>),
}

impl std::fmt::Display for RenderedPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RenderedPrompt::Completion(s) => write!(f, "{s}"),
            RenderedPrompt::Chat(messages) => {
                for message in messages {
                    writeln!(
                        f,
                        "{}: {}",
                        message.role,
                        message
                            .parts
                            .iter()
                            .map(ChatMessagePart::to_string)
                            .collect::<Vec<String>>()
                            .join("")
                    )?;
                }
                Ok(())
            }
        }
    }
}

// ============================================================================
// Render Error
// ============================================================================

/// Error during template rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Minijinja error: {0}")]
    MiniJinja(#[from] minijinja::Error),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Render error: {0}")]
    Other(String),
}

// ============================================================================
// Render Functions
// ============================================================================

/// Render a prompt template with the given arguments and context.
///
/// This is the main entry point for rendering BAML prompts.
pub fn render_prompt(
    template: &str,
    args: &BamlValue,
    ctx: RenderContext,
) -> Result<RenderedPrompt, RenderError> {
    let default_role = ctx.client.default_role.clone();
    let allowed_roles = ctx.client.allowed_roles.clone();
    let remap_role = ctx.client.remap_role.clone();

    // Convert args to minijinja value
    let args_jinja = args.to_minijinja_value();

    render_minijinja(
        template,
        &args_jinja,
        ctx,
        default_role,
        allowed_roles,
        remap_role,
    )
}

/// Render a template with simple variable substitution (legacy).
///
/// For backwards compatibility with simple use cases.
pub fn render_template(
    template: &str,
    context: &IndexMap<String, BamlValue>,
) -> Result<String, RenderError> {
    let mut env = minijinja::Environment::new();
    env.add_template("template", template)?;

    let tmpl = env.get_template("template")?;

    // Convert context to minijinja values
    let ctx: IndexMap<&str, minijinja::Value> = context
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_minijinja_value()))
        .collect();

    let rendered = tmpl.render(minijinja::Value::from_iter(ctx))?;
    Ok(rendered)
}

fn render_minijinja(
    template: &str,
    args: &minijinja::Value,
    ctx: RenderContext,
    default_role: String,
    allowed_roles: Vec<String>,
    remap_role: HashMap<String, String>,
) -> Result<RenderedPrompt, RenderError> {
    let mut env = minijinja::Environment::new();

    // Allow undefined variables to render as empty string instead of erroring
    // This enables testing templates without providing all required arguments
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

    // Dedent the template
    let whitespace_length = template
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
        .min()
        .unwrap_or(0);
    let template = template
        .split('\n')
        .map(|line| line.chars().skip(whitespace_length).collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");
    let template = template.trim();

    env.add_template("prompt", template)?;

    // Add ctx global with output_format as a callable object
    let client = ctx.client.clone();
    let tags = ctx.tags.clone();
    let output_format = OutputFormat::from_content(ctx.output_format.clone());
    env.add_global(
        "ctx",
        context! {
            client => client,
            tags => tags,
            output_format => minijinja::Value::from_object(output_format),
        },
    );

    // Add the role function for _.chat() / _.role()
    let role_fn = minijinja::Value::from_function(
        |role: Option<String>, kwargs: Kwargs| -> Result<String, minijinja::Error> {
            let role = match (role, kwargs.get::<String>("role")) {
                (Some(b), Ok(a)) => {
                    return Err(minijinja::Error::new(
                        ErrorKind::TooManyArguments,
                        format!("role() called with two roles: '{a}' and '{b}'"),
                    ));
                }
                (Some(role), _) => role,
                (_, Ok(role)) => role,
                _ => {
                    return Err(minijinja::Error::new(
                        ErrorKind::MissingArgument,
                        "role() called without role. Try role('role') or role(role='role').",
                    ));
                }
            };

            let allow_duplicate_role = match kwargs.get::<bool>("__baml_allow_dupe_role__") {
                Ok(allow) => allow,
                Err(e) => match e.kind() {
                    ErrorKind::MissingArgument => false,
                    _ => return Err(e),
                },
            };

            let additional_properties = {
                let mut props = kwargs
                    .args()
                    .filter(|&k| k != "role")
                    .map(|k| {
                        Ok((
                            k,
                            serde_json::Value::deserialize(kwargs.get::<minijinja::Value>(k)?)?,
                        ))
                    })
                    .collect::<Result<HashMap<&str, serde_json::Value>, minijinja::Error>>()?;

                props.insert("role", role.clone().into());
                props.insert("__baml_allow_dupe_role__", allow_duplicate_role.into());
                props
            };

            let additional_properties = serde_json::json!(additional_properties).to_string();

            Ok(format!(
                "{MAGIC_CHAT_ROLE_DELIMITER}:baml-start-baml:{additional_properties}:baml-end-baml:{MAGIC_CHAT_ROLE_DELIMITER}"
            ))
        },
    );

    env.add_global(
        "_",
        context! {
            chat => role_fn,
            role => role_fn
        },
    );

    let tmpl = env.get_template("prompt")?;
    let rendered = tmpl.render(args)?;

    // If no chat delimiters, return as completion
    if !rendered.contains(MAGIC_CHAT_ROLE_DELIMITER) && !rendered.contains(MAGIC_MEDIA_DELIMITER) {
        return Ok(RenderedPrompt::Completion(rendered));
    }

    // Parse chat messages
    let mut chat_messages = vec![];
    let mut role = None;
    let mut meta = None;
    let mut allow_duplicate_role = false;

    for chunk in rendered.split(MAGIC_CHAT_ROLE_DELIMITER) {
        if chunk.starts_with(":baml-start-baml:") && chunk.ends_with(":baml-end-baml:") {
            let parsed = chunk
                .strip_prefix(":baml-start-baml:")
                .unwrap_or(chunk)
                .strip_suffix(":baml-end-baml:")
                .unwrap_or(chunk);

            if let Ok(mut parsed) =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(parsed)
            {
                if let Some(role_val) = parsed.remove("role") {
                    role = Some(role_val.as_str().unwrap_or("").to_string());
                }

                allow_duplicate_role = parsed
                    .remove("__baml_allow_dupe_role__")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if parsed.is_empty() {
                    meta = None;
                } else {
                    meta = Some(parsed);
                }
            }
        } else if role.is_none() && chunk.is_empty() {
            // Discard whitespace before first _.chat()
        } else {
            let mut parts = vec![];

            for part in chunk.split(MAGIC_MEDIA_DELIMITER) {
                let part = if part.starts_with(":baml-start-media:")
                    && part.ends_with(":baml-end-media:")
                {
                    let media_data = part
                        .strip_prefix(":baml-start-media:")
                        .unwrap_or(part)
                        .strip_suffix(":baml-end-media:")
                        .unwrap_or(part);

                    match serde_json::from_str::<BamlMedia>(media_data) {
                        Ok(m) => Some(ChatMessagePart::Media(m)),
                        Err(_) => {
                            return Err(RenderError::Other(format!(
                                "Media variable had unrecognizable data: {media_data}"
                            )))
                        }
                    }
                } else if !part.trim().is_empty() {
                    Some(ChatMessagePart::Text(part.trim().to_string()))
                } else {
                    None
                };

                if let Some(part) = part {
                    if let Some(meta) = &meta {
                        parts.push(part.with_meta(meta.clone()));
                    } else {
                        parts.push(part);
                    }
                }
            }

            if !parts.is_empty() {
                chat_messages.push(RenderedChatMessage {
                    role: match role.as_ref() {
                        Some(r) if allowed_roles.contains(r) => r.clone(),
                        Some(_) => default_role.clone(),
                        None => default_role.clone(),
                    },
                    allow_duplicate_role,
                    parts,
                });
            }
        }
    }

    // Apply role remapping
    for msg in &mut chat_messages {
        if let Some(remap) = remap_role.get(&msg.role) {
            msg.role = remap.clone();
        }
    }

    Ok(RenderedPrompt::Chat(chat_messages))
}

/// Evaluate a Jinja expression on a BamlValue.
///
/// Used for constraint evaluation.
pub fn evaluate_predicate(
    value: &BamlValue,
    expression: &ir_stub::JinjaExpression,
) -> Result<bool, RenderError> {
    let mut env = minijinja::Environment::new();

    // Create a template that evaluates the expression
    let template = format!("{{% if {} %}}true{{% else %}}false{{% endif %}}", expression.0);
    env.add_template("predicate", &template)?;

    let tmpl = env.get_template("predicate")?;
    let jinja_value = value.to_minijinja_value();

    // Wrap the value as "this" for the expression
    let ctx = context! {
        this => jinja_value,
    };

    let result = tmpl.render(ctx)?;
    Ok(result.trim() == "true")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir_stub::BamlMap;

    #[test]
    fn test_render_template_simple() {
        let mut ctx = IndexMap::new();
        ctx.insert("name".to_string(), BamlValue::String("Alice".to_string()));

        let result = render_template("Hello, {{ name }}!", &ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, Alice!");
    }

    #[test]
    fn test_render_template_with_int() {
        let mut ctx = IndexMap::new();
        ctx.insert("count".to_string(), BamlValue::Int(42));

        let result = render_template("Count: {{ count }}", &ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Count: 42");
    }

    #[test]
    fn test_render_prompt_completion() {
        let mut args = BamlMap::new();
        args.insert("text".to_string(), BamlValue::String("Hello".to_string()));
        let args = BamlValue::Map(args);

        let result = render_prompt("Process: {{ text }}", &args, RenderContext::default());

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => assert_eq!(s, "Process: Hello"),
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_render_prompt_chat() {
        let mut args = BamlMap::new();
        args.insert("text".to_string(), BamlValue::String("Hello".to_string()));
        let args = BamlValue::Map(args);

        let template = r#"{{ _.chat("system") }}
You are a helpful assistant.
{{ _.chat("user") }}
{{ text }}"#;

        let result = render_prompt(template, &args, RenderContext::default());

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Chat(messages) => {
                assert_eq!(messages.len(), 2);
                assert_eq!(messages[0].role, "system");
                assert_eq!(messages[1].role, "user");
            }
            _ => panic!("Expected Chat"),
        }
    }

    #[test]
    fn test_render_prompt_with_list() {
        let args = BamlValue::Map({
            let mut m = BamlMap::new();
            m.insert(
                "items".to_string(),
                BamlValue::List(vec![
                    BamlValue::String("one".to_string()),
                    BamlValue::String("two".to_string()),
                ]),
            );
            m
        });

        let template = r#"{% for item in items %}- {{ item }}
{% endfor %}"#;

        let result = render_prompt(template, &args, RenderContext::default());
        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => {
                assert!(s.contains("- one"));
                assert!(s.contains("- two"));
            }
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_render_prompt_role_alias() {
        let args = BamlValue::Map(BamlMap::new());

        let template = r#"{{ _.role("system") }}
Hello"#;

        let result = render_prompt(template, &args, RenderContext::default());

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Chat(messages) => {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].role, "system");
            }
            _ => panic!("Expected Chat"),
        }
    }

    #[test]
    fn test_evaluate_predicate() {
        let value = BamlValue::Int(42);
        let expr = ir_stub::JinjaExpression("this > 10".to_string());

        let result = evaluate_predicate(&value, &expr);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_predicate_false() {
        let value = BamlValue::Int(5);
        let expr = ir_stub::JinjaExpression("this > 10".to_string());

        let result = evaluate_predicate(&value, &expr);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_evaluate_predicate_string() {
        let value = BamlValue::String("hello world".to_string());
        let expr = ir_stub::JinjaExpression("this | length > 5".to_string());

        let result = evaluate_predicate(&value, &expr);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_ctx_output_format_int() {
        use crate::output_format::OutputFormatBuilder;
        use baml_base::Ty;

        let args = BamlValue::Map(BamlMap::new());

        let output_format = OutputFormatBuilder::new()
            .with_target(Ty::Int)
            .build();

        let ctx = RenderContext {
            output_format,
            ..Default::default()
        };

        let template = "Return: {{ ctx.output_format }}";
        let result = render_prompt(template, &args, ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => {
                assert!(s.contains("Answer as an int"), "Expected 'Answer as an int' but got: {}", s);
            }
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_ctx_output_format_class() {
        use crate::output_format::{Class, OutputFormatBuilder};
        use baml_base::{Ty, Name as BaseName};

        let args = BamlValue::Map(BamlMap::new());

        let person_class = Class::new("Person")
            .with_field("name", Ty::String, Some("The person's name".to_string()), true)
            .with_field("age", Ty::Int, None, true);

        let output_format = OutputFormatBuilder::new()
            .with_class(person_class)
            .with_target(Ty::Class(BaseName::from("Person")))
            .build();

        let ctx = RenderContext {
            output_format,
            ..Default::default()
        };

        let template = "Return JSON:\n{{ ctx.output_format }}";
        let result = render_prompt(template, &args, ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => {
                assert!(s.contains("name: string"), "Expected 'name: string' but got: {}", s);
                assert!(s.contains("age: int"), "Expected 'age: int' but got: {}", s);
                assert!(s.contains("The person's name"), "Expected description but got: {}", s);
            }
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_ctx_output_format_callable_with_kwargs() {
        use crate::output_format::{Enum, OutputFormatBuilder};
        use baml_base::{Ty, Name as BaseName};

        let args = BamlValue::Map(BamlMap::new());

        let color_enum = Enum::new("Color")
            .with_variant("red", None)
            .with_variant("green", None)
            .with_variant("blue", None);

        let output_format = OutputFormatBuilder::new()
            .with_enum(color_enum)
            .with_target(Ty::Enum(BaseName::from("Color")))
            .build();

        let ctx = RenderContext {
            output_format,
            ..Default::default()
        };

        // Test with prefix=null to suppress the "Answer with any of the categories:" prefix
        let template = "{{ ctx.output_format(prefix=null) }}";
        let result = render_prompt(template, &args, ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => {
                // Top-level enums render in full format with name and values
                assert!(s.contains("Color"), "Expected 'Color' but got: {}", s);
                assert!(s.contains("- red"), "Expected '- red' but got: {}", s);
                assert!(s.contains("- green"), "Expected '- green' but got: {}", s);
                assert!(s.contains("- blue"), "Expected '- blue' but got: {}", s);
                // Should NOT contain the prefix since we set prefix=null
                assert!(!s.contains("Answer with"), "Should not contain prefix but got: {}", s);
            }
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_ctx_output_format_custom_or_splitter() {
        use crate::output_format::{Class, Enum, OutputFormatBuilder};
        use baml_base::{Ty, Name as BaseName};

        let args = BamlValue::Map(BamlMap::new());

        // Create an enum that's used in a class field (so it renders inline)
        let status_enum = Enum::new("Status")
            .with_variant("pending", None)
            .with_variant("done", None);

        let task_class = Class::new("Task")
            .with_field("status", Ty::Enum(BaseName::from("Status")), None, true);

        let output_format = OutputFormatBuilder::new()
            .with_enum(status_enum)
            .with_class(task_class)
            .with_target(Ty::Class(BaseName::from("Task")))
            .build();

        let ctx = RenderContext {
            output_format,
            ..Default::default()
        };

        // Test with custom or_splitter
        let template = "{{ ctx.output_format(or_splitter=' | ') }}";
        let result = render_prompt(template, &args, ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RenderedPrompt::Completion(s) => {
                // Enum values in class field should use custom or_splitter
                assert!(s.contains("'pending' | 'done'"), "Expected custom or_splitter but got: {}", s);
            }
            _ => panic!("Expected Completion"),
        }
    }
}
