//! Prepared function - validated and ready for execution.

use baml_program::Ty;
use indexmap::IndexMap;

use crate::types::BamlValue;

/// A value paired with its type information.
#[derive(Debug, Clone)]
pub struct TypedArg {
    pub value: BamlValue,
    pub arg_ty: Ty,
}

/// A function that has been validated and prepared for execution.
///
/// This struct contains all the information needed to render prompts
/// and execute the function against an LLM provider.
#[derive(Debug, Clone)]
pub struct PreparedFunction {
    /// Function name.
    pub function_name: String,
    /// Validated and coerced arguments.
    pub args: IndexMap<String, BamlValue>,
    /// Type-annotated arguments (for constraint checking).
    pub args_with_types: IndexMap<String, TypedArg>,
    /// Return type.
    pub return_ty: Ty,
    /// Client name.
    pub client_name: String,
    /// Prompt template string.
    pub prompt_template: String,
}

impl PreparedFunction {
    /// Create a new prepared function with minimal configuration.
    /// Used for testing and stub implementations.
    pub fn new_stub(
        function_name: impl Into<String>,
        args: IndexMap<String, BamlValue>,
        return_ty: Ty,
        client_name: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self {
        let args_with_types = args
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    TypedArg {
                        value: v.clone(),
                        arg_ty: Ty::String,
                    },
                )
            })
            .collect();

        Self {
            function_name: function_name.into(),
            args,
            args_with_types,
            return_ty,
            client_name: client_name.into(),
            prompt_template: prompt_template.into(),
        }
    }
}
