use baml_types::ir_type::UnionConstructor;
use pyo3::{
    prelude::PyAnyMethods,
    pymethods,
    types::{PyTuple, PyTupleMethods},
    Bound, PyResult,
};

use crate::{errors::BamlError, runtime::BamlRuntime};

crate::lang_wrapper!(
    TypeBuilder,
    baml_type_builder::TypeBuilder<baml_runtime::runtime::InternalBamlRuntime>
);
crate::lang_wrapper!(
    EnumBuilder,
    baml_type_builder::EnumBuilder<baml_runtime::runtime::InternalBamlRuntime>
);
crate::lang_wrapper!(
    ClassBuilder,
    baml_type_builder::ClassBuilder<baml_runtime::runtime::InternalBamlRuntime>
);
crate::lang_wrapper!(
    EnumValueBuilder,
    baml_type_builder::EnumValueBuilder<baml_runtime::runtime::InternalBamlRuntime>
);
crate::lang_wrapper!(
    ClassPropertyBuilder,
    baml_type_builder::ClassPropertyBuilder<baml_runtime::runtime::InternalBamlRuntime>
);
crate::lang_wrapper!(FieldType, baml_types::TypeIR);

#[pymethods]
impl TypeBuilder {
    #[new]
    pub fn new(runtime: &BamlRuntime) -> Self {
        baml_type_builder::TypeBuilder::new(runtime.inner.internal().clone()).into()
    }

    pub fn reset(&self) {
        self.inner.reset()
    }

    /// provides a detailed string representation of the typebuilder for python users.
    ///
    /// this method exposes the rust-implemented string formatting to python, ensuring
    /// consistent and professional output across both languages. the representation
    /// includes a complete view of:
    ///
    /// * all defined classes with their properties
    /// * all defined enums with their values
    /// * metadata such as aliases and descriptions
    /// * type information for properties
    ///
    /// the output format is carefully structured for readability, making it quite easy :D
    /// to understand the complete type hierarchy at a glance.
    pub fn __str__(&self) -> String {
        todo!("implement this")
    }

    pub fn r#enum(&self, name: &str) -> PyResult<EnumBuilder> {
        let result = self.inner.add_enum(name).map_err(BamlError::from_anyhow)?;
        Ok(EnumBuilder { inner: result })
    }

    // Rename to "class_"
    #[pyo3(name = "class_")]
    pub fn class(&self, name: &str) -> PyResult<ClassBuilder> {
        let result = self.inner.add_class(name).map_err(BamlError::from_anyhow)?;
        Ok(ClassBuilder { inner: result })
    }

    pub fn literal_string(&self, value: &str) -> FieldType {
        baml_types::TypeIR::literal_string(value.to_string()).into()
    }

    pub fn literal_int(&self, value: i64) -> FieldType {
        baml_types::TypeIR::literal_int(value).into()
    }

    pub fn literal_bool(&self, value: bool) -> FieldType {
        baml_types::TypeIR::literal_bool(value).into()
    }

    pub fn list(&self, inner: &FieldType) -> FieldType {
        inner.inner.clone().as_list().into()
    }

    pub fn optional(&self, inner: &FieldType) -> FieldType {
        inner.inner.clone().as_optional().into()
    }

    pub fn string(&self) -> FieldType {
        baml_types::TypeIR::string().into()
    }

    pub fn int(&self) -> FieldType {
        baml_types::TypeIR::int().into()
    }

    pub fn float(&self) -> FieldType {
        baml_types::TypeIR::float().into()
    }

    pub fn bool(&self) -> FieldType {
        baml_types::TypeIR::bool().into()
    }

    pub fn null(&self) -> FieldType {
        baml_types::TypeIR::null().into()
    }

    pub fn map(&self, key: &FieldType, value: &FieldType) -> FieldType {
        baml_types::TypeIR::map(key.inner.clone(), value.inner.clone()).into()
    }

    #[pyo3(signature = (*types))]
    pub fn union(&self, types: &Bound<'_, PyTuple>) -> PyResult<FieldType> {
        let mut rs_types = vec![];
        for idx in 0..types.len() {
            let item = types.get_item(idx)?;
            let item = item.downcast::<FieldType>()?;
            rs_types.push(item.borrow().inner.clone());
        }
        Ok(baml_types::TypeIR::union(rs_types).into())
    }

    pub fn add_baml(&self, baml: &str) -> PyResult<()> {
        self.inner.add_baml(baml).map_err(BamlError::from_anyhow)
    }
}

#[pymethods]
impl FieldType {
    pub fn list(&self) -> FieldType {
        self.inner.clone().as_list().into()
    }

    pub fn optional(&self) -> FieldType {
        self.inner.clone().as_optional().into()
    }

    pub fn __eq__(&self, other: &FieldType) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl EnumBuilder {
    pub fn value(&self, value: &str) -> PyResult<EnumValueBuilder> {
        let result = self
            .inner
            .add_value(value)
            .map_err(BamlError::from_anyhow)?;
        Ok(EnumValueBuilder { inner: result })
    }

    #[pyo3(signature = (alias = None))]
    pub fn alias(&self, alias: Option<&str>) -> PyResult<Self> {
        self.inner
            .set_alias(alias)
            .map_err(BamlError::from_anyhow)?;
        Ok(self.inner.clone().into())
    }

    pub fn field(&self) -> FieldType {
        baml_types::TypeIR::r#enum(&self.inner.enum_name).into()
    }
}

#[pymethods]
impl EnumValueBuilder {
    #[pyo3(signature = (alias = None))]
    pub fn alias(&self, alias: Option<&str>) -> PyResult<Self> {
        self.inner
            .set_alias(alias)
            .map_err(BamlError::from_anyhow)?;
        Ok(self.inner.clone().into())
    }

    #[pyo3(signature = (skip = true))]
    pub fn skip(&self, skip: Option<bool>) -> PyResult<Self> {
        self.inner.set_skip(skip).map_err(BamlError::from_anyhow)?;
        Ok(self.inner.clone().into())
    }

    #[pyo3(signature = (description = None))]
    pub fn description(&self, description: Option<&str>) -> PyResult<Self> {
        self.inner
            .set_description(description)
            .map_err(BamlError::from_anyhow)?;
        Ok(self.inner.clone().into())
    }
}

#[pymethods]
impl ClassBuilder {
    pub fn field(&self) -> FieldType {
        baml_types::TypeIR::class(&self.inner.class_name).into()
    }

    pub fn list_properties(&self) -> PyResult<Vec<(String, ClassPropertyBuilder)>> {
        Ok(self
            .inner
            .list_properties()
            .map_err(BamlError::from_anyhow)?
            .into_iter()
            .map(|prop| {
                (
                    prop.property_name.clone(),
                    ClassPropertyBuilder { inner: prop },
                )
            })
            .collect())
    }

    pub fn remove_property(&self, name: &str) -> PyResult<()> {
        self.inner
            .remove_property(name)
            .map_err(BamlError::from_anyhow)
    }

    pub fn reset(&self) -> PyResult<()> {
        self.inner.reset().map_err(BamlError::from_anyhow)
    }

    pub fn property(&self, name: &str) -> PyResult<ClassPropertyBuilder> {
        let result = self.inner.property(name).map_err(BamlError::from_anyhow)?;
        Ok(ClassPropertyBuilder { inner: result })
    }
}

#[pymethods]
impl ClassPropertyBuilder {
    pub fn r#type(&self, r#type: &FieldType) -> PyResult<Self> {
        self.inner
            .set_type(r#type.inner.clone())
            .map_err(BamlError::from_anyhow)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    pub fn get_type(&self) -> PyResult<FieldType> {
        Ok(FieldType {
            inner: self.inner.type_().map_err(BamlError::from_anyhow)?,
        })
    }

    #[pyo3(signature = (alias = None))]
    pub fn alias(&self, alias: Option<&str>) -> PyResult<Self> {
        self.inner
            .set_alias(alias)
            .map_err(BamlError::from_anyhow)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(signature = (description = None))]
    pub fn description(&self, description: Option<&str>) -> PyResult<Self> {
        self.inner
            .set_description(description)
            .map_err(BamlError::from_anyhow)?;
        Ok(Self {
            inner: self.inner.clone(),
        })
    }
}
