use baml_types::{BamlMedia, BamlMediaType};
use std::sync::Arc;

use super::constructors::construct_object;
use crate::baml::cffi::{
    cffi_raw_object::Object as RawPointerVariant, cffi_value_holder::Value as HolderValue,
    CffiMapEntry, CffiObjectType, CffiPointerType, CffiRawObject, CffiValueHolder,
};

/// Lightweight handle around a raw object pointer managed by the shared runtime.
#[derive(Clone, Debug)]
pub struct RawObjectHandle {
    raw: CffiRawObject,
}

impl RawObjectHandle {
    pub(crate) fn new(raw: CffiRawObject) -> Self {
        Self { raw }
    }

    pub fn to_cffi(&self) -> CffiRawObject {
        self.raw.clone()
    }
}

/// Handle for collector objects.
#[derive(Clone, Debug)]
pub struct CollectorHandle {
    handle: RawObjectHandle,
}

impl CollectorHandle {
    pub fn new(name: Option<&str>) -> Result<Self, String> {
        let mut kwargs = Vec::new();
        if let Some(name) = name {
            kwargs.push(string_entry("name", name));
        }

        construct_object(CffiObjectType::ObjectCollector, kwargs)
            .map(|raw| Self {
                handle: RawObjectHandle::new(raw),
            })
            .map_err(|err| err.to_string())
    }

    pub fn to_cffi(&self) -> CffiRawObject {
        self.handle.to_cffi()
    }
}

/// Handle for type builder objects.
#[derive(Clone, Debug)]
pub struct TypeBuilderHandle {
    handle: RawObjectHandle,
}

impl TypeBuilderHandle {
    pub fn new() -> Result<Self, String> {
        construct_object(CffiObjectType::ObjectTypeBuilder, Vec::new())
            .map(|raw| Self {
                handle: RawObjectHandle::new(raw),
            })
            .map_err(|err| err.to_string())
    }

    pub fn to_cffi(&self) -> CffiRawObject {
        self.handle.to_cffi()
    }
}

/// Convert a [`BamlMedia`] value into the raw representation expected by the CFFI layer.
pub fn media_to_raw(media: &BamlMedia) -> CffiRawObject {
    let pointer = Arc::into_raw(Arc::new(media.clone())) as i64;
    let pointer = CffiPointerType { pointer };

    let object = match media.media_type {
        BamlMediaType::Image => RawPointerVariant::MediaImage(pointer),
        BamlMediaType::Audio => RawPointerVariant::MediaAudio(pointer),
        BamlMediaType::Pdf => RawPointerVariant::MediaPdf(pointer),
        BamlMediaType::Video => RawPointerVariant::MediaVideo(pointer),
    };

    CffiRawObject {
        object: Some(object),
    }
}

fn string_entry(key: &str, value: &str) -> CffiMapEntry {
    CffiMapEntry {
        key: key.to_string(),
        value: Some(CffiValueHolder {
            value: Some(HolderValue::StringValue(value.to_string())),
            r#type: None,
        }),
    }
}
