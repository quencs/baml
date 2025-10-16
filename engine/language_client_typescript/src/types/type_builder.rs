use std::{collections::BTreeMap, ops::Deref};

// This file provides the native bindings between our Rust implementation and TypeScript
// We use NAPI-RS to expose Rust functionality to JavaScript/TypeScript
use baml_type_builder::{
    ClassBuilder as BamlClassBuilder, ClassPropertyBuilder as BamlClassPropertyBuilder,
    EnumBuilder as BamlEnumBuilder, EnumValueBuilder as BamlEnumValueBuilder,
    TypeBuilder as BamlTypeBuilder,
};
use baml_types::{ir_type::UnionConstructor, BamlValue};
use napi::{
    bindgen_prelude::{Array, JavaScriptClassExt},
    Env,
};
use napi_derive::napi;

// Create TypeScript-compatible wrappers for our Rust types
// These macros generate the necessary code for TypeScript interop
crate::lang_wrapper!(
    TypeBuilder,
    BamlTypeBuilder<baml_runtime::runtime::InternalBamlRuntime>
);

// Thread-safe wrapper for EnumBuilder
crate::lang_wrapper!(
    EnumBuilder,
    BamlEnumBuilder<baml_runtime::runtime::InternalBamlRuntime>
);

// Thread-safe wrapper for ClassBuilder
crate::lang_wrapper!(
    ClassBuilder,
    BamlClassBuilder<baml_runtime::runtime::InternalBamlRuntime>
);

// Thread-safe wrapper for EnumValueBuilder
crate::lang_wrapper!(
    EnumValueBuilder,
    BamlEnumValueBuilder<baml_runtime::runtime::InternalBamlRuntime>
);

// Thread-safe wrapper for ClassPropertyBuilder
crate::lang_wrapper!(
    ClassPropertyBuilder,
    BamlClassPropertyBuilder<baml_runtime::runtime::InternalBamlRuntime>
);

// Thread-safe wrapper for FieldType
// Core type system representation with thread-safety guarantees
crate::lang_wrapper!(FieldType, baml_types::TypeIR, sync_thread_safe);

// note: you may notice a rust-analyzer warning in vs code when working with this file.
// the warning "did not find struct napitypebuilder parsed before expand #[napi] for impl"
// is a known false positive that occurs due to how rust-analyzer processes macro state.
//
// don't worry - the code compiles and works correctly! the warning is yet to be addressed by napi maintainers.
//
// if you'd like to hide this warning in vs code, you can add this to your settings.json:
//   "rust-analyzer.diagnostics.disabled": ["macro-error"]
//
// ref:
// https://github.com/napi-rs/napi-rs/issues/1630
// A complex struct that cannot be exposed to JavaScript directly.
#[napi]
impl TypeBuilder {
    #[napi(factory)]
    pub fn new(runtime: &crate::BamlRuntime) -> Self {
        let tb = BamlTypeBuilder::new(runtime.inner.internal().clone());
        tb.into()
    }

    #[napi]
    pub fn reset(&self) {
        self.inner.reset();
    }

    #[napi]
    pub fn add_enum(&self, name: String) -> napi::Result<EnumBuilder> {
        let result = self
            .inner
            .add_enum(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(EnumBuilder { inner: result })
    }

    #[napi]
    pub fn get_enum(&self, name: String) -> napi::Result<EnumBuilder> {
        let result = self
            .inner
            .r#enum(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(EnumBuilder { inner: result })
    }

    #[napi]
    pub fn add_class(&self, name: String) -> napi::Result<ClassBuilder> {
        let result = self
            .inner
            .add_class(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(ClassBuilder { inner: result })
    }

    #[napi]
    pub fn get_class(&self, name: String) -> napi::Result<ClassBuilder> {
        let result = self
            .inner
            .class(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(ClassBuilder { inner: result })
    }

    #[napi]
    pub fn list(&self, inner: &FieldType) -> FieldType {
        inner.inner.lock().unwrap().clone().as_list().into()
    }

    #[napi]
    pub fn optional(&self, inner: &FieldType) -> FieldType {
        inner.inner.lock().unwrap().clone().as_optional().into()
    }

    #[napi]
    pub fn string(&self) -> FieldType {
        baml_types::TypeIR::string().into()
    }

    #[napi]
    pub fn literal_string(&self, value: String) -> FieldType {
        baml_types::TypeIR::literal_string(value).into()
    }

    #[napi]
    pub fn literal_int(&self, value: i64) -> FieldType {
        baml_types::TypeIR::literal_int(value).into()
    }

    #[napi]
    pub fn literal_bool(&self, value: bool) -> FieldType {
        baml_types::TypeIR::literal_bool(value).into()
    }

    #[napi]
    pub fn int(&self) -> FieldType {
        baml_types::TypeIR::int().into()
    }

    #[napi]
    pub fn float(&self) -> FieldType {
        baml_types::TypeIR::float().into()
    }

    #[napi]
    pub fn bool(&self) -> FieldType {
        baml_types::TypeIR::bool().into()
    }

    #[napi]
    pub fn null(&self) -> FieldType {
        baml_types::TypeIR::null().into()
    }

    #[napi]
    pub fn map(&self, key: &FieldType, value: &FieldType) -> FieldType {
        baml_types::TypeIR::map(
            key.inner.lock().unwrap().clone(),
            value.inner.lock().unwrap().clone(),
        )
        .into()
    }

    #[napi]
    pub fn union(&self, types: Vec<&FieldType>) -> FieldType {
        baml_types::TypeIR::union(
            types
                .iter()
                .map(|t| t.inner.lock().unwrap().clone())
                .collect(),
        )
        .into()
    }

    #[napi]
    pub fn add_baml(&self, baml: String) -> napi::Result<()> {
        self.inner
            .add_baml(&baml)
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

#[napi]
impl FieldType {
    #[napi]
    pub fn list(&self) -> FieldType {
        self.inner.lock().unwrap().clone().as_list().into()
    }

    #[napi]
    pub fn optional(&self) -> FieldType {
        self.inner.lock().unwrap().clone().as_optional().into()
    }

    #[napi]
    pub fn equals(&self, other: &FieldType) -> bool {
        self.inner.lock().unwrap().deref() == other.inner.lock().unwrap().deref()
    }
}

#[napi]
impl EnumBuilder {
    #[napi]
    pub fn add_value(&self, name: String) -> napi::Result<EnumValueBuilder> {
        let result = self
            .inner
            .add_value(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(EnumValueBuilder { inner: result })
    }

    #[napi]
    pub fn get_value(&self, name: String) -> napi::Result<EnumValueBuilder> {
        let result = self
            .inner
            .value(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(EnumValueBuilder { inner: result })
    }

    #[napi]
    pub fn remove_value(&self, name: String) -> napi::Result<()> {
        self.inner
            .remove_value(&name)
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn set_alias(&self, alias: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_alias(alias.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn set_description(&self, description: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_description(description.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn alias(&self) -> napi::Result<Option<String>> {
        self.inner.alias().map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn description(&self) -> napi::Result<Option<String>> {
        self.inner
            .description()
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn field(&self) -> FieldType {
        baml_types::TypeIR::r#enum(&self.inner.enum_name).into()
    }

    #[napi(ts_return_type = "Array<[string, EnumValueBuilder]>")]
    pub fn list_values<'e>(&self, env: &'e Env) -> napi::Result<Array<'e>> {
        let values = self
            .inner
            .list_values()
            .map_err(crate::errors::from_anyhow_error)?
            .into_iter()
            .map(|v| (v.value_name.clone(), EnumValueBuilder { inner: v }));

        let mut js_array = env.create_array(values.len() as u32)?;
        for (i, (name, val_builder)) in values.enumerate() {
            let mut tuple = env.create_array(2)?;
            tuple.set(0, env.create_string(&name)?)?;
            tuple.set(1, val_builder.into_instance(env)?)?;
            js_array.set(i as u32, tuple)?;
        }
        Ok(js_array)
    }

    #[napi(ts_return_type = "'baml' | 'dynamic'")]
    pub fn source(&self) -> napi::Result<String> {
        let from_ast = self
            .inner
            .is_from_ast()
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(if from_ast {
            "baml".to_string()
        } else {
            "dynamic".to_string()
        })
    }
}

#[napi]
impl EnumValueBuilder {
    #[napi]
    pub fn set_alias(&self, alias: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_alias(alias.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn set_skip(&self, skip: Option<bool>) -> napi::Result<Self> {
        self.inner
            .set_skip(skip)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn set_description(&self, description: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_description(description.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn alias(&self) -> napi::Result<Option<String>> {
        self.inner.alias().map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn description(&self) -> napi::Result<Option<String>> {
        self.inner
            .description()
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn skip(&self) -> napi::Result<bool> {
        self.inner.skip().map_err(crate::errors::from_anyhow_error)
    }

    #[napi(ts_return_type = "'baml' | 'dynamic'")]
    pub fn source(&self) -> napi::Result<String> {
        let from_ast = self
            .inner
            .is_from_ast()
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(if from_ast {
            "baml".to_string()
        } else {
            "dynamic".to_string()
        })
    }
}

#[napi]
impl ClassBuilder {
    #[napi]
    pub fn field(&self) -> FieldType {
        baml_types::TypeIR::class(&self.inner.class_name).into()
    }

    #[napi(ts_return_type = "Array<[string, ClassPropertyBuilder]>")]
    pub fn list_properties<'e>(&self, env: &'e Env) -> napi::Result<Array<'e>> {
        let properties = self
            .inner
            .list_properties()
            .map_err(crate::errors::from_anyhow_error)?
            .into_iter()
            .map(|prop| {
                (
                    prop.property_name.clone(),
                    ClassPropertyBuilder { inner: prop },
                )
            });

        let mut js_array = env.create_array(properties.len() as u32)?;

        for (i, (name, prop_builder)) in properties.enumerate() {
            let mut tuple = env.create_array(2)?;
            tuple.set(0, env.create_string(&name)?)?;
            tuple.set(1, prop_builder.into_instance(&env)?)?;
            js_array.set(i as u32, tuple)?;
        }

        Ok(js_array)
    }

    #[napi]
    pub fn remove_property(&self, name: String) -> napi::Result<()> {
        self.inner
            .remove_property(&name)
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn reset(&self) -> napi::Result<()> {
        self.inner.reset().map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn get_property(&self, name: String) -> napi::Result<ClassPropertyBuilder> {
        let result = self
            .inner
            .property(&name)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(ClassPropertyBuilder { inner: result })
    }

    #[napi]
    pub fn add_property(
        &self,
        name: String,
        field_type: &FieldType,
    ) -> napi::Result<ClassPropertyBuilder> {
        let result = self
            .inner
            .add_property(&name, field_type.inner.lock().unwrap().clone(), true)
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(ClassPropertyBuilder { inner: result })
    }

    #[napi]
    pub fn set_alias(&self, alias: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_alias(alias.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn set_description(&self, description: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_description(description.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(self.inner.clone().into())
    }

    #[napi]
    pub fn alias(&self) -> napi::Result<Option<String>> {
        self.inner.alias().map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn description(&self) -> napi::Result<Option<String>> {
        self.inner
            .description()
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi(ts_return_type = "'baml' | 'dynamic'")]
    pub fn source(&self) -> napi::Result<String> {
        let from_ast = self
            .inner
            .is_from_ast()
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(if from_ast {
            "baml".to_string()
        } else {
            "dynamic".to_string()
        })
    }
}

#[napi]
impl ClassPropertyBuilder {
    #[napi]
    pub fn set_type(&self, field_type: &FieldType) -> napi::Result<Self> {
        self.inner
            .set_type(field_type.inner.lock().unwrap().clone())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[napi]
    pub fn get_type(&self) -> napi::Result<FieldType> {
        Ok(FieldType {
            inner: std::sync::Arc::new(std::sync::Mutex::new(
                self.inner
                    .type_()
                    .map_err(crate::errors::from_anyhow_error)?,
            )),
        })
    }

    #[napi]
    pub fn set_alias(&self, alias: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_alias(alias.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[napi]
    pub fn set_description(&self, description: Option<String>) -> napi::Result<Self> {
        self.inner
            .set_description(description.as_deref())
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[napi]
    pub fn alias(&self) -> napi::Result<Option<String>> {
        self.inner.alias().map_err(crate::errors::from_anyhow_error)
    }

    #[napi]
    pub fn description(&self) -> napi::Result<Option<String>> {
        self.inner
            .description()
            .map_err(crate::errors::from_anyhow_error)
    }

    #[napi(ts_return_type = "'baml' | 'dynamic'")]
    pub fn source(&self) -> napi::Result<String> {
        let from_ast = self
            .inner
            .is_from_ast()
            .map_err(crate::errors::from_anyhow_error)?;
        Ok(if from_ast {
            "baml".to_string()
        } else {
            "dynamic".to_string()
        })
    }
}
