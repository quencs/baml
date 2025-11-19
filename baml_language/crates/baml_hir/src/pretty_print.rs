//! Pretty-printing for HIR structures.
//!
//! This module provides pretty-printing functionality for the HIR using the `pretty` library.
//! The output is canonical formatted BAML code based on the HIR structure, without preserving
//! original formatting, comments, or whitespace.

use crate::{
    body::*, item_tree::*, signature::*, type_ref::TypeRef, FunctionMarker, ItemTree, LocalItemId,
};
use pretty::{DocAllocator, DocBuilder, RcAllocator};

/// Pretty-print an entire ItemTree.
///
/// This is the main entry point for pretty-printing. It will format all items
/// in the tree (functions, classes, enums, etc.) with proper spacing.
pub fn pretty_print_item_tree(
    tree: &ItemTree,
    width: usize,
    get_body: impl Fn(LocalItemId<FunctionMarker>) -> Option<(FunctionSignature, FunctionBody)>,
) -> String {
    let arena = RcAllocator;
    let mut docs = Vec::new();

    // Sort items by name for deterministic output
    let mut funcs: Vec<_> = tree.functions.iter().collect();
    funcs.sort_by(|a, b| a.1.name.as_str().cmp(b.1.name.as_str()));

    let mut classes: Vec<_> = tree.classes.iter().collect();
    classes.sort_by(|a, b| a.1.name.as_str().cmp(b.1.name.as_str()));

    let mut enums: Vec<_> = tree.enums.iter().collect();
    enums.sort_by(|a, b| a.1.name.as_str().cmp(b.1.name.as_str()));

    let mut clients: Vec<_> = tree.clients.iter().collect();
    clients.sort_by(|a, b| a.1.name.as_str().cmp(b.1.name.as_str()));

    // Print classes
    for (_, class) in classes {
        docs.push(pretty_print_class_doc(&arena, class));
    }

    // Print enums
    for (_, enum_def) in enums {
        docs.push(pretty_print_enum_doc(&arena, enum_def));
    }

    // Print clients
    for (_, client) in clients {
        docs.push(pretty_print_client_doc(&arena, client));
    }

    let mut output = Vec::new();
    let doc = arena.intersperse(docs, arena.hardline().append(arena.hardline()));
    doc.render(width, &mut output).unwrap();

    // Print functions
    // We render each function immediately to avoid lifetime issues with DocBuilder
    // borrowing from local variables (sig, body)
    for (id, func) in funcs {
        // Add separator if we have previous content
        if !output.is_empty() {
            output.extend_from_slice(b"\n\n");
        }

        if let Some((sig, body)) = get_body(*id) {
            let func_doc = pretty_print_function_doc(&arena, func, &sig, &body);
            func_doc.render(width, &mut output).unwrap();
        } else {
            let func_doc = pretty_print_function_minimal_doc(&arena, func);
            func_doc.render(width, &mut output).unwrap();
        };
    }

    String::from_utf8(output).unwrap()
}

/// Pretty-print a function with its signature and body.
pub fn pretty_print_function(
    func: &Function,
    sig: &FunctionSignature,
    body: &FunctionBody,
    width: usize,
) -> String {
    let arena = RcAllocator;
    let doc = pretty_print_function_doc(&arena, func, sig, body);
    let mut output = Vec::new();
    doc.render(width, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Pretty-print a class definition.
pub fn pretty_print_class(class: &Class, width: usize) -> String {
    let arena = RcAllocator;
    let doc = pretty_print_class_doc(&arena, class);
    let mut output = Vec::new();
    doc.render(width, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Pretty-print an enum definition.
pub fn pretty_print_enum(enum_def: &Enum, width: usize) -> String {
    let arena = RcAllocator;
    let doc = pretty_print_enum_doc(&arena, enum_def);
    let mut output = Vec::new();
    doc.render(width, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

//
// ──────────────────────────────────────────────────── INTERNAL HELPERS ─────
//

fn pretty_print_function_minimal_doc<'a>(
    arena: &'a RcAllocator,
    func: &'a Function,
) -> DocBuilder<'a, RcAllocator> {
    arena
        .text("function")
        .append(arena.space())
        .append(arena.text(func.name.as_str()))
        .append(arena.text("() { /* body omitted */ }"))
}

fn pretty_print_function_doc<'a>(
    arena: &'a RcAllocator,
    func: &'a Function,
    sig: &'a FunctionSignature,
    body: &'a FunctionBody,
) -> DocBuilder<'a, RcAllocator> {
    let header = arena
        .text("function")
        .append(arena.space())
        .append(arena.text(func.name.as_str()));

    // Parameters
    let params = if sig.params.is_empty() {
        arena.text("()")
    } else {
        let param_docs: Vec<_> = sig
            .params
            .iter()
            .map(|p| {
                arena
                    .text(p.name.as_str())
                    .append(arena.text(":"))
                    .append(arena.space())
                    .append(pretty_print_type_ref(arena, &p.type_ref))
            })
            .collect();

        arena
            .text("(")
            .append(arena.intersperse(param_docs, arena.text(",").append(arena.space())))
            .append(arena.text(")"))
    };

    // Return type
    let return_type = arena
        .space()
        .append(arena.text("->"))
        .append(arena.space())
        .append(pretty_print_type_ref(arena, &sig.return_type));

    // Body
    let body_doc = match body {
        FunctionBody::Llm(llm_body) => pretty_print_llm_body(arena, llm_body),
        FunctionBody::Expr(expr_body) => pretty_print_expr_body(arena, expr_body),
        FunctionBody::Missing => arena.text("{ /* missing */ }"),
    };

    header
        .append(params)
        .append(return_type)
        .append(arena.space())
        .append(body_doc)
}

fn pretty_print_class_doc<'a>(arena: &'a RcAllocator, class: &'a Class) -> DocBuilder<'a, RcAllocator> {
    let header = arena
        .text("class")
        .append(arena.space())
        .append(arena.text(class.name.as_str()))
        .append(arena.space())
        .append(arena.text("{"));

    if class.fields.is_empty() {
        return header.append(arena.text("}"));
    }

    let field_docs: Vec<_> = class
        .fields
        .iter()
        .map(|f| {
            arena
                .text(f.name.as_str())
                .append(arena.text(":"))
                .append(arena.space())
                .append(pretty_print_type_ref(arena, &f.type_ref))
        })
        .collect();

    let fields = arena
        .hardline()
        .append(arena.intersperse(field_docs, arena.hardline()))
        .nest(2)
        .append(arena.hardline());

    let footer = if class.is_dynamic {
        arena
            .hardline()
            .append(arena.text("@@dynamic"))
            .nest(2)
            .append(arena.hardline())
            .append(arena.text("}"))
    } else {
        arena.text("}")
    };

    header.append(fields).append(footer)
}

fn pretty_print_enum_doc<'a>(arena: &'a RcAllocator, enum_def: &'a Enum) -> DocBuilder<'a, RcAllocator> {
    let header = arena
        .text("enum")
        .append(arena.space())
        .append(arena.text(enum_def.name.as_str()))
        .append(arena.space())
        .append(arena.text("{"));

    if enum_def.variants.is_empty() {
        return header.append(arena.text("}"));
    }

    let variant_docs: Vec<_> = enum_def
        .variants
        .iter()
        .map(|v| arena.text(v.name.as_str()))
        .collect();

    let variants = arena
        .hardline()
        .append(arena.intersperse(variant_docs, arena.hardline()))
        .nest(2)
        .append(arena.hardline());

    header.append(variants).append(arena.text("}"))
}

fn pretty_print_client_doc<'a>(arena: &'a RcAllocator, client: &'a Client) -> DocBuilder<'a, RcAllocator> {
    arena
        .text("client")
        .append(arena.space())
        .append(arena.text(client.name.as_str()))
        .append(arena.space())
        .append(arena.text("{"))
        .append(
            arena
                .hardline()
                .append(arena.text("provider"))
                .append(arena.space())
                .append(arena.text(client.provider.as_str()))
                .nest(2),
        )
        .append(arena.hardline())
        .append(arena.text("}"))
}

fn pretty_print_type_ref<'a>(arena: &'a RcAllocator, type_ref: &'a TypeRef) -> DocBuilder<'a, RcAllocator> {
    match type_ref {
        TypeRef::Int => arena.text("int"),
        TypeRef::Float => arena.text("float"),
        TypeRef::String => arena.text("string"),
        TypeRef::Bool => arena.text("bool"),
        TypeRef::Null => arena.text("null"),
        TypeRef::Image => arena.text("image"),
        TypeRef::Audio => arena.text("audio"),
        TypeRef::Video => arena.text("video"),
        TypeRef::Pdf => arena.text("pdf"),
        TypeRef::Path(path) => {
            let parts: Vec<_> = path.segments.iter().map(|s| arena.text(s.as_str())).collect();
            arena.intersperse(parts, arena.text("."))
        }
        TypeRef::Optional(inner) => {
            pretty_print_type_ref(arena, inner).append(arena.text("?"))
        }
        TypeRef::List(inner) => pretty_print_type_ref(arena, inner).append(arena.text("[]")),
        TypeRef::Map { key, value } => arena
            .text("map<")
            .append(pretty_print_type_ref(arena, key))
            .append(arena.text(","))
            .append(arena.space())
            .append(pretty_print_type_ref(arena, value))
            .append(arena.text(">")),
        TypeRef::Union(types) => {
            let type_docs: Vec<_> = types
                .iter()
                .map(|t| pretty_print_type_ref(arena, t))
                .collect();
            arena.intersperse(type_docs, arena.space().append(arena.text("|")).append(arena.space()))
        }
        TypeRef::StringLiteral(s) => arena.text("\"").append(arena.text(s)).append(arena.text("\"")),
        TypeRef::IntLiteral(i) => arena.text(i.to_string()),
        TypeRef::FloatLiteral(f) => arena.text(f),
        TypeRef::Generic { base, args } => {
            let arg_docs: Vec<_> = args
                .iter()
                .map(|t| pretty_print_type_ref(arena, t))
                .collect();
            pretty_print_type_ref(arena, base)
                .append(arena.text("<"))
                .append(arena.intersperse(arg_docs, arena.text(",").append(arena.space())))
                .append(arena.text(">"))
        }
        TypeRef::TypeParam(name) => arena.text(name.as_str()),
        TypeRef::Error => arena.text("<error>"),
        TypeRef::Unknown => arena.text("<unknown>"),
    }
}

fn pretty_print_llm_body<'a>(arena: &'a RcAllocator, body: &'a LlmBody) -> DocBuilder<'a, RcAllocator> {
    let mut parts = Vec::new();

    if let Some(client) = &body.client {
        parts.push(
            arena
                .text("client")
                .append(arena.space())
                .append(arena.text(client.as_str())),
        );
    }

    if let Some(prompt) = &body.prompt {
        parts.push(
            arena
                .text("prompt")
                .append(arena.space())
                .append(arena.text("#\""))
                .append(arena.text(&prompt.text))
                .append(arena.text("\"#")),
        );
    }

    if parts.is_empty() {
        return arena.text("{}");
    }

    arena
        .text("{")
        .append(
            arena
                .hardline()
                .append(arena.intersperse(parts, arena.hardline()))
                .nest(2),
        )
        .append(arena.hardline())
        .append(arena.text("}"))
}

fn pretty_print_expr_body<'a>(arena: &'a RcAllocator, body: &'a ExprBody) -> DocBuilder<'a, RcAllocator> {
    if let Some(root_expr) = body.root_expr {
        pretty_print_expr(arena, root_expr, body)
    } else {
        arena.text("{}")
    }
}

fn pretty_print_expr<'a>(
    arena: &'a RcAllocator,
    expr_id: ExprId,
    body: &'a ExprBody,
) -> DocBuilder<'a, RcAllocator> {
    let expr = &body.exprs[expr_id];
    match expr {
        Expr::Literal(lit) => pretty_print_literal(arena, lit),
        Expr::Path(name) => arena.text(name.as_str()),
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let if_part = arena
                .text("if")
                .append(arena.space())
                .append(pretty_print_expr(arena, *condition, body))
                .append(arena.space())
                .append(pretty_print_expr(arena, *then_branch, body));

            if let Some(else_expr) = else_branch {
                if_part
                    .append(arena.space())
                    .append(arena.text("else"))
                    .append(arena.space())
                    .append(pretty_print_expr(arena, *else_expr, body))
            } else {
                if_part
            }
        }
        Expr::Match { scrutinee, arms } => {
            let match_header = arena
                .text("match")
                .append(arena.space())
                .append(pretty_print_expr(arena, *scrutinee, body))
                .append(arena.space())
                .append(arena.text("{"));

            if arms.is_empty() {
                return match_header.append(arena.text("}"));
            }

            let arm_docs: Vec<_> = arms
                .iter()
                .map(|arm| {
                    pretty_print_pattern(arena, arm.pattern, body)
                        .append(arena.space())
                        .append(arena.text("=>"))
                        .append(arena.space())
                        .append(pretty_print_expr(arena, arm.expr, body))
                })
                .collect();

            match_header
                .append(
                    arena
                        .hardline()
                        .append(arena.intersperse(
                            arm_docs,
                            arena.text(",").append(arena.hardline()),
                        ))
                        .nest(2),
                )
                .append(arena.hardline())
                .append(arena.text("}"))
        }
        Expr::Binary { op, lhs, rhs } => {
            let op_str = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
                BinaryOp::BitAnd => "&",
                BinaryOp::BitOr => "|",
                BinaryOp::BitXor => "^",
                BinaryOp::Shl => "<<",
                BinaryOp::Shr => ">>",
            };

            pretty_print_expr(arena, *lhs, body)
                .append(arena.space())
                .append(arena.text(op_str))
                .append(arena.space())
                .append(pretty_print_expr(arena, *rhs, body))
        }
        Expr::Unary { op, expr } => {
            let op_str = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            };
            arena
                .text(op_str)
                .append(pretty_print_expr(arena, *expr, body))
        }
        Expr::Call { callee, args } => {
            let arg_docs: Vec<_> = args
                .iter()
                .map(|arg| pretty_print_expr(arena, *arg, body))
                .collect();

            pretty_print_expr(arena, *callee, body)
                .append(arena.text("("))
                .append(arena.intersperse(arg_docs, arena.text(",").append(arena.space())))
                .append(arena.text(")"))
        }
        Expr::Object { type_name, fields } => {
            let header = if let Some(name) = type_name {
                arena
                    .text(name.as_str())
                    .append(arena.space())
                    .append(arena.text("{"))
            } else {
                arena.text("{")
            };

            if fields.is_empty() {
                return header.append(arena.text("}"));
            }

            let field_docs: Vec<_> = fields
                .iter()
                .map(|(name, expr)| {
                    arena
                        .text(name.as_str())
                        .append(arena.text(":"))
                        .append(arena.space())
                        .append(pretty_print_expr(arena, *expr, body))
                })
                .collect();

            header
                .append(arena.space())
                .append(arena.intersperse(field_docs, arena.text(",").append(arena.space())))
                .append(arena.space())
                .append(arena.text("}"))
        }
        Expr::Array { elements } => {
            let elem_docs: Vec<_> = elements
                .iter()
                .map(|e| pretty_print_expr(arena, *e, body))
                .collect();

            arena
                .text("[")
                .append(arena.intersperse(elem_docs, arena.text(",").append(arena.space())))
                .append(arena.text("]"))
        }
        Expr::Block { stmts: stmt_ids, tail_expr } => {
            let mut parts = Vec::new();

            for stmt_id in stmt_ids {
                parts.push(pretty_print_stmt(arena, *stmt_id, body));
            }

            if let Some(tail) = tail_expr {
                parts.push(pretty_print_expr(arena, *tail, body));
            }

            if parts.is_empty() {
                return arena.text("{}");
            }

            arena
                .text("{")
                .append(
                    arena
                        .hardline()
                        .append(arena.intersperse(parts, arena.hardline()))
                        .nest(2),
                )
                .append(arena.hardline())
                .append(arena.text("}"))
        }
        Expr::FieldAccess { base, field } => pretty_print_expr(arena, *base, body)
            .append(arena.text("."))
            .append(arena.text(field.as_str())),
        Expr::Index { base, index } => pretty_print_expr(arena, *base, body)
            .append(arena.text("["))
            .append(pretty_print_expr(arena, *index, body))
            .append(arena.text("]")),
        Expr::Missing => arena.text("<missing>"),
    }
}

fn pretty_print_stmt<'a>(
    arena: &'a RcAllocator,
    stmt_id: StmtId,
    body: &'a ExprBody,
) -> DocBuilder<'a, RcAllocator> {
    let stmt = &body.stmts[stmt_id];
    match stmt {
        Stmt::Expr(expr_id) => pretty_print_expr(arena, *expr_id, body)
            .append(arena.text(";")),
        Stmt::Let {
            pattern,
            type_annotation,
            initializer,
        } => {
            let mut doc = arena
                .text("let")
                .append(arena.space())
                .append(pretty_print_pattern(arena, *pattern, body));

            if let Some(ty) = type_annotation {
                doc = doc
                    .append(arena.text(":"))
                    .append(arena.space())
                    .append(pretty_print_type_ref(arena, ty));
            }

            if let Some(init) = initializer {
                doc = doc
                    .append(arena.space())
                    .append(arena.text("="))
                    .append(arena.space())
                    .append(pretty_print_expr(arena, *init, body));
            }

            doc.append(arena.text(";"))
        }
        Stmt::Return(expr_opt) => {
            let mut doc = arena.text("return");
            if let Some(expr_id) = expr_opt {
                doc = doc
                    .append(arena.space())
                    .append(pretty_print_expr(arena, *expr_id, body));
            }
            doc.append(arena.text(";"))
        }
        Stmt::Missing => arena.text("<missing>;"),
    }
}

fn pretty_print_pattern<'a>(
    arena: &'a RcAllocator,
    pat_id: PatId,
    body: &'a ExprBody,
) -> DocBuilder<'a, RcAllocator> {
    let pattern = &body.patterns[pat_id];
    match pattern {
        Pattern::Literal(lit) => pretty_print_literal(arena, lit),
        Pattern::Path(name) => arena.text(name.as_str()),
        Pattern::Binding(name) => arena.text(name.as_str()),
        Pattern::Wildcard => arena.text("_"),
    }
}

fn pretty_print_literal<'a>(arena: &'a RcAllocator, lit: &'a Literal) -> DocBuilder<'a, RcAllocator> {
    match lit {
        Literal::String(s) => arena.text("\"").append(arena.text(s)).append(arena.text("\"")),
        Literal::Int(i) => arena.text(i.to_string()),
        Literal::Float(f) => arena.text(f),
        Literal::Bool(b) => arena.text(if *b { "true" } else { "false" }),
        Literal::Null => arena.text("null"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_tree::{Class, Enum, EnumVariant, Field, Function, ItemTree};
    use crate::signature::FunctionSignature;
    use crate::type_ref::TypeRef;
    use baml_base::Name;

    #[test]
    fn test_pretty_print_item_tree() {
        let mut tree = ItemTree::new();

        // Add a function
        let func_id = tree.alloc_function(Function {
            name: Name::new("myFunc"),
        });

        // Add a class
        tree.alloc_class(Class {
            name: Name::new("MyClass"),
            fields: vec![
                Field {
                    name: Name::new("field1"),
                    type_ref: TypeRef::String,
                },
                Field {
                    name: Name::new("field2"),
                    type_ref: TypeRef::Int,
                },
            ],
            is_dynamic: false,
        });

        // Add an enum
        tree.alloc_enum(Enum {
            name: Name::new("MyEnum"),
            variants: vec![
                EnumVariant {
                    name: Name::new("VARIANT_A"),
                },
                EnumVariant {
                    name: Name::new("VARIANT_B"),
                },
            ],
        });

        let output = pretty_print_item_tree(&tree, 80, |id| {
            if id == func_id {
                Some((
                    FunctionSignature {
                        name: Name::new("myFunc"),
                        params: vec![],
                        return_type: TypeRef::String,
                        attrs: Default::default(),
                    },
                    FunctionBody::Missing,
                ))
            } else {
                None
            }
        });
        
        // Verify output contains expected strings
        assert!(output.contains("function myFunc() -> string { /* missing */ }"));
        assert!(output.contains("class MyClass"));
        assert!(output.contains("field1: string"));
        assert!(output.contains("enum MyEnum"));
        assert!(output.contains("VARIANT_A"));
    }
}
