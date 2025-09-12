use baml_cffi_macros::export_baml_fn;
use baml_types::{ir_type::UnionConstructor, BamlValue, TypeIR};

use super::{BamlObjectResponse, BamlObjectResponseSuccess, CallMethod};
use crate::raw_ptr_wrapper::{
    ClassBuilderWrapper, ClassPropertyBuilderWrapper, EnumBuilderWrapper, EnumValueBuilderWrapper,
    TypeBuilderWrapper, TypeWrapper,
};

#[export_baml_fn]
impl TypeBuilderWrapper {
    #[export_baml_fn]
    fn string(&self) -> TypeIR {
        TypeIR::string()
    }

    #[export_baml_fn]
    fn int(&self) -> TypeIR {
        TypeIR::int()
    }

    #[export_baml_fn]
    fn float(&self) -> TypeIR {
        TypeIR::float()
    }

    #[export_baml_fn]
    fn bool(&self) -> TypeIR {
        TypeIR::bool()
    }

    #[export_baml_fn]
    fn literal_string(&self, value: &str) -> TypeIR {
        TypeIR::literal_string(value.to_string())
    }

    #[export_baml_fn]
    fn literal_int(&self, value: i64) -> TypeIR {
        TypeIR::literal_int(value)
    }

    #[export_baml_fn]
    fn literal_bool(&self, value: bool) -> TypeIR {
        TypeIR::literal_bool(value)
    }

    #[export_baml_fn]
    fn null(&self) -> TypeIR {
        TypeIR::null()
    }

    #[export_baml_fn]
    fn map(&self, key: &TypeWrapper, value: &TypeWrapper) -> TypeIR {
        TypeIR::map(key.as_ref().clone(), value.as_ref().clone())
    }

    #[export_baml_fn]
    fn list(&self, inner: &TypeWrapper) -> TypeIR {
        TypeIR::list(inner.as_ref().clone())
    }

    #[export_baml_fn]
    fn optional(&self, inner: &TypeWrapper) -> TypeIR {
        TypeIR::optional(inner.as_ref().clone())
    }

    #[export_baml_fn]
    #[allow(clippy::ptr_arg)]
    fn union(&self, types: &Vec<TypeWrapper>) -> TypeIR {
        TypeIR::union(types.iter().map(|t| t.as_ref().clone()).collect())
    }

    #[export_baml_fn]
    fn add_baml(&self, baml: &str) -> Result<(), String> {
        self.inner.add_baml(baml).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn add_enum(&self, name: &str) -> Result<EnumBuilderWrapper, String> {
        self.inner.add_enum(name).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn add_class(&self, name: &str) -> Result<ClassBuilderWrapper, String> {
        self.inner
            .add_class(name)
            .map(ClassBuilderWrapper::from_object)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn class(&self, name: &str) -> Result<ClassBuilderWrapper, String> {
        self.inner
            .class(name)
            .map(ClassBuilderWrapper::from_object)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn enum_(&self, name: &str) -> Result<objects::EnumBuilder, String> {
        self.inner.r#enum(name).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn list_enums(&self) -> Vec<objects::EnumBuilder> {
        self.inner.list_enums()
    }

    #[export_baml_fn]
    fn list_classes(&self) -> Vec<objects::ClassBuilder> {
        self.inner.list_classes()
    }

    #[export_baml_fn]
    fn __display__(&self) -> String {
        self.inner.to_string()
    }
}

#[export_baml_fn]
impl TypeWrapper {
    #[export_baml_fn]
    fn list(&self) -> TypeIR {
        self.as_ref().clone().as_list()
    }

    #[export_baml_fn]
    fn optional(&self) -> TypeIR {
        self.as_ref().clone().as_optional()
    }

    #[export_baml_fn]
    fn __display__(&self) -> String {
        self.as_ref().to_string()
    }
}

#[export_baml_fn]
impl EnumBuilderWrapper {
    #[export_baml_fn]
    fn name(&self) -> String {
        self.inner.enum_name.to_string()
    }

    #[export_baml_fn]
    fn add_value(&self, value: &str) -> Result<objects::EnumValueBuilder, String> {
        self.inner.add_value(value).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_description(&self, description: &str) -> Result<(), String> {
        self.inner
            .set_description(description)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_alias(&self, alias: &str) -> Result<(), String> {
        self.inner.set_alias(alias).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn description(&self) -> Result<Option<String>, String> {
        self.inner.description().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn alias(&self) -> Result<Option<String>, String> {
        self.inner.alias().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn type_(&self) -> Result<TypeIR, String> {
        self.inner.r#type().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn list_values(&self) -> Result<Vec<objects::EnumValueBuilder>, String> {
        self.inner
            .list_values()
            .map(|builders| builders.into_iter().collect())
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn value(&self, name: &str) -> Result<EnumValueBuilderWrapper, String> {
        self.inner
            .value(name)
            .map(EnumValueBuilderWrapper::from_object)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn is_from_ast(&self) -> Result<bool, String> {
        self.inner.is_from_ast().map_err(|e| e.to_string())
    }
}

#[export_baml_fn]
impl EnumValueBuilderWrapper {
    #[export_baml_fn]
    fn name(&self) -> String {
        self.inner.value_name.to_string()
    }

    #[export_baml_fn]
    fn set_description(&self, description: &str) -> Result<(), String> {
        self.inner
            .set_description(description)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_alias(&self, alias: &str) -> Result<(), String> {
        self.inner.set_alias(alias).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn description(&self) -> Result<Option<String>, String> {
        self.inner.description().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn alias(&self) -> Result<Option<String>, String> {
        self.inner.alias().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_skip(&self, skip: bool) -> Result<(), String> {
        self.inner.set_skip(skip).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn skip(&self) -> Result<bool, String> {
        self.inner.skip().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn is_from_ast(&self) -> Result<bool, String> {
        self.inner.is_from_ast().map_err(|e| e.to_string())
    }
}

#[export_baml_fn]
impl ClassBuilderWrapper {
    #[export_baml_fn]
    fn name(&self) -> String {
        self.inner.class_name.to_string()
    }

    #[export_baml_fn]
    fn type_(&self) -> Result<TypeIR, String> {
        self.inner.r#type().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn list_properties(&self) -> Result<Vec<objects::ClassPropertyBuilder>, String> {
        self.inner.list_properties().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_alias(&self, alias: &str) -> Result<(), String> {
        self.inner.set_alias(alias).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_description(&self, description: &str) -> Result<(), String> {
        self.inner
            .set_description(description)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn alias(&self) -> Result<Option<String>, String> {
        self.inner.alias().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn description(&self) -> Result<Option<String>, String> {
        self.inner.description().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn add_property(
        &self,
        name: &str,
        field_type: &TypeWrapper,
    ) -> Result<objects::ClassPropertyBuilder, String> {
        self.inner
            .add_property(name, field_type.as_ref().clone())
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn property(&self, name: &str) -> Result<objects::ClassPropertyBuilder, String> {
        self.inner.property(name).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn is_from_ast(&self) -> Result<bool, String> {
        self.inner.is_from_ast().map_err(|e| e.to_string())
    }
}

#[export_baml_fn]
impl ClassPropertyBuilderWrapper {
    #[export_baml_fn]
    fn name(&self) -> String {
        self.inner.property_name.to_string()
    }

    #[export_baml_fn]
    fn set_description(&self, description: &str) -> Result<(), String> {
        self.inner
            .set_description(description)
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_alias(&self, alias: &str) -> Result<(), String> {
        self.inner.set_alias(alias).map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn set_type(&self, field_type: &TypeWrapper) -> Result<(), String> {
        self.inner
            .set_type(field_type.as_ref().clone())
            .map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn description(&self) -> Result<Option<String>, String> {
        self.inner.description().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn alias(&self) -> Result<Option<String>, String> {
        self.inner.alias().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn type_(&self) -> Result<TypeIR, String> {
        self.inner.type_().map_err(|e| e.to_string())
    }

    #[export_baml_fn]
    fn is_from_ast(&self) -> Result<bool, String> {
        self.inner.is_from_ast().map_err(|e| e.to_string())
    }
}
