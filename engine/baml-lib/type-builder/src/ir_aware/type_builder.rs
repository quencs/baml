use crate::{
    core::{self, TypeBuilder as CoreTypeBuilder, WithMeta},
    traits::{IRProvider, RuntimeProvider},
    Meta,
};
use baml_types::{BamlValue, EvaluationContext, TypeIR};
use indexmap::{IndexMap, IndexSet};
use internal_baml_core::ir::{repr::TypeBuilderEntry, IRHelper};
use std::sync::{Arc, Mutex};

type RuntimeTypeBuilder = Arc<CoreTypeBuilder>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRW {
    // Only view the data (no modifications allowed)
    ReadOnly,
    // View data, but can modify attributes like alias / description
    LLMOnly,
    // Go wild
    ReadWrite,
}

impl NodeRW {
    fn at_least(&self, other: NodeRW) -> anyhow::Result<()> {
        if self < &other {
            anyhow::bail!(
                "Insufficient permissions to perform this operation: {:?} < {:?}",
                self,
                other
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_rw_at_least() {
        assert!(NodeRW::ReadOnly.at_least(NodeRW::ReadOnly).is_ok());
        assert!(NodeRW::ReadOnly.at_least(NodeRW::LLMOnly).is_err());
        assert!(NodeRW::ReadOnly.at_least(NodeRW::ReadWrite).is_err());

        assert!(NodeRW::LLMOnly.at_least(NodeRW::ReadOnly).is_ok());
        assert!(NodeRW::LLMOnly.at_least(NodeRW::LLMOnly).is_ok());
        assert!(NodeRW::LLMOnly.at_least(NodeRW::ReadWrite).is_err());

        assert!(NodeRW::ReadWrite.at_least(NodeRW::ReadOnly).is_ok());
        assert!(NodeRW::ReadWrite.at_least(NodeRW::LLMOnly).is_ok());
        assert!(NodeRW::ReadWrite.at_least(NodeRW::ReadWrite).is_ok());
    }
}

#[derive(Debug, Clone)]
pub struct TypeBuilder<IR: IRProvider> {
    type_builder: RuntimeTypeBuilder,
    ir_provider: Arc<IR>,
}

impl<IR: IRProvider> TypeBuilder<IR> {
    pub fn new(ir_provider: Arc<IR>) -> Self {
        Self {
            type_builder: Arc::new(CoreTypeBuilder::new()),
            ir_provider,
        }
    }

    pub fn add_enum(&self, name: &str) -> anyhow::Result<EnumBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        match ir.find_enum(name) {
            Ok(_) => {
                anyhow::bail!("Enum with name {name} already exists");
            }
            Err(_) => {
                let _ = self.type_builder.upsert_enum(name);
                let builder = EnumBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                );
                Ok(builder.mode(NodeRW::ReadWrite))
            }
        }
    }

    pub fn add_class(&self, name: &str) -> anyhow::Result<ClassBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        match ir.find_class(name) {
            Ok(_) => {
                anyhow::bail!("Class with name {name} already exists");
            }
            Err(_) => {
                let _ = self.type_builder.upsert_class(name);
                let builder = ClassBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                );
                Ok(builder.mode(NodeRW::ReadWrite))
            }
        }
    }

    pub fn class(&self, name: &str) -> anyhow::Result<ClassBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        match ir.find_class(name) {
            Ok(cls) => {
                let _ = self.type_builder.upsert_class(name);
                let builder = ClassBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                );
                if !cls.item.attributes.dynamic() {
                    Ok(builder.mode(NodeRW::ReadOnly))
                } else {
                    Ok(builder.mode(NodeRW::ReadWrite))
                }
            }
            Err(_) => match self.type_builder.maybe_get_class(name) {
                Some(_) => Ok(ClassBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                )),
                None => {
                    anyhow::bail!("Class with name {name} does not exist");
                }
            },
        }
    }

    pub fn r#enum(&self, name: &str) -> anyhow::Result<EnumBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        match ir.find_enum(name) {
            Ok(enm) => {
                let _ = self.type_builder.upsert_enum(name);
                let builder = EnumBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                );
                if !enm.item.attributes.dynamic() {
                    return Ok(builder.mode(NodeRW::ReadOnly));
                }
                Ok(builder.mode(NodeRW::ReadWrite))
            }
            Err(_) => match self.type_builder.maybe_get_enum(name) {
                Some(_) => Ok(EnumBuilder::new(
                    self.type_builder.clone(),
                    self.ir_provider.clone(),
                    name.to_string(),
                )),
                None => {
                    anyhow::bail!("Enum with name {name} does not exist");
                }
            },
        }
    }

    /// Get the underlying core type builder
    pub fn core(&self) -> &Arc<CoreTypeBuilder> {
        &self.type_builder
    }

    pub fn list_enums(&self) -> Vec<EnumBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        let enums = ir.walk_enums();
        enums
            .map(|enm| enm.name().to_string())
            .chain(self.type_builder.list_enums())
            .collect::<indexmap::IndexSet<_>>()
            .into_iter()
            .map(|name| EnumBuilder::new(self.type_builder.clone(), self.ir_provider.clone(), name))
            .collect()
    }

    pub fn list_classes(&self) -> Vec<ClassBuilder<IR>> {
        let ir = self.ir_provider.get_ir();
        let classes = ir.walk_classes();
        classes
            .map(|cls| cls.name().to_string())
            .chain(self.type_builder.list_classes())
            .collect::<indexmap::IndexSet<_>>()
            .into_iter()
            .map(|name| {
                ClassBuilder::new(self.type_builder.clone(), self.ir_provider.clone(), name)
            })
            .collect()
    }

    pub fn maybe_get_enum(&self, name: &str) -> Option<Arc<Mutex<core::EnumBuilder>>> {
        self.type_builder.maybe_get_enum(name)
    }

    pub fn recursive_type_aliases(&self) -> Arc<Mutex<Vec<IndexMap<String, TypeIR>>>> {
        self.type_builder.recursive_type_aliases()
    }

    pub fn recursive_classes(&self) -> Arc<Mutex<Vec<IndexSet<String>>>> {
        self.type_builder.recursive_classes()
    }

    pub fn reset(&self) {
        self.type_builder.reset();
    }
}

#[derive(Debug, Clone)]
pub struct ClassBuilder<IR: IRProvider> {
    type_builder: RuntimeTypeBuilder,
    ir_provider: Arc<IR>,
    pub class_name: String,
    mode: NodeRW,
}

impl<IR: IRProvider> ClassBuilder<IR> {
    fn new(type_builder: RuntimeTypeBuilder, ir_provider: Arc<IR>, class_name: String) -> Self {
        Self {
            type_builder,
            ir_provider,
            class_name,
            mode: NodeRW::ReadOnly,
        }
    }

    fn mode(self, mode: NodeRW) -> Self {
        Self { mode, ..self }
    }

    fn create_property(&self, name: &str) -> ClassPropertyBuilder<IR> {
        let builder = ClassPropertyBuilder::new(
            self.type_builder.clone(),
            self.ir_provider.clone(),
            self.class_name.clone(),
            name.to_string(),
        );

        let target_mode = match self.mode {
            NodeRW::ReadOnly => NodeRW::ReadOnly,
            NodeRW::LLMOnly => NodeRW::LLMOnly,
            NodeRW::ReadWrite => {
                let ir = self.ir_provider.get_ir();
                if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                    if cls.find_field(name).is_some() {
                        NodeRW::LLMOnly
                    } else {
                        NodeRW::ReadWrite
                    }
                } else {
                    NodeRW::ReadWrite
                }
            }
        };

        builder.mode(target_mode)
    }

    fn cls(&self) -> anyhow::Result<Arc<std::sync::Mutex<core::ClassBuilder>>> {
        let ir = self.ir_provider.get_ir();
        // if the IR defines the class, then its always valid
        if ir.find_class(self.class_name.as_str()).is_ok() {
            let cls = self.type_builder.upsert_class(self.class_name.as_str());
            return Ok(cls);
        }

        let Some(cls) = self.type_builder.maybe_get_class(self.class_name.as_str()) else {
            anyhow::bail!("Class not found: {}", self.class_name);
        };
        Ok(cls)
    }

    pub fn r#type(&self) -> anyhow::Result<TypeIR> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let _ = self.cls()?;

        Ok(TypeIR::class(self.class_name.as_str()))
    }

    pub fn list_properties(&self) -> anyhow::Result<Vec<ClassPropertyBuilder<IR>>> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let lock = self.cls()?;
        let builder = lock.lock().unwrap();

        let ir = self.ir_provider.get_ir();
        let ir_properties = match ir.find_class(self.class_name.as_str()) {
            Ok(ir_cls) => ir_cls
                .item
                .elem
                .static_fields
                .iter()
                .map(|field| field.elem.name.to_string())
                .collect(),
            Err(_) => vec![],
        };

        let dynamic_properties = builder
            .list_properties()
            .into_iter()
            .filter(|name| !ir_properties.contains(name))
            .collect::<Vec<_>>();

        let properties = ir_properties.into_iter().chain(dynamic_properties);
        Ok(properties
            .into_iter()
            .map(|name| self.create_property(name.as_str()))
            .collect())
    }

    pub fn set_alias(&self, alias: &str) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        builder.with_meta("alias", BamlValue::String(alias.to_string()));
        Ok(())
    }

    pub fn set_description(&self, description: &str) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        builder.with_meta("description", BamlValue::String(description.to_string()));
        Ok(())
    }

    pub fn alias(&self) -> Result<Option<String>, anyhow::Error> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let ast_alias = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                cls.alias(&Default::default()).ok().flatten()
            } else {
                None
            }
        };

        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        let result = builder
            .get_meta("alias")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_alias);
        Ok(result)
    }

    pub fn description(&self) -> Result<Option<String>, anyhow::Error> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        // ast does not support description
        let ast_description = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                cls.description(&Default::default()).ok().flatten()
            } else {
                None
            }
        };

        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        let result = builder
            .get_meta("description")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_description);
        Ok(result)
    }

    pub fn add_property(
        &self,
        name: &str,
        field_type: TypeIR,
    ) -> anyhow::Result<ClassPropertyBuilder<IR>> {
        self.mode.at_least(NodeRW::ReadWrite)?;
        let cls = self.cls()?;

        // if the IR already has the property, then its not valid to add it again
        let ir = self.ir_provider.get_ir();
        if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
            if cls.find_field(name).is_some() {
                anyhow::bail!(
                    "Property already exists: {} in class {}",
                    name,
                    self.class_name
                );
            }
        }

        let builder = cls.lock().unwrap();
        let prop = builder.upsert_property(name);
        prop.lock().unwrap().set_type(field_type);
        Ok(self.create_property(name))
    }

    pub fn remove_property(&self, name: &str) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::ReadWrite)?;
        // if the IR already has the property, then its not valid to remove it
        let ir = self.ir_provider.get_ir();
        if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
            if cls.find_field(name).is_some() {
                anyhow::bail!(
                    "Property is statically defined: {} in class {}. Cannot remove it.",
                    name,
                    self.class_name
                );
            }
        }

        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        builder.remove_property(name);
        Ok(())
    }

    pub fn property(&self, name: &str) -> anyhow::Result<ClassPropertyBuilder<IR>> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let cls = self.cls()?;

        let builder = cls.lock().unwrap();
        match builder.maybe_get_property(name) {
            Some(_) => Ok(self.create_property(name)),
            None => {
                // if the IR has the property, then its valid to add it again
                let ir = self.ir_provider.get_ir();
                if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                    if cls.find_field(name).is_some() {
                        let _ = builder.upsert_property(name);
                        Ok(self.create_property(name))
                    } else {
                        anyhow::bail!("Property not found: {} in class {}", name, self.class_name)
                    }
                } else {
                    anyhow::bail!("Property not found: {} in class {}", name, self.class_name)
                }
            }
        }
    }

    pub fn is_from_ast(&self) -> anyhow::Result<bool> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let ir = self.ir_provider.get_ir();
        Ok(ir.find_class(self.class_name.as_str()).is_ok())
    }

    pub fn reset(&self) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::ReadWrite)?;
        let cls = self.cls()?;
        let builder = cls.lock().unwrap();
        builder.reset();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ClassPropertyBuilder<IR: IRProvider> {
    type_builder: RuntimeTypeBuilder,
    ir_provider: Arc<IR>,
    class_name: String,
    pub property_name: String,
    mode: NodeRW,
}

// impl<IR: IRProvider> WithMeta for ClassPropertyBuilder<IR> {
//     fn get_meta(&self, key: &str) -> Option<BamlValue> {
//         let prop = self.prop()?;

//         let ast_description = || {
//             let ir = self.ir_provider.get_ir();
//             if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
//                 if let Some(field) = cls.find_field(&self.property_name) {
//                     field.description(&Default::default()).ok().flatten()
//                 } else {
//                     None
//                 }
//             } else {
//                 None
//             }
//         };
//         let builder = prop.lock().unwrap();
//         let result = builder
//             .get_meta("description")
//             .and_then(|value| value.as_str().map(|s| s.to_string()))
//             .or_else(ast_description);
//     }

//     fn with_meta(&self, key: &str, value: BamlValue) -> &Self {
//         match key {
//             "alias" => {
//                 self.set_alias(&value.as_str().unwrap()).unwrap();
//             }
//             "description" => {
//                 self.set_description(&value.as_str().unwrap()).unwrap();
//             }
//             _ => {}
//         }
//         self
//     }
// }

impl<IR: IRProvider> ClassPropertyBuilder<IR> {
    fn new(
        type_builder: RuntimeTypeBuilder,
        ir_provider: Arc<IR>,
        class_name: String,
        property_name: String,
    ) -> Self {
        Self {
            type_builder,
            ir_provider,
            class_name,
            property_name,
            mode: NodeRW::ReadOnly,
        }
    }

    fn mode(self, mode: NodeRW) -> Self {
        Self { mode, ..self }
    }

    fn prop(&self) -> anyhow::Result<Arc<std::sync::Mutex<core::ClassPropertyBuilder>>> {
        let ir = self.ir_provider.get_ir();
        // if the class is defined in the IR, then its always valid
        if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
            if cls.find_field(&self.property_name).is_some() {
                let cls = self.type_builder.upsert_class(self.class_name.as_str());
                let builder = cls.lock().unwrap();
                let prop = builder.upsert_property(&self.property_name);
                return Ok(prop);
            }
        }

        let Some(cls) = self.type_builder.maybe_get_class(self.class_name.as_str()) else {
            return Err(anyhow::anyhow!("Class not found: {}", self.class_name));
        };
        let builder = cls.lock().unwrap();
        match builder.maybe_get_property(&self.property_name) {
            Some(prop) => Ok(prop),
            None => {
                anyhow::bail!(
                    "Property not found: {} in class {}",
                    self.property_name,
                    self.class_name
                )
            }
        }
    }

    pub fn description(&self) -> Result<Option<String>, anyhow::Error> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let prop = self.prop()?;

        let ast_description = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                if let Some(field) = cls.find_field(&self.property_name) {
                    field.description(&Default::default()).ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        };
        let builder = prop.lock().unwrap();
        let result = builder
            .get_meta("description")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_description);
        Ok(result)
    }

    pub fn alias(&self) -> Result<Option<String>, anyhow::Error> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let prop = self.prop()?;

        let ast_alias = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                if let Some(field) = cls.find_field(&self.property_name) {
                    field.description(&Default::default()).ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        };
        let builder = prop.lock().unwrap();
        let result = builder
            .get_meta("alias")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_alias);
        Ok(result)
    }

    pub fn type_(&self) -> Result<TypeIR, anyhow::Error> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let ast_type = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
                cls.find_field(&self.property_name)
                    .map(|field| field.r#type().clone())
            } else {
                None
            }
        };

        let prop = self.prop()?;
        let builder = prop.lock().unwrap();
        let result = builder.r#type().or_else(ast_type).ok_or_else(|| {
            anyhow::anyhow!(
                "Type not found for property {} in class {}",
                self.property_name,
                self.class_name
            )
        });
        result
    }

    pub fn set_description(&self, description: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let prop = self.prop()?;
        let builder = prop.lock().unwrap();
        match description {
            Some(description) => {
                builder.with_meta("description", BamlValue::String(description.to_string()));
            }
            None => {
                builder.remove_meta("description");
            }
        }
        Ok(())
    }

    pub fn set_alias(&self, alias: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let prop = self.prop()?;
        let builder = prop.lock().unwrap();
        match alias {
            Some(alias) => {
                builder.with_meta("alias", BamlValue::String(alias.to_string()));
            }
            None => {
                builder.remove_meta("alias");
            }
        }
        Ok(())
    }

    pub fn set_type(&self, field_type: TypeIR) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::ReadWrite)?;

        let prop = self.prop()?;
        let builder = prop.lock().unwrap();
        builder.set_type(field_type);
        Ok(())
    }

    pub fn is_from_ast(&self) -> anyhow::Result<bool> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let ir = self.ir_provider.get_ir();
        if let Ok(cls) = ir.find_class(self.class_name.as_str()) {
            if cls.find_field(&self.property_name).is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Debug, Clone)]
pub struct EnumBuilder<IR: IRProvider> {
    type_builder: RuntimeTypeBuilder,
    ir_provider: Arc<IR>,
    pub enum_name: String,
    mode: NodeRW,
}

impl<IR: IRProvider> EnumBuilder<IR> {
    fn new(type_builder: RuntimeTypeBuilder, ir_provider: Arc<IR>, enum_name: String) -> Self {
        Self {
            type_builder,
            ir_provider,
            enum_name,
            mode: NodeRW::ReadOnly,
        }
    }

    fn mode(self, mode: NodeRW) -> Self {
        Self { mode, ..self }
    }

    fn enm(&self) -> anyhow::Result<Arc<std::sync::Mutex<core::EnumBuilder>>> {
        let ir = self.ir_provider.get_ir();
        // if the IR defines the enum, then its always valid
        if ir.find_enum(self.enum_name.as_str()).is_ok() {
            return Ok(self.type_builder.upsert_enum(self.enum_name.as_str()));
        }

        let Some(enm) = self.type_builder.maybe_get_enum(self.enum_name.as_str()) else {
            anyhow::bail!("Enum not found: {}", self.enum_name);
        };
        Ok(enm)
    }

    fn create_value(&self, name: &str) -> EnumValueBuilder<IR> {
        let target_mode = match self.mode {
            NodeRW::ReadOnly => NodeRW::ReadOnly,
            NodeRW::LLMOnly => NodeRW::LLMOnly,
            NodeRW::ReadWrite => {
                let ir = self.ir_provider.get_ir();
                if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                    if enm.find_value(name).is_some() {
                        NodeRW::LLMOnly
                    } else {
                        NodeRW::ReadWrite
                    }
                } else {
                    NodeRW::ReadWrite
                }
            }
        };

        EnumValueBuilder::new(
            self.type_builder.clone(),
            self.ir_provider.clone(),
            self.enum_name.clone(),
            name.to_string(),
        )
        .mode(target_mode)
    }

    pub fn add_value(&self, value: &str) -> anyhow::Result<EnumValueBuilder<IR>> {
        self.mode.at_least(NodeRW::ReadWrite)?;
        let enm = self.enm()?;

        // if the IR already has the value, then its not valid to add it again
        let ir = self.ir_provider.get_ir();
        if let Ok(enm_ir) = ir.find_enum(self.enum_name.as_str()) {
            if enm_ir.find_value(value).is_some() {
                anyhow::bail!(
                    "Enum value already exists: {} in enum {}",
                    value,
                    self.enum_name
                );
            }
        }

        let builder = enm.lock().unwrap();
        let _ = builder.upsert_value(value);
        Ok(self.create_value(value))
    }

    pub fn set_description(&self, description: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let enm = self.enm()?;
        let builder = enm.lock().unwrap();
        match description {
            Some(description) => {
                builder.with_meta("description", BamlValue::String(description.to_string()));
            }
            None => {
                builder.remove_meta("description");
            }
        }
        Ok(())
    }

    pub fn set_alias(&self, alias: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let enm = self.enm()?;
        let builder = enm.lock().unwrap();
        match alias {
            Some(alias) => {
                builder.with_meta("alias", BamlValue::String(alias.to_string()));
            }
            None => {
                builder.remove_meta("alias");
            }
        }
        Ok(())
    }

    pub fn alias(&self) -> Result<Option<String>, anyhow::Error> {
        let ast_alias = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                enm.alias(&Default::default()).ok().flatten()
            } else {
                None
            }
        };

        let enm = self.enm()?;
        let builder = enm.lock().unwrap();
        let result = builder
            .get_meta("alias")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_alias);
        Ok(result)
    }

    pub fn description(&self) -> Result<Option<String>, anyhow::Error> {
        let ast_description = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                enm.description(&Default::default()).ok().flatten()
            } else {
                None
            }
        };

        let enm = self.enm()?;
        let builder = enm.lock().unwrap();
        let result = builder
            .get_meta("description")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_description);
        Ok(result)
    }

    pub fn r#type(&self) -> anyhow::Result<TypeIR> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let _ = self.enm()?;

        Ok(TypeIR::r#enum(self.enum_name.as_str()))
    }

    pub fn list_values(&self) -> anyhow::Result<Vec<EnumValueBuilder<IR>>> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let enm = self.enm()?;

        let ir = self.ir_provider.get_ir();
        let ir_values = match ir.find_enum(self.enum_name.as_str()) {
            Ok(ir_enm) => ir_enm
                .item
                .elem
                .values
                .iter()
                .map(|value| value.0.elem.0.to_string())
                .collect(),
            Err(_) => vec![],
        };

        let builder = enm.lock().unwrap();
        let dynamic_values = builder
            .list_values()
            .into_iter()
            .filter(|name| !ir_values.contains(name))
            .collect::<Vec<_>>();

        let values = ir_values.into_iter().chain(dynamic_values);
        Ok(values
            .into_iter()
            .map(|name| self.create_value(name.as_str()))
            .collect())
    }

    pub fn value(&self, name: &str) -> anyhow::Result<EnumValueBuilder<IR>> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let enm = self.enm()?;

        let builder = enm.lock().unwrap();
        let values = builder.list_values();
        if values.contains(&name.to_string()) {
            Ok(self.create_value(name))
        } else {
            // if the IR has the value, then its valid to add it again
            let ir = self.ir_provider.get_ir();
            if let Ok(enm_ir) = ir.find_enum(self.enum_name.as_str()) {
                if enm_ir.find_value(name).is_some() {
                    let _ = builder.upsert_value(name);
                    Ok(self.create_value(name))
                } else {
                    anyhow::bail!("Enum value not found: {} in enum {}", name, self.enum_name)
                }
            } else {
                anyhow::bail!("Enum value not found: {} in enum {}", name, self.enum_name)
            }
        }
    }

    pub fn is_from_ast(&self) -> anyhow::Result<bool> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let ir = self.ir_provider.get_ir();
        Ok(ir.find_enum(self.enum_name.as_str()).is_ok())
    }
}

#[derive(Debug, Clone)]
pub struct EnumValueBuilder<IR: IRProvider> {
    type_builder: RuntimeTypeBuilder,
    ir_provider: Arc<IR>,
    enum_name: String,
    pub value_name: String,
    mode: NodeRW,
}

impl<IR: IRProvider> EnumValueBuilder<IR> {
    fn new(
        type_builder: RuntimeTypeBuilder,
        ir_provider: Arc<IR>,
        enum_name: String,
        value_name: String,
    ) -> Self {
        Self {
            type_builder,
            ir_provider,
            enum_name,
            value_name,
            mode: NodeRW::ReadOnly,
        }
    }

    fn mode(self, mode: NodeRW) -> Self {
        Self { mode, ..self }
    }

    fn value(&self) -> anyhow::Result<Arc<std::sync::Mutex<core::EnumValueBuilder>>> {
        let ir = self.ir_provider.get_ir();
        // if the enum is defined in the IR, then its always valid
        if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
            if enm.find_value(self.value_name.as_str()).is_some() {
                let enm = self.type_builder.upsert_enum(self.enum_name.as_str());
                let builder = enm.lock().unwrap();
                let value = builder.upsert_value(&self.value_name);
                return Ok(value);
            }
        }

        let Some(enm) = self.type_builder.maybe_get_enum(self.enum_name.as_str()) else {
            return Err(anyhow::anyhow!("Enum not found: {}", self.enum_name));
        };
        let builder = enm.lock().unwrap();
        match builder.maybe_get_value(&self.value_name) {
            Some(value) => Ok(value),
            None => {
                anyhow::bail!(
                    "Enum value not found: {} in enum {}",
                    self.value_name,
                    self.enum_name
                )
            }
        }
    }

    pub fn set_description(&self, description: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let value = self.value()?;
        let builder = value.lock().unwrap();
        match description {
            Some(description) => {
                builder.with_meta("description", BamlValue::String(description.to_string()));
            }
            None => {
                builder.remove_meta("description");
            }
        }
        Ok(())
    }

    pub fn set_alias(&self, alias: Option<&str>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let value = self.value()?;
        let builder = value.lock().unwrap();
        match alias {
            Some(alias) => {
                builder.with_meta("alias", BamlValue::String(alias.to_string()));
            }
            None => {
                builder.remove_meta("alias");
            }
        }
        Ok(())
    }

    pub fn description(&self) -> Result<Option<String>, anyhow::Error> {
        let ast_description = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                if let Some(value) = enm.find_value(&self.value_name) {
                    value.description(&Default::default()).ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        };

        let value = self.value()?;
        let builder = value.lock().unwrap();
        let result = builder
            .get_meta("description")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_description);
        Ok(result)
    }

    pub fn alias(&self) -> Result<Option<String>, anyhow::Error> {
        let ast_alias = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                if let Some(value) = enm.find_value(&self.value_name) {
                    value.alias(&Default::default()).ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        };

        let value = self.value()?;
        let builder = value.lock().unwrap();
        let result = builder
            .get_meta("alias")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .or_else(ast_alias);
        Ok(result)
    }

    pub fn set_skip(&self, skip: Option<bool>) -> anyhow::Result<()> {
        self.mode.at_least(NodeRW::LLMOnly)?;

        let value = self.value()?;
        let builder = value.lock().unwrap();
        match skip {
            Some(skip) => {
                builder.with_meta("skip", BamlValue::Bool(skip));
            }
            None => {
                builder.remove_meta("skip");
            }
        }
        Ok(())
    }

    pub fn skip(&self) -> anyhow::Result<bool> {
        self.mode.at_least(NodeRW::ReadOnly)?;

        let ast_skip = || {
            let ir = self.ir_provider.get_ir();
            if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
                if let Some(value) = enm.find_value(&self.value_name) {
                    value.skip(&Default::default()).ok()
                } else {
                    None
                }
            } else {
                None
            }
        };

        let value = self.value()?;
        let builder = value.lock().unwrap();
        let skip = builder
            .get_meta("skip")
            .and_then(|value| value.as_bool())
            .or_else(ast_skip)
            .unwrap_or(false);
        Ok(skip)
    }

    pub fn is_from_ast(&self) -> anyhow::Result<bool> {
        self.mode.at_least(NodeRW::ReadOnly)?;
        let ir = self.ir_provider.get_ir();
        if let Ok(enm) = ir.find_enum(self.enum_name.as_str()) {
            if enm.find_value(&self.value_name).is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

// Runtime-specific extensions for TypeBuilder
impl<IR: RuntimeProvider> TypeBuilder<IR> {
    /// Add entries from parsed BAML to the type builder
    pub fn add_entries(&self, entries: &[TypeBuilderEntry]) {
        for entry in entries {
            match entry {
                TypeBuilderEntry::Class(cls) => {
                    let mutex = self.type_builder.upsert_class(&cls.elem.name);
                    let class_builder = mutex.lock().unwrap();
                    for f in &cls.elem.static_fields {
                        class_builder
                            .upsert_property(&f.elem.name)
                            .lock()
                            .unwrap()
                            .set_type(f.elem.r#type.elem.to_owned())
                            .with_meta(
                                "alias",
                                f.attributes.alias().map_or(BamlValue::Null, |v| {
                                    v.resolve(&EvaluationContext::default())
                                        .map_or(BamlValue::Null, BamlValue::String)
                                }),
                            )
                            .with_meta(
                                "description",
                                f.attributes.description().map_or(BamlValue::Null, |v| {
                                    v.resolve(&EvaluationContext::default())
                                        .map_or(BamlValue::Null, BamlValue::String)
                                }),
                            );
                    }
                }

                TypeBuilderEntry::Enum(enm) => {
                    let mutex = self.type_builder.upsert_enum(&enm.elem.name);
                    let enum_builder = mutex.lock().unwrap();
                    for (variant, _) in &enm.elem.values {
                        enum_builder
                            .upsert_value(&variant.elem.0)
                            .lock()
                            .unwrap()
                            .with_meta(
                                "alias",
                                variant.attributes.alias().map_or(BamlValue::Null, |v| {
                                    v.resolve(&EvaluationContext::default())
                                        .map_or(BamlValue::Null, BamlValue::String)
                                }),
                            )
                            .with_meta(
                                "description",
                                variant
                                    .attributes
                                    .description()
                                    .map_or(BamlValue::Null, |v| {
                                        v.resolve(&EvaluationContext::default())
                                            .map_or(BamlValue::Null, BamlValue::String)
                                    }),
                            )
                            .with_meta(
                                "skip",
                                if variant.attributes.skip() {
                                    BamlValue::Bool(true)
                                } else {
                                    BamlValue::Bool(false)
                                },
                            );
                    }
                }

                TypeBuilderEntry::TypeAlias(alias) => {
                    let mutex = self.type_builder.upsert_type_alias(&alias.elem.name);
                    let alias_builder = mutex.lock().unwrap();
                    alias_builder.target(alias.elem.r#type.elem.to_owned());
                }
            }
        }
    }

    /// Parse and add BAML code to the type builder
    pub fn add_baml(&self, baml: &str) -> anyhow::Result<()> {
        use internal_baml_core::{
            internal_baml_ast::parse_type_builder_contents_from_str,
            internal_baml_diagnostics::{Diagnostics, SourceFile},
            ir::repr::IntermediateRepr,
            run_validation_pipeline_on_db, validate_type_builder_entries,
        };

        let path = std::path::PathBuf::from("TypeBuilder::add_baml");
        let source = SourceFile::from((path.clone(), baml));

        let mut diagnostics = Diagnostics::new(path);
        diagnostics.set_source(&source);

        let type_builder_entries = parse_type_builder_contents_from_str(baml, &mut diagnostics)?;

        if diagnostics.has_errors() {
            anyhow::bail!("{}", diagnostics.to_pretty_string());
        }

        // TODO: A bunch of mem usage here but at least we drop this one at the
        // end of the function, unlike scoped DBs for type builders.
        let mut scoped_db = self.ir_provider.clone_db();

        let local_ast =
            validate_type_builder_entries(&mut diagnostics, &scoped_db, &type_builder_entries);
        scoped_db.add_ast(local_ast);

        if let Err(d) = scoped_db.validate(&mut diagnostics) {
            diagnostics.push(d);
            anyhow::bail!("{}", diagnostics.to_pretty_string());
        }

        run_validation_pipeline_on_db(&mut scoped_db, &mut diagnostics);

        if diagnostics.has_errors() {
            anyhow::bail!("{}", diagnostics.to_pretty_string());
        }

        let (classes, enums, type_aliases, recursive_classes, recursive_aliases) =
            IntermediateRepr::type_builder_entries_from_scoped_db(
                &scoped_db,
                &self.ir_provider.get_db(),
            )
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        self.add_entries(
            &classes
                .into_iter()
                .map(TypeBuilderEntry::Class)
                .chain(enums.into_iter().map(TypeBuilderEntry::Enum))
                .chain(type_aliases.into_iter().map(TypeBuilderEntry::TypeAlias))
                .collect::<Vec<_>>(),
        );

        self.type_builder
            .recursive_type_aliases()
            .lock()
            .unwrap()
            .extend(recursive_aliases);

        self.type_builder
            .recursive_classes()
            .lock()
            .unwrap()
            .extend(recursive_classes);

        Ok(())
    }
}
