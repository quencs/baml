//! Output format builder.
//!
//! This module builds `OutputFormatContent` from TIR types.
//! It collects all referenced classes and enums and converts them to
//! the format needed by the Jinja runtime.

use std::collections::{HashMap, HashSet, VecDeque};

use baml_base::{Name, Ty};
use baml_db::baml_workspace::Project;
use baml_project::ProjectDatabase as RootDatabase;
use baml_jinja_runtime::{
    OutputFormatContent, OutputFormatBuilder, Class, Enum,
};
use baml_compiler_tir::{class_field_types, enum_variants};

/// Build OutputFormatContent from a TIR return type.
///
/// This collects all referenced classes and enums to build the complete
/// output format schema.
pub fn build_output_format(
    db: &RootDatabase,
    project: Project,
    return_ty: &Ty,
) -> OutputFormatContent {
    let mut builder = OutputFormatBuilder::new();

    // Set the target type directly (no conversion needed)
    builder = builder.with_target(return_ty.clone());

    // Get class fields from TIR (needed for recursive collection)
    let class_fields_map = class_field_types(db, project);
    let class_fields_ref = class_fields_map.classes(db);

    // Collect all referenced types (including nested ones)
    let (visited_classes, visited_enums) = collect_all_types(return_ty, class_fields_ref);

    // Get enum variants from TIR
    let enum_variants_map = enum_variants(db, project);
    let enum_variants_ref = enum_variants_map.enums(db);

    // Build enum definitions
    for enum_name in &visited_enums {
        if let Some(variants) = enum_variants_ref.get::<Name>(enum_name) {
            let mut e = Enum::new(enum_name.as_str());
            for variant_name in variants {
                // TODO: Add description support once HIR has it
                e = e.with_variant(variant_name.as_str(), None);
            }
            builder = builder.with_enum(e);
        }
    }

    // Build class definitions
    for class_name in &visited_classes {
        if let Some(fields) = class_fields_ref.get::<Name>(class_name) {
            let mut c = Class::new(class_name.as_str());
            for (field_name, field_ty) in fields {
                // TODO: Add description support once HIR has it
                c = c.with_field(
                    field_name.as_str(),
                    field_ty.clone(),
                    None,  // description
                    !field_ty.is_optional(),  // required if not optional
                );
            }
            builder = builder.with_class(c);
        }
    }

    builder.build()
}

/// Collect all class and enum names referenced in a type, including nested types.
///
/// This does a BFS traversal to find all types:
/// 1. Start with the return type
/// 2. For each class found, also look at its field types
/// 3. Continue until no new types are found
fn collect_all_types(
    return_ty: &Ty,
    class_fields: &HashMap<Name, HashMap<Name, Ty>>,
) -> (HashSet<Name>, HashSet<Name>) {
    let mut visited_classes = HashSet::new();
    let mut visited_enums = HashSet::new();

    // Initial collection from the return type
    let mut pending_classes: VecDeque<Name> = VecDeque::new();

    // First pass: collect direct references from return type
    collect_types_from_ty(return_ty, &mut pending_classes, &mut visited_enums);

    // BFS to collect all nested class references
    while let Some(class_name) = pending_classes.pop_front() {
        // Skip if already visited
        if visited_classes.contains(&class_name) {
            continue;
        }
        visited_classes.insert(class_name.clone());

        // Look at fields of this class
        if let Some(fields) = class_fields.get::<Name>(&class_name) {
            for (_field_name, field_ty) in fields {
                collect_types_from_ty(field_ty, &mut pending_classes, &mut visited_enums);
            }
        }
    }

    (visited_classes, visited_enums)
}

/// Collect class and enum names from a single type expression.
/// Classes are added to pending_classes for further exploration.
/// Enums are added directly to visited_enums.
fn collect_types_from_ty(
    ty: &Ty,
    pending_classes: &mut VecDeque<Name>,
    visited_enums: &mut HashSet<Name>,
) {
    match ty {
        Ty::Class(name) => {
            pending_classes.push_back(name.clone());
        }
        Ty::Enum(name) => {
            visited_enums.insert(name.clone());
        }
        Ty::Named(name) => {
            // Named could be class or enum - add to both
            // The class will be skipped if not found in class_fields
            pending_classes.push_back(name.clone());
            visited_enums.insert(name.clone());
        }
        Ty::Optional(inner) => {
            collect_types_from_ty(inner, pending_classes, visited_enums);
        }
        Ty::List(inner) => {
            collect_types_from_ty(inner, pending_classes, visited_enums);
        }
        Ty::Map { key, value } => {
            collect_types_from_ty(key, pending_classes, visited_enums);
            collect_types_from_ty(value, pending_classes, visited_enums);
        }
        Ty::Union(variants) => {
            for variant in variants {
                collect_types_from_ty(variant, pending_classes, visited_enums);
            }
        }
        // Primitives and other types don't reference classes/enums
        Ty::Int | Ty::Float | Ty::String | Ty::Bool | Ty::Null
        | Ty::Image | Ty::Audio | Ty::Video | Ty::Pdf
        | Ty::Literal(_) | Ty::Unknown | Ty::Error | Ty::Void => {}
        Ty::Function { params, ret } => {
            for param in params {
                collect_types_from_ty(param, pending_classes, visited_enums);
            }
            collect_types_from_ty(ret, pending_classes, visited_enums);
        }
        Ty::WatchAccessor(inner) => {
            collect_types_from_ty(inner, pending_classes, visited_enums);
        }
    }
}
