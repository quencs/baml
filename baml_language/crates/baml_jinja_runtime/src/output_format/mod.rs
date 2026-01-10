//! Output format types and rendering for BAML schemas.
//!
//! This module provides:
//! - `OutputFormatContent` - Container for all type schemas (re-exported from baml_output_format)
//! - `OutputFormat` - Wrapper implementing `minijinja::value::Object` for callable support
//! - `RenderOptions` - Options for rendering output format (re-exported from baml_output_format)

// Re-export from baml_output_format
pub use baml_output_format::{
    render, Class, ClassField, Enum, EnumVariant, HoistClasses, MapStyle, Name,
    OutputFormatBuilder, OutputFormatContent, RenderOptions,
};

use minijinja::{value::Kwargs, ErrorKind, Value};
use std::sync::Arc;

use crate::RenderContext;

/// Wrapper around OutputFormatContent that implements minijinja::value::Object.
///
/// This makes `ctx.output_format` callable in Jinja templates with kwargs support.
#[derive(Debug)]
pub struct OutputFormat {
    content: OutputFormatContent,
}

impl OutputFormat {
    /// Create a new OutputFormat from a RenderContext.
    pub fn new(ctx: &RenderContext) -> Self {
        Self {
            content: ctx.output_format.clone(),
        }
    }

    /// Create a new OutputFormat from OutputFormatContent directly.
    pub fn from_content(content: OutputFormatContent) -> Self {
        Self { content }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match render(&self.content, &RenderOptions::default()) {
            Ok(Some(content)) => write!(f, "{}", content),
            Ok(None) => Ok(()),
            Err(e) => {
                // Log the error for debugging purposes before returning the opaque fmt::Error
                #[cfg(debug_assertions)]
                eprintln!("OutputFormat render error: {}", e);
                Err(std::fmt::Error)
            }
        }
    }
}

impl minijinja::value::Object for OutputFormat {
    fn call(
        self: &Arc<Self>,
        _state: &minijinja::State<'_, '_>,
        args: &[Value],
    ) -> Result<Value, minijinja::Error> {
        use minijinja::{value::from_args, Error};

        let (positional, kwargs): (&[Value], Kwargs) = from_args(args)?;
        if !positional.is_empty() {
            return Err(Error::new(
                ErrorKind::TooManyArguments,
                "output_format() may only be called with named arguments",
            ));
        }

        // Parse prefix kwarg
        let prefix = if kwargs.has("prefix") {
            match kwargs.get::<Option<String>>("prefix") {
                Ok(prefix) => Some(prefix),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for prefix (expected string | null): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse or_splitter kwarg
        let or_splitter = if kwargs.has("or_splitter") {
            match kwargs.get::<String>("or_splitter") {
                Ok(value) => Some(value),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for or_splitter (expected string): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse enum_value_prefix kwarg
        let enum_value_prefix = if kwargs.has("enum_value_prefix") {
            match kwargs.get::<Option<String>>("enum_value_prefix") {
                Ok(prefix) => Some(prefix),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for enum_value_prefix (expected string | null): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse always_hoist_enums kwarg
        let always_hoist_enums = if kwargs.has("always_hoist_enums") {
            match kwargs.get::<bool>("always_hoist_enums") {
                Ok(value) => Some(value),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for always_hoist_enums (expected bool): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse hoisted_class_prefix kwarg
        let hoisted_class_prefix = if kwargs.has("hoisted_class_prefix") {
            match kwargs.get::<Option<String>>("hoisted_class_prefix") {
                Ok(prefix) => Some(prefix),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for hoisted_class_prefix (expected string | null): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse hoist_classes kwarg
        let hoist_classes = if kwargs.has("hoist_classes") {
            // Try bool first
            match kwargs.get::<bool>("hoist_classes") {
                Ok(true) => Some(HoistClasses::All),
                Ok(false) => Some(HoistClasses::Auto),
                // Try string "auto"
                Err(_) => match kwargs.get::<String>("hoist_classes") {
                    Ok(s) if s == "auto" => Some(HoistClasses::Auto),
                    // Try array of class names
                    _ => match kwargs.get::<Vec<String>>("hoist_classes") {
                        Ok(classes) => Some(HoistClasses::Subset(classes)),
                        Err(e) => {
                            return Err(Error::new(
                                ErrorKind::SyntaxError,
                                format!(
                                    "Invalid value for hoist_classes (expected bool | \"auto\" | string[]): {e}"
                                ),
                            ))
                        }
                    },
                },
            }
        } else {
            None
        };

        // Parse map_style kwarg
        let map_style = if kwargs.has("map_style") {
            match kwargs.get::<String>("map_style") {
                Ok(s) => match s.parse::<MapStyle>() {
                    Ok(style) => Some(style),
                    Err(e) => {
                        return Err(Error::new(
                            ErrorKind::SyntaxError,
                            format!("Invalid value for map_style (expected 'angle' or 'object'): {e}"),
                        ))
                    }
                },
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for map_style (expected 'angle' or 'object'): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Parse quote_class_fields kwarg
        let quote_class_fields = if kwargs.has("quote_class_fields") {
            match kwargs.get::<bool>("quote_class_fields") {
                Ok(value) => Some(value),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::SyntaxError,
                        format!("Invalid value for quote_class_fields (expected bool): {e}"),
                    ))
                }
            }
        } else {
            None
        };

        // Check for unknown kwargs
        if kwargs.assert_all_used().is_err() {
            return Err(Error::new(
                ErrorKind::TooManyArguments,
                "output_format() got an unexpected keyword argument (only 'prefix', 'always_hoist_enums', 'enum_value_prefix', 'or_splitter', 'hoisted_class_prefix', 'hoist_classes', 'map_style', and 'quote_class_fields' are allowed)",
            ));
        }

        // Build options and render
        let options = RenderOptions::new(
            prefix,
            or_splitter,
            enum_value_prefix,
            always_hoist_enums,
            map_style,
            hoisted_class_prefix,
            hoist_classes,
            quote_class_fields,
        );

        let content = render(&self.content, &options).map_err(|e| {
            minijinja::Error::new(ErrorKind::InvalidOperation, e.to_string())
        })?;

        match content {
            Some(content) => Ok(Value::from_safe_string(content)),
            None => Ok(Value::from("")),
        }
    }

    fn call_method(
        self: &Arc<Self>,
        _state: &minijinja::State<'_, '_>,
        name: &str,
        _args: &[Value],
    ) -> Result<Value, minijinja::Error> {
        Err(minijinja::Error::new(
            ErrorKind::UnknownMethod,
            format!("output_format has no callable attribute '{name}'"),
        ))
    }

    fn render(self: &Arc<Self>, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}
