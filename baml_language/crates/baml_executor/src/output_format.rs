//! Build OutputFormatContent from BamlProgram.
//!
//! This module provides the bridge between the serializable BamlProgram
//! and the OutputFormatContent used for rendering output schemas.

use std::collections::HashSet;

use baml_output_format::{
    Class, Enum, EnumVariant, Name, OutputFormatBuilder, OutputFormatContent,
};
use baml_program::{BamlProgram, ClassDef, EnumDef};

/// Build OutputFormatContent for a given return type.
///
/// This extracts all classes and enums reachable from the return type
/// and converts them to the output format representation.
pub fn build_output_format(
    program: &BamlProgram,
    return_ty: &baml_program::Ty,
) -> OutputFormatContent {
    let mut builder = OutputFormatBuilder::new();
    let mut visited_classes = HashSet::new();
    let mut visited_enums = HashSet::new();
    let mut recursive_classes = HashSet::new();

    // Collect all types reachable from return type
    collect_types(
        return_ty,
        program,
        &mut visited_classes,
        &mut visited_enums,
        &mut recursive_classes,
        &mut HashSet::new(),
    );

    // Add classes
    for class_name in &visited_classes {
        if let Some(class_def) = program.classes.get(class_name) {
            builder = builder.with_class(convert_class(class_def));
        }
    }

    // Add enums
    for enum_name in &visited_enums {
        if let Some(enum_def) = program.enums.get(enum_name) {
            builder = builder.with_enum(convert_enum(enum_def));
        }
    }

    // Mark recursive classes
    for class_name in &recursive_classes {
        builder = builder.with_recursive_class(class_name.clone());
    }

    // Set target type
    builder = builder.with_target(convert_ty(return_ty));

    builder.build()
}

/// Collect all types reachable from a given type.
fn collect_types(
    ty: &baml_program::Ty,
    program: &BamlProgram,
    visited_classes: &mut HashSet<String>,
    visited_enums: &mut HashSet<String>,
    recursive_classes: &mut HashSet<String>,
    in_progress: &mut HashSet<String>,
) {
    match ty {
        baml_program::Ty::Class(name) => {
            if in_progress.contains(name) {
                // This is a recursive reference
                recursive_classes.insert(name.clone());
                return;
            }
            if visited_classes.contains(name) {
                return;
            }
            visited_classes.insert(name.clone());
            in_progress.insert(name.clone());

            if let Some(class_def) = program.classes.get(name) {
                for field in &class_def.fields {
                    collect_types(
                        &field.field_type,
                        program,
                        visited_classes,
                        visited_enums,
                        recursive_classes,
                        in_progress,
                    );
                }
            }

            in_progress.remove(name);
        }
        baml_program::Ty::Enum(name) => {
            visited_enums.insert(name.clone());
        }
        baml_program::Ty::Optional(inner) => {
            collect_types(
                inner,
                program,
                visited_classes,
                visited_enums,
                recursive_classes,
                in_progress,
            );
        }
        baml_program::Ty::List(inner) => {
            collect_types(
                inner,
                program,
                visited_classes,
                visited_enums,
                recursive_classes,
                in_progress,
            );
        }
        baml_program::Ty::Map { key, value } => {
            collect_types(
                key,
                program,
                visited_classes,
                visited_enums,
                recursive_classes,
                in_progress,
            );
            collect_types(
                value,
                program,
                visited_classes,
                visited_enums,
                recursive_classes,
                in_progress,
            );
        }
        baml_program::Ty::Union(variants) => {
            for variant in variants {
                collect_types(
                    variant,
                    program,
                    visited_classes,
                    visited_enums,
                    recursive_classes,
                    in_progress,
                );
            }
        }
        // Primitives and media types don't reference other types
        _ => {}
    }
}

/// Convert baml_program::ClassDef to baml_output_format::Class.
fn convert_class(class_def: &ClassDef) -> Class {
    let mut class = Class::new(&class_def.name);

    if let Some(desc) = &class_def.description {
        class = class.with_description(desc.clone());
    }

    for field in &class_def.fields {
        // Use alias as the rendered name if present
        let field_name = field.alias.as_ref().unwrap_or(&field.name);
        class = class.with_field(
            field_name.clone(),
            convert_ty(&field.field_type),
            field.description.clone(),
            !is_optional(&field.field_type),
        );
    }

    class
}

/// Check if a type is optional.
fn is_optional(ty: &baml_program::Ty) -> bool {
    matches!(ty, baml_program::Ty::Optional(_))
}

/// Convert baml_program::EnumDef to baml_output_format::Enum.
fn convert_enum(enum_def: &EnumDef) -> Enum {
    let mut e = Enum::new(&enum_def.name);

    for variant in &enum_def.variants {
        let name = if let Some(alias) = &variant.alias {
            Name::with_alias(&variant.name, alias.clone())
        } else {
            Name::new(&variant.name)
        };

        e.variants.push(EnumVariant {
            name,
            description: variant.description.clone(),
        });
    }

    e
}

/// Convert baml_program::Ty to baml_base::Ty.
pub fn convert_ty(ty: &baml_program::Ty) -> baml_base::Ty {
    match ty {
        baml_program::Ty::Int => baml_base::Ty::Int,
        baml_program::Ty::Float => baml_base::Ty::Float,
        baml_program::Ty::String => baml_base::Ty::String,
        baml_program::Ty::Bool => baml_base::Ty::Bool,
        baml_program::Ty::Null => baml_base::Ty::Null,

        baml_program::Ty::Media(kind) => match kind {
            baml_program::MediaKind::Image => baml_base::Ty::Image,
            baml_program::MediaKind::Audio => baml_base::Ty::Audio,
            baml_program::MediaKind::Video => baml_base::Ty::Video,
            baml_program::MediaKind::Pdf => baml_base::Ty::Pdf,
        },

        baml_program::Ty::Literal(lit) => {
            let base_lit = match lit {
                baml_program::LiteralValue::Int(i) => baml_base::LiteralValue::Int(*i),
                baml_program::LiteralValue::Bool(b) => baml_base::LiteralValue::Bool(*b),
                baml_program::LiteralValue::String(s) => baml_base::LiteralValue::String(s.clone()),
            };
            baml_base::Ty::Literal(base_lit)
        }

        baml_program::Ty::Class(name) => baml_base::Ty::Class(name.clone().into()),
        baml_program::Ty::Enum(name) => baml_base::Ty::Enum(name.clone().into()),

        baml_program::Ty::Optional(inner) => baml_base::Ty::Optional(Box::new(convert_ty(inner))),
        baml_program::Ty::List(inner) => baml_base::Ty::List(Box::new(convert_ty(inner))),
        baml_program::Ty::Map { key, value } => baml_base::Ty::Map {
            key: Box::new(convert_ty(key)),
            value: Box::new(convert_ty(value)),
        },
        baml_program::Ty::Union(variants) => {
            baml_base::Ty::Union(variants.iter().map(convert_ty).collect())
        }
    }
}

#[cfg(test)]
mod tests {

    use baml_program::{ClassDef, EnumDef, EnumVariantDef, FieldDef};

    use super::*;

    fn make_program() -> BamlProgram {
        let mut program = BamlProgram::new();

        // Add Person class
        program.classes.insert(
            "Person".to_string(),
            ClassDef {
                name: "Person".to_string(),
                fields: vec![
                    FieldDef {
                        name: "name".to_string(),
                        field_type: baml_program::Ty::String,
                        description: Some("The person's name".to_string()),
                        alias: None,
                    },
                    FieldDef {
                        name: "age".to_string(),
                        field_type: baml_program::Ty::Int,
                        description: None,
                        alias: None,
                    },
                ],
                description: Some("A person".to_string()),
            },
        );

        // Add Status enum
        program.enums.insert(
            "Status".to_string(),
            EnumDef {
                name: "Status".to_string(),
                variants: vec![
                    EnumVariantDef {
                        name: "Active".to_string(),
                        description: None,
                        alias: None,
                    },
                    EnumVariantDef {
                        name: "Inactive".to_string(),
                        description: None,
                        alias: None,
                    },
                ],
                description: None,
            },
        );

        program
    }

    #[test]
    fn test_build_output_format_class() {
        let program = make_program();
        let return_ty = baml_program::Ty::Class("Person".to_string());

        let output_format = build_output_format(&program, &return_ty);

        assert!(output_format.find_class("Person").is_some());
        let person = output_format.find_class("Person").unwrap();
        assert_eq!(person.fields.len(), 2);
        assert_eq!(person.fields[0].name.real_name(), "name");
    }

    #[test]
    fn test_build_output_format_enum() {
        let program = make_program();
        let return_ty = baml_program::Ty::Enum("Status".to_string());

        let output_format = build_output_format(&program, &return_ty);

        assert!(output_format.find_enum("Status").is_some());
        let status = output_format.find_enum("Status").unwrap();
        assert_eq!(status.variants.len(), 2);
    }

    #[test]
    fn test_build_output_format_primitive() {
        let program = make_program();
        let return_ty = baml_program::Ty::String;

        let output_format = build_output_format(&program, &return_ty);

        // Primitives don't need classes/enums
        assert!(output_format.classes.is_empty());
        assert!(output_format.enums.is_empty());
    }

    #[test]
    fn test_recursive_class_detection() {
        let mut program = BamlProgram::new();

        // Add Node class that references itself
        program.classes.insert(
            "Node".to_string(),
            ClassDef {
                name: "Node".to_string(),
                fields: vec![
                    FieldDef {
                        name: "value".to_string(),
                        field_type: baml_program::Ty::Int,
                        description: None,
                        alias: None,
                    },
                    FieldDef {
                        name: "next".to_string(),
                        field_type: baml_program::Ty::Optional(Box::new(baml_program::Ty::Class(
                            "Node".to_string(),
                        ))),
                        description: None,
                        alias: None,
                    },
                ],
                description: None,
            },
        );

        let return_ty = baml_program::Ty::Class("Node".to_string());
        let output_format = build_output_format(&program, &return_ty);

        assert!(output_format.recursive_classes.contains("Node"));
    }

    #[test]
    fn test_convert_ty_primitives() {
        assert_eq!(convert_ty(&baml_program::Ty::Int), baml_base::Ty::Int);
        assert_eq!(convert_ty(&baml_program::Ty::String), baml_base::Ty::String);
        assert_eq!(convert_ty(&baml_program::Ty::Bool), baml_base::Ty::Bool);
    }
}
