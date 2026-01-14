//! Compilation from salsa database to BamlProgram.
//!
//! This crate provides the bridge between the compiler (salsa-based) and
//! the serializable BamlProgram representation.

use std::collections::HashMap;

use baml_compiler_hir::{
    self, Expr, ExprBody, FunctionBody as HirFunctionBody, ItemId, Literal, file_item_tree,
};
use baml_compiler_tir::{Ty as TirTy, TypeResolutionContext};
use baml_db::baml_workspace::Project;
use baml_program::{
    BamlMap, BamlProgram, BamlValue, ClassDef, ClientDef, EnumDef, EnumVariantDef, FieldDef,
    FunctionBody, FunctionDef, LiteralValue, MediaKind, ParamDef, TestArgs, TestCaseDef, Ty,
};
use baml_project::ProjectDatabase;

/// Compile a BamlProgram from a salsa database.
///
/// This extracts all definitions from the database and creates a
/// serializable snapshot.
pub fn compile_program(db: &ProjectDatabase, project: Project) -> BamlProgram {
    let mut program = BamlProgram::new();
    let items = baml_compiler_hir::project_items(db, project);
    let resolution_ctx = TypeResolutionContext::new(db, project);

    for item in items.items(db) {
        match item {
            ItemId::Class(class_loc) => {
                if let Some(class_def) = compile_class(db, project, *class_loc, &resolution_ctx) {
                    program.classes.insert(class_def.name.clone(), class_def);
                }
            }
            ItemId::Enum(enum_loc) => {
                if let Some(enum_def) = compile_enum(db, *enum_loc) {
                    program.enums.insert(enum_def.name.clone(), enum_def);
                }
            }
            ItemId::Function(func_loc) => {
                if let Some(func_def) = compile_function(db, project, *func_loc, &resolution_ctx) {
                    program.functions.insert(func_def.name.clone(), func_def);
                }
            }
            ItemId::Client(client_loc) => {
                if let Some(client_def) = compile_client(db, *client_loc) {
                    program.clients.insert(client_def.name.clone(), client_def);
                }
            }
            ItemId::Test(test_loc) => {
                if let Some(test_def) = compile_test(db, *test_loc) {
                    program.tests.insert(test_def.name.clone(), test_def);
                }
            }
            _ => {}
        }
    }

    program
}

/// Convert TIR type to BamlProgram type.
///
/// This is the bridge between compiler types and runtime types.
pub fn convert_tir_ty(ty: &TirTy) -> Ty {
    match ty {
        TirTy::Int => Ty::Int,
        TirTy::Float => Ty::Float,
        TirTy::String => Ty::String,
        TirTy::Bool => Ty::Bool,
        TirTy::Null => Ty::Null,
        TirTy::Media(kind) => Ty::Media(convert_media_kind(kind)),
        TirTy::Named(name) | TirTy::Class(name) => Ty::Class(name.to_string()),
        TirTy::Enum(name) => Ty::Enum(name.to_string()),
        TirTy::Optional(inner) => Ty::Optional(Box::new(convert_tir_ty(inner))),
        TirTy::List(inner) => Ty::List(Box::new(convert_tir_ty(inner))),
        TirTy::Map { key, value } => Ty::Map {
            key: Box::new(convert_tir_ty(key)),
            value: Box::new(convert_tir_ty(value)),
        },
        TirTy::Union(variants) => Ty::Union(variants.iter().map(convert_tir_ty).collect()),
        TirTy::Literal(lit) => Ty::Literal(convert_literal(lit)),
        TirTy::Unknown | TirTy::Error | TirTy::Void => Ty::String, // Fallback
        TirTy::Function { .. } => Ty::String, // Functions not supported as types
        TirTy::WatchAccessor(inner) => convert_tir_ty(inner),
    }
}

fn compile_class(
    db: &ProjectDatabase,
    _project: Project,
    class_loc: baml_compiler_hir::ClassLoc<'_>,
    resolution_ctx: &TypeResolutionContext,
) -> Option<ClassDef> {
    let file = class_loc.file(db);
    let item_tree = file_item_tree(db, file);
    let class = &item_tree[class_loc.id(db)];

    let fields = baml_compiler_hir::class_fields(db, class_loc);
    let compiled_fields: Vec<FieldDef> = fields
        .fields(db)
        .iter()
        .map(|(name, type_ref)| {
            let (ty, _errors) = resolution_ctx.lower_type_ref(type_ref, Default::default());
            FieldDef {
                name: name.to_string(),
                field_type: convert_tir_ty(&ty),
                description: None, // TODO: Extract from attributes
                alias: None,       // TODO: Extract from attributes
            }
        })
        .collect();

    Some(ClassDef {
        name: class.name.to_string(),
        fields: compiled_fields,
        description: None, // TODO: Extract from doc comments
    })
}

fn compile_enum(db: &ProjectDatabase, enum_loc: baml_compiler_hir::EnumLoc<'_>) -> Option<EnumDef> {
    let file = enum_loc.file(db);
    let item_tree = file_item_tree(db, file);
    let enum_def = &item_tree[enum_loc.id(db)];

    let variants: Vec<EnumVariantDef> = enum_def
        .variants
        .iter()
        .map(|v| EnumVariantDef {
            name: v.name.to_string(),
            description: None, // TODO: Extract from attributes
            alias: None,       // TODO: Extract from attributes
        })
        .collect();

    Some(EnumDef {
        name: enum_def.name.to_string(),
        variants,
        description: None,
    })
}

fn compile_function(
    db: &ProjectDatabase,
    _project: Project,
    func_loc: baml_compiler_hir::FunctionLoc<'_>,
    resolution_ctx: &TypeResolutionContext,
) -> Option<FunctionDef> {
    let file = func_loc.file(db);
    let item_tree = file_item_tree(db, file);
    let func = &item_tree[func_loc.id(db)];

    let signature = baml_compiler_hir::function_signature(db, func_loc);
    let body = baml_compiler_hir::function_body(db, func_loc);

    let params: Vec<ParamDef> = signature
        .params
        .iter()
        .map(|p| {
            let (ty, _errors) = resolution_ctx.lower_type_ref(&p.type_ref, Default::default());
            ParamDef {
                name: p.name.to_string(),
                param_type: convert_tir_ty(&ty),
            }
        })
        .collect();

    let (return_ty, _errors) =
        resolution_ctx.lower_type_ref(&signature.return_type, Default::default());

    let compiled_body = match body.as_ref() {
        HirFunctionBody::Llm(llm) => FunctionBody::Llm {
            prompt_template: llm
                .prompt
                .as_ref()
                .map(|p| p.text.clone())
                .unwrap_or_default(),
            client: llm
                .client
                .clone()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "default".to_string()),
        },
        HirFunctionBody::Expr(_) => FunctionBody::Expr { bytecode_index: 0 }, // TODO: Compile bytecode
        HirFunctionBody::Missing => FunctionBody::Missing,
    };

    Some(FunctionDef {
        name: func.name.to_string(),
        params,
        return_type: convert_tir_ty(&return_ty),
        body: compiled_body,
    })
}

fn compile_client(
    db: &ProjectDatabase,
    client_loc: baml_compiler_hir::ClientLoc<'_>,
) -> Option<ClientDef> {
    let file = client_loc.file(db);
    let item_tree = file_item_tree(db, file);
    let client = &item_tree[client_loc.id(db)];

    // TODO: Extract provider and options from client definition
    Some(ClientDef {
        name: client.name.to_string(),
        provider: "openai".to_string(), // TODO: Extract from client
        options: HashMap::new(),        // TODO: Extract from client
        retry_policy: None,             // TODO: Extract from client
    })
}

fn compile_test(
    db: &ProjectDatabase,
    test_loc: baml_compiler_hir::TestLoc<'_>,
) -> Option<TestCaseDef> {
    let file = test_loc.file(db);
    let item_tree = file_item_tree(db, file);
    let test = &item_tree[test_loc.id(db)];

    // Extract function name (first function reference)
    let function = test
        .function_refs
        .first()
        .map(|n| n.to_string())
        .unwrap_or_default();

    // Convert ExprBody args to BamlValue
    let args = convert_expr_body_to_args(&test.args);

    Some(TestCaseDef {
        name: test.name.to_string(),
        function,
        args: TestArgs::Literal(args),
    })
}

/// Convert ExprBody (HIR expression tree) to test args map.
///
/// The ExprBody should have a root Expr::Map expression containing
/// the test arguments as key-value pairs.
fn convert_expr_body_to_args(body: &ExprBody) -> HashMap<String, BamlValue> {
    let Some(root_id) = body.root_expr else {
        return HashMap::new();
    };

    let root_expr = &body.exprs[root_id];

    // The root should be an Expr::Map
    if let Expr::Map { entries } = root_expr {
        let mut args = HashMap::new();

        for (key_id, value_id) in entries {
            // Key should be a string literal
            let key = match &body.exprs[*key_id] {
                Expr::Literal(Literal::String(s)) => s.clone(),
                _ => continue, // Skip non-string keys
            };

            // Convert value expression to BamlValue
            let value = convert_expr_to_baml_value(body, *value_id);
            args.insert(key, value);
        }

        args
    } else {
        HashMap::new()
    }
}

/// Convert a single expression to BamlValue.
fn convert_expr_to_baml_value(body: &ExprBody, expr_id: baml_compiler_hir::ExprId) -> BamlValue {
    let expr = &body.exprs[expr_id];

    match expr {
        Expr::Literal(lit) => match lit {
            Literal::String(s) => BamlValue::String(s.clone()),
            Literal::Int(i) => BamlValue::Int(*i),
            Literal::Float(f) => {
                // Parse float string to f64
                f.parse::<f64>()
                    .map(BamlValue::Float)
                    .unwrap_or(BamlValue::Null)
            }
            Literal::Bool(b) => BamlValue::Bool(*b),
            Literal::Null => BamlValue::Null,
        },
        Expr::Array { elements } => {
            let items: Vec<BamlValue> = elements
                .iter()
                .map(|e| convert_expr_to_baml_value(body, *e))
                .collect();
            BamlValue::List(items)
        }
        Expr::Map { entries } => {
            let map: BamlMap = entries
                .iter()
                .filter_map(|(key_id, value_id)| {
                    // Key should be a string literal
                    let key = match &body.exprs[*key_id] {
                        Expr::Literal(Literal::String(s)) => s.clone(),
                        _ => return None,
                    };
                    let value = convert_expr_to_baml_value(body, *value_id);
                    Some((key, value))
                })
                .collect();
            BamlValue::Map(map)
        }
        Expr::Object { fields, .. } => {
            // Convert object to map
            let map: BamlMap = fields
                .iter()
                .map(|(name, value_id)| {
                    let value = convert_expr_to_baml_value(body, *value_id);
                    (name.to_string(), value)
                })
                .collect();
            BamlValue::Map(map)
        }
        Expr::Path(path) => {
            // For simple paths (variable references), convert to string
            // This handles enum values and identifiers
            if path.len() == 1 {
                BamlValue::String(path[0].to_string())
            } else {
                // For multi-segment paths like Status.Active, join with "."
                let path_str = path
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                BamlValue::String(path_str)
            }
        }
        // For other expression types, return null (these should be compiled to bytecode
        // in a full implementation)
        _ => BamlValue::Null,
    }
}

fn convert_media_kind(kind: &baml_base::MediaKind) -> MediaKind {
    match kind {
        baml_base::MediaKind::Image => MediaKind::Image,
        baml_base::MediaKind::Audio => MediaKind::Audio,
        baml_base::MediaKind::Video => MediaKind::Video,
        baml_base::MediaKind::Pdf => MediaKind::Pdf,
        baml_base::MediaKind::Generic => MediaKind::Image, // Fallback
    }
}

fn convert_literal(lit: &baml_compiler_tir::LiteralValue) -> LiteralValue {
    match lit {
        baml_compiler_tir::LiteralValue::String(s) => LiteralValue::String(s.to_string()),
        baml_compiler_tir::LiteralValue::Int(i) => LiteralValue::Int(*i),
        baml_compiler_tir::LiteralValue::Bool(b) => LiteralValue::Bool(*b),
        baml_compiler_tir::LiteralValue::Float(f) => {
            // Convert float string to int if possible, otherwise store as string literal
            if let Ok(i) = f.parse::<i64>() {
                LiteralValue::Int(i)
            } else {
                LiteralValue::String(f.clone())
            }
        }
    }
}
