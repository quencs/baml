//! Schema Index
//!
//! Pre-computes lookups from TypeIR for fast parsing decisions.
//! Handles type resolution, recursion detection, and union key indexing.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use baml_types::{LiteralValue, StreamingMode, TypeIR, TypeValue};
use indexmap::IndexMap;
use internal_baml_jinja::types::OutputFormatContent;

use super::coerce::remove_accents;

/// Unique identifier for types in the schema
pub type TypeId = u32;

/// Pre-indexed schema for a type tree
pub struct SchemaIndex {
    /// TypeId counter
    next_id: TypeId,
    /// TypeIR hash -> TypeId mapping (for recursion detection)
    type_ids: HashMap<TypeKey, TypeId>,
    /// Per-type metadata
    pub type_info: HashMap<TypeId, TypeInfo>,
    /// Root type ID
    root_id: TypeId,
}

/// Key for deduplicating TypeIR (based on structural equality)
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct TypeKey(String);

impl TypeKey {
    fn from_type(ty: &TypeIR) -> Self {
        // Create a string key from the type structure
        TypeKey(format!("{:?}", ty))
    }
}

/// Type information for parsing
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub kind: TypeKind,
    pub id: TypeId,
    /// Original TypeIR for this type (preserves metadata)
    pub source_type: TypeIR,
}

/// Kinds of types with their specific metadata
#[derive(Debug, Clone)]
pub enum TypeKind {
    Primitive(PrimitiveKind),
    Enum {
        name: String,
        /// rendered_name -> real_name mapping
        values: HashMap<String, String>,
        /// lowercase/normalized -> real_name (for fuzzy matching)
        fuzzy_map: HashMap<String, String>,
    },
    Literal(LiteralKind),
    Class {
        name: String,
        /// field rendered_name -> FieldInfo
        fields: HashMap<String, FieldInfo>,
        /// Required field names (rendered names)
        required: HashSet<String>,
        /// Fuzzy field name mapping: lowercase/normalized -> rendered_name
        fuzzy_fields: HashMap<String, String>,
    },
    List {
        element: TypeId,
    },
    Map {
        key: TypeId,
        value: TypeId,
    },
    Union {
        variants: Vec<TypeId>,
        /// key -> which variant indices have this key (for narrowing)
        key_to_variants: HashMap<String, Vec<usize>>,
    },
    Optional {
        inner: TypeId,
    },
    Tuple {
        elements: Vec<TypeId>,
    },
    RecursiveAlias {
        name: String,
        target: Option<TypeId>,
    },
    Top,
}

/// Field information in a class
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// The rendered name (used for JSON key matching)
    pub name: String,
    /// The real name (used for output)
    pub real_name: String,
    pub type_id: TypeId,
    pub required: bool,
    /// Original TypeIR for this field
    pub source_type: TypeIR,
}

/// Primitive type kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    String,
    Int,
    Float,
    Bool,
    Null,
    Media,
}

/// Literal value kinds
#[derive(Debug, Clone)]
pub enum LiteralKind {
    String(String),
    Int(i64),
    Bool(bool),
}

impl TypeKind {
    /// Get enum name if this is an enum
    pub fn enum_name(&self) -> Option<&str> {
        match self {
            TypeKind::Enum { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Get class name if this is a class
    pub fn class_name(&self) -> Option<&str> {
        match self {
            TypeKind::Class { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// Context for building schema index
struct BuildContext<'a> {
    of: Option<&'a OutputFormatContent>,
}

impl SchemaIndex {
    /// Build index from root TypeIR with optional context
    pub fn build(root: &TypeIR, of: Option<&OutputFormatContent>) -> Self {
        let ctx = BuildContext { of };

        let mut index = SchemaIndex {
            next_id: 0,
            type_ids: HashMap::new(),
            type_info: HashMap::new(),
            root_id: 0,
        };
        index.root_id = index.index_type_with_context(root, &ctx);
        index
    }

    /// Get the root type ID
    pub fn root_id(&self) -> TypeId {
        self.root_id
    }

    /// Get type info by ID
    pub fn get(&self, id: TypeId) -> Option<&TypeInfo> {
        self.type_info.get(&id)
    }

    /// Index a type, returning its TypeId
    fn index_type_with_context(&mut self, ty: &TypeIR, ctx: &BuildContext) -> TypeId {
        let key = TypeKey::from_type(ty);

        // Return existing ID if already indexed (handles recursion)
        if let Some(&id) = self.type_ids.get(&key) {
            return id;
        }

        // Allocate ID first (before recursing) to handle cycles
        let id = self.next_id;
        self.next_id += 1;
        self.type_ids.insert(key, id);

        let kind = self.build_type_kind_with_context(ty, ctx);
        self.type_info.insert(id, TypeInfo { kind, id, source_type: ty.clone() });
        id
    }

    fn build_type_kind_with_context(&mut self, ty: &TypeIR, ctx: &BuildContext) -> TypeKind {
        use baml_types::ir_type::TypeGeneric;

        match ty {
            TypeGeneric::Top(_) => TypeKind::Top,

            TypeGeneric::Primitive(type_value, _) => {
                let prim = match type_value {
                    TypeValue::String => PrimitiveKind::String,
                    TypeValue::Int => PrimitiveKind::Int,
                    TypeValue::Float => PrimitiveKind::Float,
                    TypeValue::Bool => PrimitiveKind::Bool,
                    TypeValue::Null => PrimitiveKind::Null,
                    TypeValue::Media(_) => PrimitiveKind::Media,
                };
                TypeKind::Primitive(prim)
            }

            TypeGeneric::Enum { name, .. } => {
                // Get enum values from context
                // Enum.values is Vec<(Name, Option<String>)> where Name has rendered_name() and real_name()
                let (values, fuzzy_map) = if let Some(of) = ctx.of {
                    if let Some(e) = of.enums.get(name) {
                        let mut values_map = HashMap::new();
                        let mut fuzzy = HashMap::new();
                        // e.values is Vec<(Name, Option<String>)>
                        for (val_name, description) in &e.values {
                            let real = val_name.real_name().to_string();
                            let rendered = val_name.rendered_name().to_string();

                            // Map rendered name to real name
                            values_map.insert(rendered.clone(), real.clone());

                            // Check if this value has an alias (rendered != real)
                            let has_alias = rendered != real;

                            // Fuzzy matching: always add rendered name variations
                            fuzzy.insert(rendered.to_lowercase(), real.clone());
                            let rendered_unaccented = remove_accents(&rendered).to_lowercase();
                            fuzzy.insert(rendered_unaccented, real.clone());

                            // Only add real name variations if there's no alias
                            if !has_alias {
                                fuzzy.insert(real.to_lowercase(), real.clone());
                                let real_unaccented = remove_accents(&real).to_lowercase();
                                fuzzy.insert(real_unaccented, real.clone());
                            }

                            // Also add description as fuzzy match if present
                            if let Some(desc) = description {
                                let desc_trimmed = desc.trim();
                                if !desc_trimmed.is_empty() {
                                    fuzzy.insert(desc_trimmed.to_lowercase(), real.clone());
                                    fuzzy.insert(remove_accents(desc_trimmed).to_lowercase(), real.clone());
                                }
                            }
                        }
                        (values_map, fuzzy)
                    } else {
                        (HashMap::new(), HashMap::new())
                    }
                } else {
                    (HashMap::new(), HashMap::new())
                };

                TypeKind::Enum {
                    name: name.clone(),
                    values,
                    fuzzy_map,
                }
            }

            TypeGeneric::Literal(lit, _) => match lit {
                LiteralValue::String(s) => TypeKind::Literal(LiteralKind::String(s.clone())),
                LiteralValue::Int(i) => TypeKind::Literal(LiteralKind::Int(*i)),
                LiteralValue::Bool(b) => TypeKind::Literal(LiteralKind::Bool(*b)),
            },

            TypeGeneric::Class { name, mode, .. } => {
                // Get class fields from context
                // Class.fields is Vec<(Name, TypeIR, Option<String>, bool)>
                // where the tuple is (name, type, description, streaming_needed)
                let (fields, required, fuzzy_fields) = if let Some(of) = ctx.of {
                    // Try the exact mode first, then NonStreaming as fallback
                    let class = of.classes.get(&(name.clone(), *mode))
                        .or_else(|| of.classes.get(&(name.clone(), StreamingMode::NonStreaming)));

                    if let Some(c) = class {
                        let mut fields_map = HashMap::new();
                        let mut required_set = HashSet::new();
                        let mut fuzzy_map = HashMap::new();

                        // c.fields is Vec<(Name, TypeIR, Option<String>, bool)>
                        for (field_name, field_type, _description, _streaming_needed) in &c.fields {
                            // Index the field type
                            let field_type_id = self.index_type_with_context(field_type, ctx);
                            let is_required = !field_type.is_optional();

                            let rendered = field_name.rendered_name().to_string();
                            let real = field_name.real_name().to_string();

                            let field_info = FieldInfo {
                                name: rendered.clone(),
                                real_name: real.clone(),
                                type_id: field_type_id,
                                required: is_required,
                                source_type: field_type.clone(),
                            };

                            if is_required {
                                required_set.insert(rendered.clone());
                            }

                            // Add fuzzy mappings for field name matching
                            // Only map the alias (rendered_name) for fuzzy matching
                            // If an alias exists (rendered != real), we should NOT accept the original name
                            let has_alias = rendered != real;

                            // Map lowercase rendered name
                            fuzzy_map.insert(rendered.to_lowercase(), rendered.clone());
                            // Map unaccented lowercase version of rendered name
                            let rendered_unaccented = remove_accents(&rendered).to_lowercase();
                            fuzzy_map.insert(rendered_unaccented, rendered.clone());

                            // Only add real name mappings if there's no alias
                            if !has_alias {
                                fuzzy_map.insert(real.to_lowercase(), rendered.clone());
                                let real_unaccented = remove_accents(&real).to_lowercase();
                                fuzzy_map.insert(real_unaccented, rendered.clone());
                            }

                            fields_map.insert(rendered, field_info);
                        }

                        (fields_map, required_set, fuzzy_map)
                    } else {
                        (HashMap::new(), HashSet::new(), HashMap::new())
                    }
                } else {
                    (HashMap::new(), HashSet::new(), HashMap::new())
                };

                TypeKind::Class {
                    name: name.clone(),
                    fields,
                    required,
                    fuzzy_fields,
                }
            }

            TypeGeneric::List(elem, _) => {
                let elem_id = self.index_type_with_context(elem, ctx);
                TypeKind::List { element: elem_id }
            }

            TypeGeneric::Map(key_ty, val_ty, _) => {
                let key_id = self.index_type_with_context(key_ty, ctx);
                let val_id = self.index_type_with_context(val_ty, ctx);
                TypeKind::Map {
                    key: key_id,
                    value: val_id,
                }
            }

            TypeGeneric::Union(union_type, _) => {
                let variant_ids: Vec<TypeId> = union_type
                    .iter_skip_null()
                    .into_iter()
                    .map(|v| self.index_type_with_context(v, ctx))
                    .collect();

                // Check if nullable
                if union_type.is_optional() {
                    // If it's a simple optional (T | null), treat as Optional
                    if variant_ids.len() == 1 {
                        return TypeKind::Optional {
                            inner: variant_ids[0],
                        };
                    }
                }

                // Build key -> variant index map for class-like variants
                let mut key_to_variants: HashMap<String, Vec<usize>> = HashMap::new();
                for (i, &var_id) in variant_ids.iter().enumerate() {
                    if let Some(TypeInfo {
                        kind: TypeKind::Class { fields, fuzzy_fields, .. },
                        ..
                    }) = self.type_info.get(&var_id)
                    {
                        for key in fields.keys() {
                            key_to_variants.entry(key.clone()).or_default().push(i);
                        }
                        // Also add fuzzy keys so unions can match aliased field names
                        for key in fuzzy_fields.keys() {
                            key_to_variants.entry(key.clone()).or_default().push(i);
                        }
                    }
                }

                TypeKind::Union {
                    variants: variant_ids,
                    key_to_variants,
                }
            }

            TypeGeneric::Tuple(elems, _) => {
                let elem_ids: Vec<TypeId> = elems.iter().map(|e| self.index_type_with_context(e, ctx)).collect();
                TypeKind::Tuple { elements: elem_ids }
            }

            TypeGeneric::RecursiveTypeAlias { name, .. } => {
                // Look up the resolved type structure from OutputFormatContent
                if let Some(of) = ctx.of {
                    if let Some(inner_type) = of.structural_recursive_aliases.get(name) {
                        // Index the inner type (which may reference back to this alias)
                        let inner_id = self.index_type_with_context(inner_type, ctx);
                        return TypeKind::RecursiveAlias {
                            name: name.clone(),
                            target: Some(inner_id),
                        };
                    }
                }
                // Fallback if not found in context
                TypeKind::RecursiveAlias {
                    name: name.clone(),
                    target: None,
                }
            }

            TypeGeneric::Arrow(_, _) => {
                // Function types are not directly parseable
                TypeKind::Top
            }
        }
    }

    /// Check if a type is a class
    pub fn is_class(&self, id: TypeId) -> bool {
        matches!(
            self.type_info.get(&id),
            Some(TypeInfo {
                kind: TypeKind::Class { .. },
                ..
            })
        )
    }

    /// Check if a type is a list
    pub fn is_list(&self, id: TypeId) -> bool {
        matches!(
            self.type_info.get(&id),
            Some(TypeInfo {
                kind: TypeKind::List { .. },
                ..
            })
        )
    }

    /// Check if a type is a union
    pub fn is_union(&self, id: TypeId) -> bool {
        matches!(
            self.type_info.get(&id),
            Some(TypeInfo {
                kind: TypeKind::Union { .. },
                ..
            })
        )
    }

    /// Get union variants
    pub fn union_variants(&self, id: TypeId) -> Option<&[TypeId]> {
        match self.type_info.get(&id) {
            Some(TypeInfo {
                kind: TypeKind::Union { variants, .. },
                ..
            }) => Some(variants),
            _ => None,
        }
    }

    /// Get list element type (resolves through RecursiveAlias)
    pub fn list_element(&self, id: TypeId) -> Option<TypeId> {
        match self.type_info.get(&id) {
            Some(TypeInfo {
                kind: TypeKind::List { element },
                ..
            }) => Some(*element),
            Some(TypeInfo {
                kind: TypeKind::RecursiveAlias { target: Some(inner), .. },
                ..
            }) => self.list_element(*inner),
            Some(TypeInfo {
                kind: TypeKind::Optional { inner },
                ..
            }) => self.list_element(*inner),
            _ => None,
        }
    }

    /// Check if type expects an object structure
    pub fn expects_object(&self, id: TypeId) -> bool {
        match self.type_info.get(&id) {
            Some(TypeInfo { kind, .. }) => match kind {
                TypeKind::Class { .. } | TypeKind::Map { .. } => true,
                TypeKind::Union { variants, .. } => {
                    variants.iter().any(|&v| self.expects_object(v))
                }
                TypeKind::Optional { inner } => self.expects_object(*inner),
                TypeKind::RecursiveAlias { target: Some(inner), .. } => self.expects_object(*inner),
                _ => false,
            },
            None => false,
        }
    }

    /// Check if type expects an array structure
    pub fn expects_array(&self, id: TypeId) -> bool {
        match self.type_info.get(&id) {
            Some(TypeInfo { kind, .. }) => match kind {
                TypeKind::List { .. } | TypeKind::Tuple { .. } => true,
                TypeKind::Union { variants, .. } => {
                    variants.iter().any(|&v| self.expects_array(v))
                }
                TypeKind::Optional { inner } => self.expects_array(*inner),
                TypeKind::RecursiveAlias { target: Some(inner), .. } => self.expects_array(*inner),
                _ => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baml_types::type_meta;

    #[test]
    fn test_primitive_indexing() {
        let ty = TypeIR::Primitive(TypeValue::String, type_meta::IR::default());
        let index = SchemaIndex::build(&ty, None);

        let info = index.get(index.root_id()).unwrap();
        assert!(matches!(
            info.kind,
            TypeKind::Primitive(PrimitiveKind::String)
        ));
    }

    #[test]
    fn test_list_indexing() {
        let elem = Box::new(TypeIR::Primitive(TypeValue::Int, type_meta::IR::default()));
        let ty = TypeIR::List(elem, type_meta::IR::default());
        let index = SchemaIndex::build(&ty, None);

        let info = index.get(index.root_id()).unwrap();
        assert!(matches!(info.kind, TypeKind::List { .. }));

        if let TypeKind::List { element } = &info.kind {
            let elem_info = index.get(*element).unwrap();
            assert!(matches!(
                elem_info.kind,
                TypeKind::Primitive(PrimitiveKind::Int)
            ));
        }
    }
}
