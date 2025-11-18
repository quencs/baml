//! Function signatures (parameters, return types, generics).
//!
//! Separated from `ItemTree` to provide fine-grained incrementality.
//! Signature changes invalidate type checking, but not name resolution.

use crate::{Name, type_ref::TypeRef};
use rowan::ast::AstNode;
use std::sync::Arc;

/// The signature of a function (everything except the body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    /// Function name (duplicated from `ItemTree` for convenience)
    pub name: Name,

    /// Function parameters
    pub params: Vec<Param>,

    /// Return type
    pub return_type: TypeRef,

    /// Type parameters (generics)
    pub type_params: Vec<TypeParam>,

    /// Attributes/modifiers
    pub attrs: FunctionAttributes,
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Name,
    pub type_ref: TypeRef,
}

/// Type parameter (for generic functions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub name: Name,
    // Future: bounds, defaults, etc.
}

/// Function attributes and modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionAttributes {
    /// Whether this is a streaming function
    pub is_streaming: bool,

    /// Whether this is async
    pub is_async: bool,

    /// Custom attributes (@@retry, @@cache, etc.)
    pub custom_attrs: Vec<CustomAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomAttribute {
    pub name: Name,
    pub args: Vec<String>, // Simplified for now
}

impl FunctionSignature {
    /// Lower a function signature from CST.
    pub fn lower(func_node: &baml_syntax::ast::FunctionDef) -> Arc<FunctionSignature> {
        let name = func_node
            .name()
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("UnnamedFunction"));

        // Extract parameters - manually since Parameter doesn't have accessor methods yet
        let mut params = Vec::new();
        if let Some(param_list) = func_node.param_list() {
            for param_node in param_list.params() {
                // PARAMETER node contains: WORD (name), optionally COLON, TYPE_EXPR
                let mut param_name = None;
                let mut param_type = TypeRef::Unknown;

                for child in param_node.syntax().children_with_tokens() {
                    if let Some(token) = child.as_token() {
                        if token.kind() == baml_syntax::SyntaxKind::WORD && param_name.is_none() {
                            param_name = Some(Name::new(token.text()));
                        }
                    } else if let Some(node) = child.as_node() {
                        if let Some(type_expr) = baml_syntax::ast::TypeExpr::cast(node.clone()) {
                            param_type = lower_type_ref(&type_expr);
                        }
                    }
                }

                if let Some(name) = param_name {
                    params.push(Param {
                        name,
                        type_ref: param_type,
                    });
                }
            }
        }

        // Extract return type
        let return_type = func_node
            .return_type()
            .map(|t| lower_type_ref(&t))
            .unwrap_or(TypeRef::Unknown);

        // Extract type parameters (future work)
        let type_params = vec![];

        // Extract attributes (future work)
        let attrs = FunctionAttributes::default();

        Arc::new(FunctionSignature {
            name,
            params,
            return_type,
            type_params,
            attrs,
        })
    }
}

/// Lower a type reference from CST.
fn lower_type_ref(node: &baml_syntax::ast::TypeExpr) -> TypeRef {
    let text = node.syntax().text().to_string();
    let text = text.trim();

    match text {
        "int" => TypeRef::Int,
        "float" => TypeRef::Float,
        "string" => TypeRef::String,
        "bool" => TypeRef::Bool,
        "null" => TypeRef::Null,
        "image" => TypeRef::Image,
        "audio" => TypeRef::Audio,
        "video" => TypeRef::Video,
        "pdf" => TypeRef::Pdf,
        _ => {
            if let Some(inner_text) = text.strip_suffix('?') {
                let inner = TypeRef::named(inner_text.into());
                TypeRef::optional(inner)
            } else if let Some(inner_text) = text.strip_suffix("[]") {
                let inner = TypeRef::named(inner_text.into());
                TypeRef::list(inner)
            } else {
                TypeRef::named(text.into())
            }
        }
    }
}
