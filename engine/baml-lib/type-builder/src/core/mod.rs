use std::{
    fmt,
    ops::Deref,
    sync::{Arc, Mutex},
};

use baml_types::{BamlValue, TypeIR};
use indexmap::{IndexMap, IndexSet};

// Meta system for storing metadata on builders
type MetaData = Arc<Mutex<IndexMap<String, BamlValue>>>;

pub trait Meta {
    fn meta(&self) -> MetaData;
}

pub trait WithMeta {
    fn with_meta(&self, key: &str, value: BamlValue) -> &Self;
    fn get_meta(&self, key: &str) -> Option<BamlValue>;
    fn remove_meta(&self, key: &str) -> &Self;
}

macro_rules! impl_meta {
    ($type:ty) => {
        impl Meta for $type {
            fn meta(&self) -> MetaData {
                self.meta.clone()
            }
        }
    };
}

impl<T> WithMeta for T
where
    T: Meta,
{
    fn with_meta(&self, key: &str, value: BamlValue) -> &T {
        let meta = self.meta();
        let mut meta = meta.lock().unwrap();
        meta.insert(key.to_string(), value);
        self
    }

    fn remove_meta(&self, key: &str) -> &T {
        let meta = self.meta();
        let mut meta = meta.lock().unwrap();
        meta.shift_remove(key);
        self
    }

    fn get_meta(&self, key: &str) -> Option<BamlValue> {
        let meta = self.meta();
        let meta = meta.lock().unwrap();
        meta.get(key).cloned()
    }
}

// Core TypeBuilder without IR dependencies
#[derive(Clone)]
pub struct TypeBuilder {
    classes: Arc<Mutex<IndexMap<String, Arc<Mutex<ClassBuilder>>>>>,
    enums: Arc<Mutex<IndexMap<String, Arc<Mutex<EnumBuilder>>>>>,
    type_aliases: Arc<Mutex<IndexMap<String, Arc<Mutex<TypeAliasBuilder>>>>>,
    recursive_type_aliases: Arc<Mutex<Vec<IndexMap<String, TypeIR>>>>,
    recursive_classes: Arc<Mutex<Vec<IndexSet<String>>>>,
}

impl Default for TypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeBuilder {
    pub fn new() -> Self {
        Self {
            classes: Default::default(),
            enums: Default::default(),
            type_aliases: Default::default(),
            recursive_type_aliases: Default::default(),
            recursive_classes: Default::default(),
        }
    }

    pub fn reset(&self) {
        self.classes.lock().unwrap().clear();
        self.enums.lock().unwrap().clear();
        self.type_aliases.lock().unwrap().clear();
        self.recursive_type_aliases.lock().unwrap().clear();
        self.recursive_classes.lock().unwrap().clear();
    }

    pub fn upsert_class(&self, name: &str) -> Arc<Mutex<ClassBuilder>> {
        Arc::clone(
            self.classes
                .lock()
                .unwrap()
                .entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(ClassBuilder::new(name.to_string())))),
        )
    }

    pub fn maybe_get_class(&self, name: &str) -> Option<Arc<Mutex<ClassBuilder>>> {
        self.classes.lock().unwrap().get(name).cloned()
    }

    pub fn upsert_enum(&self, name: &str) -> Arc<Mutex<EnumBuilder>> {
        Arc::clone(
            self.enums
                .lock()
                .unwrap()
                .entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(EnumBuilder::new(name.to_string())))),
        )
    }

    pub fn list_enums(&self) -> Vec<String> {
        self.enums.lock().unwrap().keys().cloned().collect()
    }

    pub fn list_classes(&self) -> Vec<String> {
        self.classes.lock().unwrap().keys().cloned().collect()
    }

    pub fn maybe_get_enum(&self, name: &str) -> Option<Arc<Mutex<EnumBuilder>>> {
        self.enums.lock().unwrap().get(name).cloned()
    }

    pub fn upsert_type_alias(&self, name: &str) -> Arc<Mutex<TypeAliasBuilder>> {
        Arc::clone(
            self.type_aliases
                .lock()
                .unwrap()
                .entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(TypeAliasBuilder::new()))),
        )
    }

    pub fn maybe_get_type_alias(&self, name: &str) -> Option<Arc<Mutex<TypeAliasBuilder>>> {
        self.type_aliases.lock().unwrap().get(name).cloned()
    }

    pub fn recursive_type_aliases(&self) -> Arc<Mutex<Vec<IndexMap<String, TypeIR>>>> {
        Arc::clone(&self.recursive_type_aliases)
    }

    pub fn recursive_classes(&self) -> Arc<Mutex<Vec<IndexSet<String>>>> {
        Arc::clone(&self.recursive_classes)
    }
}

// ClassBuilder
#[derive(Debug, Clone)]
pub struct ClassBuilder {
    pub name: String,
    properties: Arc<Mutex<IndexMap<String, Arc<Mutex<ClassPropertyBuilder>>>>>,
    meta: MetaData,
}
impl_meta!(ClassBuilder);

impl ClassBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            properties: Default::default(),
            meta: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn list_properties_key_value(&self) -> Vec<(String, ClassPropertyBuilder)> {
        self.properties
            .lock()
            .unwrap()
            .iter()
            .map(|(name, prop)| (name.clone(), prop.lock().unwrap().deref().to_owned()))
            .collect()
    }

    pub fn list_properties(&self) -> Vec<String> {
        let properties = self.properties.lock().unwrap();
        properties.keys().cloned().collect()
    }

    pub fn maybe_get_property(&self, name: &str) -> Option<Arc<Mutex<ClassPropertyBuilder>>> {
        let properties = self.properties.lock().unwrap();
        properties.get(name).cloned()
    }

    pub fn upsert_property(&self, name: &str) -> Arc<Mutex<ClassPropertyBuilder>> {
        let mut properties = self.properties.lock().unwrap();
        Arc::clone(properties.entry(name.to_string()).or_insert_with(|| {
            Arc::new(Mutex::new(ClassPropertyBuilder {
                r#type: Default::default(),
                meta: Default::default(),
            }))
        }))
    }

    pub fn remove_property(&self, name: &str) {
        let mut properties = self.properties.lock().unwrap();
        properties.shift_remove(name);
    }

    pub fn reset(&self) {
        self.properties.lock().unwrap().clear();
    }
}

// ClassPropertyBuilder
#[derive(Debug, Clone)]
pub struct ClassPropertyBuilder {
    r#type: Arc<Mutex<Option<TypeIR>>>,
    meta: MetaData,
}
impl_meta!(ClassPropertyBuilder);

impl ClassPropertyBuilder {
    pub fn set_type(&self, r#type: TypeIR) -> &Self {
        *self.r#type.lock().unwrap() = Some(r#type);
        self
    }

    pub fn r#type(&self) -> Option<TypeIR> {
        self.r#type.lock().unwrap().clone()
    }
}

// EnumBuilder
#[derive(Debug, Clone)]
pub struct EnumBuilder {
    pub name: String,
    values: Arc<Mutex<IndexMap<String, Arc<Mutex<EnumValueBuilder>>>>>,
    meta: MetaData,
}
impl_meta!(EnumBuilder);

impl EnumBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: Default::default(),
            meta: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn maybe_get_value(&self, name: &str) -> Option<Arc<Mutex<EnumValueBuilder>>> {
        let values = self.values.lock().unwrap();
        values.get(name).cloned()
    }

    pub fn upsert_value(&self, name: &str) -> Arc<Mutex<EnumValueBuilder>> {
        let mut values = self.values.lock().unwrap();
        Arc::clone(values.entry(name.to_string()).or_insert_with(|| {
            Arc::new(Mutex::new(EnumValueBuilder {
                meta: Default::default(),
            }))
        }))
    }

    pub fn list_values(&self) -> Vec<String> {
        let values = self.values.lock().unwrap();
        values.keys().cloned().collect()
    }
}

// EnumValueBuilder
#[derive(Debug, Clone)]
pub struct EnumValueBuilder {
    meta: MetaData,
}
impl_meta!(EnumValueBuilder);

// TypeAliasBuilder
pub struct TypeAliasBuilder {
    target: Arc<Mutex<Option<TypeIR>>>,
    meta: MetaData,
}
impl_meta!(TypeAliasBuilder);

impl TypeAliasBuilder {
    pub fn new() -> Self {
        Self {
            target: Default::default(),
            meta: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn target(&self, target: TypeIR) -> &Self {
        *self.target.lock().unwrap() = Some(target);
        self
    }
}

impl Default for TypeAliasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Display implementations
impl fmt::Display for ClassPropertyBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let meta = self.meta.lock().unwrap();
        let type_str = self
            .r#type
            .lock()
            .unwrap()
            .as_ref()
            .map_or("(unknown-type)".to_string(), |t| format!("{}", t.clone()));

        write!(f, "{type_str}")?;

        if !meta.is_empty() {
            write!(f, " (")?;
            for (i, (key, value)) in meta.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{key}={value}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl fmt::Display for EnumValueBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let meta = self.meta.lock().unwrap();

        if !meta.is_empty() {
            write!(f, " (")?;
            for (i, (key, value)) in meta.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{key}={value}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl fmt::Display for ClassBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let properties = self.properties.lock().unwrap();
        write!(f, "{{")?;
        if !properties.is_empty() {
            for (i, (name, prop)) in properties.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "\n      {} {}", name, prop.lock().unwrap())?;
            }
            write!(f, "\n    ")?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for EnumBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values = self.values.lock().unwrap();
        write!(f, "{{")?;
        if !values.is_empty() {
            for (i, (name, value)) in values.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "\n      {}{}", name, value.lock().unwrap())?;
            }
            write!(f, "\n    ")?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for TypeBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let classes = self.classes.lock().unwrap();
        let enums = self.enums.lock().unwrap();
        let type_aliases = self.type_aliases.lock().unwrap();
        let recursive_type_aliases = self.recursive_type_aliases.lock().unwrap();

        write!(f, "TypeBuilder(")?;

        if !classes.is_empty() {
            write!(f, "\n  Classes: [")?;
            for (i, (name, cls)) in classes.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "\n    {} {}", name, cls.lock().unwrap())?;
            }
            write!(f, "\n  ]")?;
        }

        if !type_aliases.is_empty() {
            write!(f, "\n  type_aliases: ")?;

            match self.type_aliases.lock() {
                Ok(type_aliases) => {
                    let keys: Vec<_> = type_aliases.keys().collect();
                    writeln!(f, "{keys:?}")?
                }
                Err(_) => writeln!(f, "Cannot acquire lock,")?,
            }
        }

        if !recursive_type_aliases.is_empty() {
            write!(f, "\n  recursive_type_aliases: ")?;

            match self.recursive_type_aliases.lock() {
                Ok(recursive_type_aliases) => {
                    let keys: Vec<_> = recursive_type_aliases.iter().map(|v| v.keys()).collect();
                    writeln!(f, "{keys:?}")?
                }
                Err(_) => writeln!(f, "Cannot acquire lock,")?,
            }
        }

        if !enums.is_empty() {
            if !classes.is_empty() {
                write!(f, ",")?;
            }
            write!(f, "\n  Enums: [")?;
            for (i, (name, e)) in enums.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, "\n    {} {}", name, e.lock().unwrap())?;
            }
            write!(f, "\n  ]")?;
        }

        write!(f, "\n)")
    }
}

impl std::fmt::Debug for TypeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TypeBuilder {{")?;

        write!(f, "  classes: ")?;
        match self.classes.lock() {
            Ok(classes) => {
                let keys: Vec<_> = classes.keys().collect();
                writeln!(f, "{keys:?},")?
            }
            Err(_) => writeln!(f, "Cannot acquire lock,")?,
        }

        write!(f, "  enums: ")?;
        match self.enums.lock() {
            Ok(enums) => {
                let keys: Vec<_> = enums.keys().collect();
                writeln!(f, "{keys:?}")?
            }
            Err(_) => writeln!(f, "Cannot acquire lock,")?,
        }

        write!(f, "}}")
    }
}
