//! `TypeExpr → Ty` lowering using package-level name resolution.

use baml_compiler2_ast::TypeExpr;
use baml_compiler2_hir::{contributions::Definition, package::PackageItems};

use crate::ty::{Freshness, LiteralValue, PrimitiveType, Ty};

/// Resolve an AST `TypeExpr` to a `Ty` using package-level name resolution.
///
/// Names are resolved against `package_items`: classes, enums, and type aliases
/// are looked up in the type namespace. Unresolved names become `Ty::Unknown`.
/// All structural recursion (Optional, List, Map, Union, Function) is handled
/// by recursive calls to this function.
pub fn lower_type_expr(type_expr: &TypeExpr, package_items: &PackageItems<'_>) -> Ty {
    match type_expr {
        TypeExpr::Path(segments) => {
            let names: Vec<baml_base::Name> = segments.clone();
            if let Some(def) = package_items.lookup_type(&names) {
                match def {
                    Definition::Class(_loc) => {
                        // Use the last segment as the class name
                        Ty::Class(segments.last().expect("non-empty path").clone())
                    }
                    Definition::Enum(_) => {
                        Ty::Enum(segments.last().expect("non-empty path").clone())
                    }
                    Definition::TypeAlias(_) => {
                        Ty::TypeAlias(segments.last().expect("non-empty path").clone())
                    }
                    _ => Ty::Unknown,
                }
            } else {
                // Not found in type namespace — unresolved
                Ty::Unknown
            }
        }
        TypeExpr::Int => Ty::Primitive(PrimitiveType::Int),
        TypeExpr::Float => Ty::Primitive(PrimitiveType::Float),
        TypeExpr::String => Ty::Primitive(PrimitiveType::String),
        TypeExpr::Bool => Ty::Primitive(PrimitiveType::Bool),
        TypeExpr::Null => Ty::Primitive(PrimitiveType::Null),
        TypeExpr::Media(kind) => Ty::Primitive(match kind {
            baml_base::MediaKind::Image => PrimitiveType::Image,
            baml_base::MediaKind::Audio => PrimitiveType::Audio,
            baml_base::MediaKind::Video => PrimitiveType::Video,
            baml_base::MediaKind::Pdf => PrimitiveType::Pdf,
            // Generic media — treated as unknown for type resolution purposes
            baml_base::MediaKind::Generic => return Ty::Unknown,
        }),
        TypeExpr::Optional(inner) => Ty::Optional(Box::new(lower_type_expr(inner, package_items))),
        TypeExpr::List(inner) => Ty::List(Box::new(lower_type_expr(inner, package_items))),
        TypeExpr::Map { key, value } => Ty::Map(
            Box::new(lower_type_expr(key, package_items)),
            Box::new(lower_type_expr(value, package_items)),
        ),
        TypeExpr::Union(members) => Ty::Union(
            members
                .iter()
                .map(|m| lower_type_expr(m, package_items))
                .collect(),
        ),
        TypeExpr::Function { params, ret } => Ty::Function {
            params: params
                .iter()
                .map(|p| (p.name.clone(), lower_type_expr(&p.ty, package_items)))
                .collect(),
            ret: Box::new(lower_type_expr(ret, package_items)),
        },
        TypeExpr::StringLiteral(s) => {
            Ty::Literal(LiteralValue::String(s.clone()), Freshness::Regular)
        }
        TypeExpr::IntLiteral(n) => Ty::Literal(LiteralValue::Int(*n), Freshness::Regular),
        TypeExpr::FloatLiteral(s) => {
            Ty::Literal(LiteralValue::Float(s.clone()), Freshness::Regular)
        }
        TypeExpr::BoolLiteral(b) => Ty::Literal(LiteralValue::Bool(*b), Freshness::Regular),
        TypeExpr::BuiltinUnknown | TypeExpr::Error | TypeExpr::Unknown => Ty::Unknown,
        TypeExpr::Type => Ty::Unknown,
    }
}
