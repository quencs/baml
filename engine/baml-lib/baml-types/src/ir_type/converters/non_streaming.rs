use crate::{
    baml_value::TypeLookups,
    ir_type::{ArrowGeneric, TypeNonStreaming, UnionTypeGeneric},
    type_meta, BamlMediaType, StreamingMode, TypeIR, TypeValue,
};

pub fn from_type_ir(r#type: &TypeIR, _lookup: &impl TypeLookups) -> TypeNonStreaming {
    // This inner worker function goes from `FieldType` to `FieldType` to be
    // suitable for recursive use. We only wrap the outermost `FieldType` in
    // `StreamingType`.

    // A copy of the metadata to use in the new type.
    let meta = type_meta::NonStreaming {
        constraints: r#type.meta().constraints.clone(),
    };

    // Streaming behavior of the type, without regard to the `@stream` annotations.
    // (That annotation will be handled later in this function).
    let mut base_type_streaming = match r#type {
        TypeIR::Primitive(type_value, _) => match type_value {
            TypeValue::Null => TypeNonStreaming::Primitive(TypeValue::Null, meta),
            TypeValue::Int => TypeNonStreaming::Primitive(TypeValue::Int, meta),
            TypeValue::Float => TypeNonStreaming::Primitive(TypeValue::Float, meta),
            TypeValue::Bool => TypeNonStreaming::Primitive(TypeValue::Bool, meta),
            TypeValue::String => TypeNonStreaming::Primitive(TypeValue::String, meta),
            TypeValue::Media(_) => {
                TypeNonStreaming::Primitive(TypeValue::Media(BamlMediaType::Image), meta)
            }
        },
        TypeIR::Enum { name, dynamic, .. } => TypeNonStreaming::Enum {
            name: name.clone(),
            dynamic: *dynamic,
            meta: meta.clone(),
        },
        TypeIR::Literal(literal_value, _) => TypeNonStreaming::Literal(literal_value.clone(), meta),
        TypeIR::Class { name, dynamic, .. } => TypeNonStreaming::Class {
            name: name.clone(),
            mode: StreamingMode::NonStreaming,
            dynamic: *dynamic,
            meta: meta.clone(),
        },
        TypeIR::List(item_type, _) => {
            TypeNonStreaming::List(Box::new(from_type_ir(item_type, _lookup)), meta)
        }
        TypeIR::Map(key_type, item_type, _) => TypeNonStreaming::Map(
            {
                // Keys cannot be null in maps
                let mut clone = key_type.clone();
                clone.meta_mut().streaming_behavior.needed = true;
                Box::new(from_type_ir(&clone, _lookup))
            },
            Box::new(from_type_ir(item_type, _lookup)),
            meta,
        ),
        TypeIR::RecursiveTypeAlias { name, .. } => TypeNonStreaming::RecursiveTypeAlias {
            name: name.clone(),
            meta: meta.clone(),
        },
        TypeIR::Tuple(field_types, _) => TypeNonStreaming::Tuple(
            field_types
                .iter()
                .map(|t| from_type_ir(t, _lookup))
                .collect(),
            meta,
        ),
        TypeIR::Arrow(arrow, _) => TypeNonStreaming::Arrow(
            Box::new(ArrowGeneric {
                param_types: arrow
                    .param_types
                    .iter()
                    .map(|t| from_type_ir(t, _lookup))
                    .collect(),
                return_type: from_type_ir(&arrow.return_type, _lookup),
            }),
            meta,
        ),
        TypeIR::Union(union_type, _) => {
            let variants = union_type.iter_skip_null();
            let variants = variants
                .into_iter()
                .cloned()
                .map(|t| from_type_ir(&t, _lookup));

            TypeNonStreaming::Union(
                unsafe { UnionTypeGeneric::new_unsafe(variants.collect()) },
                meta,
            )
        }
    };
    if base_type_streaming.is_optional() {
        // Needed streaming types, and streaming types that are optional, need
        // no further processing to add optionality.
        base_type_streaming
    } else {
        // Currently base_type_streaming has the interesting metadata.
        // In the union we create to make base_type_streaming optional,
        // we want that inner metadata to be default, our outer union to
        // have the metadata. So we create a new default metadata and swap
        // its memory with that of the inner base_type.
        let meta = base_type_streaming.meta().clone();
        *base_type_streaming.meta_mut() = Default::default();
        let mut optional_value = TypeNonStreaming::Union(
            unsafe {
                UnionTypeGeneric::new_unsafe(vec![base_type_streaming, TypeNonStreaming::null()])
            },
            Default::default(),
        );
        *optional_value.meta_mut() = meta;
        optional_value
    }
}
