use std::sync::{Arc, Mutex};

use baml_type_builder::{
    ClassPropertyBuilder, EnumBuilder as EnumBuilderTrait, EnumValueBuilder, Meta,
};
use baml_types::BamlValue;
use indexmap::IndexMap;

use crate::{
    runtime_context::{PropertyAttributes, RuntimeClassOverride, RuntimeEnumOverride},
    InternalBamlRuntime,
};

// Re-export types for external use
pub use crate::runtime::runtime_interface::TypeBuilder;
pub use baml_type_builder::ClassBuilder;

// Conversion impl for ClassPropertyBuilder and EnumValueBuilder
impl PropertyAttributes {
    fn from_class_property_builder(value: &ClassPropertyBuilder<InternalBamlRuntime>) -> Self {
        let alias = value.alias().unwrap_or_default();
        let description = value.description().unwrap_or_default();

        Self {
            alias,
            description,
            constraints: Vec::new(),
            streaming_behavior: Default::default(),
        }
    }

    fn from_enum_value_builder(value: &EnumValueBuilder<InternalBamlRuntime>) -> Self {
        let alias = value.alias().unwrap_or_default();
        let description = value.description().unwrap_or_default();

        Self {
            alias,
            description,
            constraints: Vec::new(),
            streaming_behavior: Default::default(),
        }
    }
}

// Helper function to convert core TypeBuilder to runtime overrides
pub fn to_overrides(
    type_builder: &crate::runtime::runtime_interface::TypeBuilder,
) -> (
    IndexMap<String, RuntimeClassOverride>,
    IndexMap<String, RuntimeEnumOverride>,
    IndexMap<String, baml_types::TypeIR>,
    Vec<indexmap::IndexSet<String>>,
    Vec<IndexMap<String, baml_types::TypeIR>>,
) {
    log::debug!("Converting types to overrides");
    let cls = type_builder
        .list_classes()
        .into_iter()
        .filter_map(|cls| {
            log::debug!("Converting class: {}", cls.class_name);
            let mut overrides = RuntimeClassOverride {
                alias: None,
                new_fields: Default::default(),
                update_fields: Default::default(),
            };

            cls.list_properties()
                .map(|properties| {
                    for property in properties {
                        let name = property.property_name.clone();
                        let attrs = PropertyAttributes::from_class_property_builder(&property);
                        if property
                            .is_from_ast()
                            .expect("Should be able to check if property is from ast")
                        {
                            overrides.update_fields.insert(name, attrs);
                        } else {
                            overrides
                                .new_fields
                                .insert(name, (property.type_().unwrap(), attrs));
                        }
                    }
                    (cls.class_name.clone(), overrides)
                })
                .ok()
        })
        .collect::<IndexMap<String, RuntimeClassOverride>>();

    let enm = type_builder
        .list_enums()
        .into_iter()
        .filter_map(|enm: EnumBuilderTrait<_>| {
            enm.list_values().ok().map(|values| {
                let mut overrides = RuntimeEnumOverride {
                    alias: None,
                    values: Default::default(),
                };

                for value in values {
                    let value_name = value.value_name.clone();
                    let attrs = PropertyAttributes::from_enum_value_builder(&value);
                    overrides.values.insert(value_name, attrs);
                }
                (enm.enum_name.clone(), overrides)
            })
        })
        .collect();

    let alias = type_builder
        .recursive_type_aliases()
        .lock()
        .unwrap()
        .iter()
        .flat_map(|map| map.iter())
        .map(|(name, ty)| (name.clone(), ty.clone()))
        .collect();

    let recursive_classes = type_builder.recursive_classes().lock().unwrap().clone();
    let recursive_type_aliases = type_builder
        .recursive_type_aliases()
        .lock()
        .unwrap()
        .clone();

    (cls, enm, alias, recursive_classes, recursive_type_aliases)
}
