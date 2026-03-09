//! Stream expansion logic and output types for compiler2.
//!
//! PPIR expansion computes per-field expansion data (`stream_type`, `sap_starts_as`,
//! `sap_in_progress_never`) and per-alias expanded bodies. The synthesize module
//! consumes these to build synthetic AST items.

use baml_base::Name;
use smol_str::SmolStr;

use crate::{
    PpirNames,
    normalize,
    ty::{PpirField, PpirTy, PpirTypeAttrs},
};

//
// ──────────────────────────────────────────────── OUTPUT TYPES ─────
//

/// SAP starts-as value, synthesized from `@stream.starts_as` / `@stream.not_null` / defaults.
/// This becomes the `@sap.class_completed_field_missing` and `@sap.class_in_progress_field_missing`
/// attribute values, computed as part of `@stream.*` desugaring.
///
/// In compiler2, the `Explicit` variant stores the raw text string from
/// `RawAttributeArg.value` instead of a GreenNode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PpirStreamStartsAs {
    /// Field is absent until it begins streaming.
    /// From `@stream.not_null`, `@@stream.done`, or default for literal/never `stream_types`.
    Never,
    /// Default value computed during PPIR expansion from `stream_type`'s syntactic category.
    /// null for scalars, `[]` for lists, `{}` for maps, never for literals.
    DefaultFor(PpirTy),
    /// Explicit `@stream.starts_as(<arg>)`.
    /// `text`: raw attribute value string.
    /// `typeof_s`: best-effort inferred type (Never if unrecognizable).
    Explicit { text: String, typeof_s: PpirTy },
}

impl PpirStreamStartsAs {
    /// Extract the type representation for union computation.
    /// Used as one side of `sap_starts_as_type | stream_type`.
    pub fn as_ty(&self) -> Option<PpirTy> {
        match self {
            PpirStreamStartsAs::Never => Some(PpirTy::Never {
                attrs: PpirTypeAttrs::default(),
            }),
            PpirStreamStartsAs::DefaultFor(ty) => Some(ty.clone()),
            PpirStreamStartsAs::Explicit { typeof_s, .. } => Some(typeof_s.clone()),
        }
    }
}

/// Per-class desugared results. Carries the original class name (NOT `stream_*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpirDesugaredClass {
    pub name: Name,
    pub fields: Vec<PpirDesugaredField>,
}

/// Per-field desugared results with synthesized `@sap.*` attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpirDesugaredField {
    pub name: Name,
    /// The during-streaming type — result of `stream_expand` on the field's type.
    pub stream_type: PpirTy,
    /// `@sap.in_progress(never)` — synthesized from `@stream.done`.
    pub sap_in_progress_never: bool,
    /// Synthesized from `@stream.starts_as` / `@stream.not_null` / defaults.
    /// Becomes `@sap.class_completed_field_missing` and `@sap.class_in_progress_field_missing`.
    pub sap_starts_as: PpirStreamStartsAs,
}

/// Per-alias desugared results. Carries the original alias name (NOT `stream_*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpirDesugaredTypeAlias {
    pub name: Name,
    /// The result of `stream_expand` on the alias body.
    pub expanded_body: PpirTy,
}

//
// ──────────────────────────────────────────── STREAM EXPAND ─────
//

/// Compute the stream-expanded type from a `PpirTy`.
///
/// Checks `PpirTypeAttrs` before recursing:
/// - `@stream.type(D)`: use D, don't recurse
/// - `@stream.done` (without `stream_type`): use T as-is (atomic)
/// - Otherwise: normal recursive expansion using name classification
pub fn stream_expand(ty: &PpirTy, names: &PpirNames) -> PpirTy {
    let attrs = ty.attrs();

    // Explicit @stream.type(D) — use D directly
    if let Some(d) = &attrs.stream_type {
        return (**d).clone();
    }

    // @stream.done without explicit type — type is atomic, keep as-is
    if attrs.stream_done {
        return ty.clone_without_attrs();
    }

    // Normal recursive expansion (inline name classification via PpirNames)
    match ty {
        PpirTy::Int { .. }
        | PpirTy::Float { .. }
        | PpirTy::String { .. }
        | PpirTy::Bool { .. } => ty.clone_without_attrs(),

        PpirTy::Null { .. } => PpirTy::Null {
            attrs: PpirTypeAttrs::default(),
        },
        PpirTy::Never { .. } => PpirTy::Never {
            attrs: PpirTypeAttrs::default(),
        },

        PpirTy::StringLiteral { .. } | PpirTy::IntLiteral { .. } | PpirTy::BoolLiteral { .. } => {
            ty.clone_without_attrs()
        }

        // Inline name classification: class/type_alias → stream_*, enum → unchanged
        PpirTy::Named { name, .. } => {
            if names.class_names.contains_key(name) || names.type_alias_names.contains(name) {
                PpirTy::Named {
                    name: SmolStr::new(format!("stream_{name}")),
                    attrs: PpirTypeAttrs::default(),
                }
            } else {
                // Enum or unknown — unchanged
                ty.clone_without_attrs()
            }
        }

        PpirTy::List { inner, .. } => PpirTy::List {
            inner: Box::new(stream_expand(inner, names)),
            attrs: PpirTypeAttrs::default(),
        },

        PpirTy::Map { key, value, .. } => PpirTy::Map {
            key: key.clone(),
            value: Box::new(stream_expand(value, names)),
            attrs: PpirTypeAttrs::default(),
        },

        PpirTy::Union { variants, .. } => PpirTy::Union {
            variants: variants.iter().map(|v| stream_expand(v, names)).collect(),
            attrs: PpirTypeAttrs::default(),
        },

        PpirTy::Optional { inner, .. } => PpirTy::Union {
            variants: vec![
                stream_expand(inner, names),
                PpirTy::Null {
                    attrs: PpirTypeAttrs::default(),
                },
            ],
            attrs: PpirTypeAttrs::default(),
        },

        _ => ty.clone_without_attrs(),
    }
}

//
// ──────────────────────────────────────── DEFAULT SAP STARTS-AS ─────
//

/// Compute the default starts-as value from a field's `stream_type`.
///
/// Per the stream-types spec:
/// - Literal types → never (absent until complete)
/// - Never → never
/// - List → empty list (`list<never>`)
/// - Map → empty map (`map<key, never>`)
/// - Everything else → null
pub fn default_sap_starts_as(stream_type: &PpirTy) -> PpirStreamStartsAs {
    match stream_type {
        PpirTy::StringLiteral { .. }
        | PpirTy::IntLiteral { .. }
        | PpirTy::BoolLiteral { .. }
        | PpirTy::Never { .. } => PpirStreamStartsAs::Never,

        PpirTy::List { .. } => PpirStreamStartsAs::DefaultFor(PpirTy::List {
            inner: Box::new(PpirTy::Never {
                attrs: PpirTypeAttrs::default(),
            }),
            attrs: PpirTypeAttrs::default(),
        }),

        PpirTy::Map { key, .. } => PpirStreamStartsAs::DefaultFor(PpirTy::Map {
            key: key.clone(),
            value: Box::new(PpirTy::Never {
                attrs: PpirTypeAttrs::default(),
            }),
            attrs: PpirTypeAttrs::default(),
        }),

        _ => PpirStreamStartsAs::DefaultFor(PpirTy::Null {
            attrs: PpirTypeAttrs::default(),
        }),
    }
}

//
// ──────────────────────────────────────── BUILDING PPIR FIELDS ─────
//

/// Build a `PpirField` from a compiler2 AST `FieldDef`.
///
/// Type-level annotations (`@stream.done`, `@stream.type`, `@stream.with_state`)
/// are captured by `PpirTy::from_type_expr()` on the field's type.
/// Field-level annotations (`@stream.starts_as`, `@stream.not_null`) are read
/// from the field's `RawAttribute` list.
pub fn build_ppir_field(field: &baml_compiler2_ast::FieldDef) -> PpirField {
    let type_expr = field
        .type_expr
        .as_ref()
        .map(|s| &s.expr)
        .unwrap_or(&baml_compiler2_ast::TypeExpr::Unknown);

    let ty = PpirTy::from_type_expr(type_expr, &field.attributes);

    let starts_as = field
        .attributes
        .iter()
        .find(|a| a.name.as_str() == "stream.starts_as")
        .and_then(|a| a.args.first())
        .map(|arg| arg.value.clone());

    let not_null = field
        .attributes
        .iter()
        .any(|a| a.name.as_str() == "stream.not_null");

    PpirField {
        name: field.name.clone(),
        ty,
        starts_as,
        not_null,
    }
}

//
// ──────────────────────────────────────── FIELD DESUGARING ─────
//

/// Desugar a single field's stream annotations into `PpirDesugaredField`.
///
/// Computes `stream_type` via `stream_expand`, synthesizes @sap.* attributes.
pub fn desugar_field(pf: &PpirField, names: &PpirNames) -> PpirDesugaredField {
    // 1. Compute stream_type via stream_expand (respects type-level attrs)
    let stream_type = stream_expand(&pf.ty, names);

    // 2. Synthesize @sap.in_progress from @stream.done
    let sap_in_progress_never = pf.ty.attrs().stream_done;

    // 3. Synthesize sap_starts_as from @stream.starts_as / @stream.not_null / defaults
    //    Priority order:
    //    1. Field-level @stream.not_null → Never
    //    2. Explicit @stream.starts_as(value) → Explicit(text)
    //    3. Type-level @@stream.not_null on referenced type → Never
    //    4. Default from stream_type
    let sap_starts_as = if pf.not_null {
        PpirStreamStartsAs::Never
    } else if let Some(text) = &pf.starts_as {
        let starts_as = normalize::parse_starts_as_value(text);
        let typeof_s = normalize::infer_typeof_s(&starts_as, &names.enum_names).unwrap_or(
            PpirTy::Never {
                attrs: PpirTypeAttrs::default(),
            },
        );
        PpirStreamStartsAs::Explicit {
            text: text.clone(),
            typeof_s,
        }
    } else if type_has_block_attr(&pf.ty, "stream.not_null", names) {
        PpirStreamStartsAs::Never
    } else {
        default_sap_starts_as(&stream_type)
    };

    PpirDesugaredField {
        name: pf.name.clone(),
        stream_type,
        sap_in_progress_never,
        sap_starts_as,
    }
}

/// Check if the field's top-level type references a class/enum that has
/// a specific @@stream.* block attribute.
///
/// Only matches bare named types (e.g., `Foo`). Does NOT match `Foo[]`, `Foo?`,
/// `Foo | Bar`, etc. — those use their own default `starts_as` behavior.
fn type_has_block_attr(ty: &PpirTy, attr: &str, names: &PpirNames) -> bool {
    let PpirTy::Named { name, .. } = ty else {
        return false;
    };
    let has_attr = |attrs: &Vec<Name>| attrs.iter().any(|a| a == attr);
    names
        .class_names
        .get(name.as_str())
        .is_some_and(has_attr)
        || names
            .enum_names
            .get(name.as_str())
            .is_some_and(has_attr)
}
