use anyhow::{anyhow, Result};
use baml_types::BamlValue;
use std::os::raw::c_char;
use std::sync::Arc;

use crate::baml::cffi::{
    cffi_raw_object::Object as RawPointerVariant, cffi_value_holder::Value as HolderValue,
    cffi_value_raw_object::Object as RawObjectVariant, CffiMapEntry, CffiRawObject,
    CffiValueHolder, CffiValueRawObject, CffiValueStreamingState,
};

pub trait Decode {
    type From;

    fn decode(from: Self::From) -> Result<Self>
    where
        Self: Sized;
}

pub trait DecodeFromBuffer {
    fn from_c_buffer(buffer: *const c_char, length: usize) -> Result<Self>
    where
        Self: Sized;
}

impl<T> DecodeFromBuffer for T
where
    T: Decode,
    T::From: prost::Message + Default,
{
    fn from_c_buffer(buffer: *const c_char, length: usize) -> Result<Self>
    where
        Self: Sized,
    {
        use prost::Message;

        let buffer = unsafe { std::slice::from_raw_parts(buffer as *const u8, length) };
        let root = T::From::decode(buffer)?;

        Self::decode(root)
    }
}

impl Decode for BamlValue {
    type From = CffiValueHolder;

    fn decode(from: Self::From) -> Result<Self> {
        Ok(match from.value {
            Some(HolderValue::NullValue(_)) | None => BamlValue::Null,
            Some(HolderValue::StringValue(value)) => BamlValue::String(value),
            Some(HolderValue::IntValue(value)) => BamlValue::Int(value),
            Some(HolderValue::FloatValue(value)) => BamlValue::Float(value),
            Some(HolderValue::BoolValue(value)) => BamlValue::Bool(value),
            Some(HolderValue::ListValue(list)) => {
                let values = list
                    .values
                    .into_iter()
                    .map(BamlValue::decode)
                    .collect::<Result<Vec<_>>>()?;
                BamlValue::List(values)
            }
            Some(HolderValue::MapValue(map)) => {
                let entries = map
                    .entries
                    .into_iter()
                    .map(from_cffi_map_entry)
                    .collect::<Result<_>>()?;
                BamlValue::Map(entries)
            }
            Some(HolderValue::ClassValue(class)) => {
                let type_name = class
                    .name
                    .ok_or_else(|| anyhow!("class value missing type name"))?
                    .name;
                let fields = class
                    .fields
                    .into_iter()
                    .map(from_cffi_map_entry)
                    .collect::<Result<_>>()?;

                BamlValue::Class(type_name, fields)
            }
            Some(HolderValue::EnumValue(enm)) => {
                let type_name = enm
                    .name
                    .ok_or_else(|| anyhow!("enum value missing type name"))?
                    .name;
                BamlValue::Enum(type_name, enm.value)
            }
            Some(HolderValue::ObjectValue(object)) => decode_object_value(object)?,
            Some(HolderValue::TupleValue(tuple)) => {
                let values = tuple
                    .values
                    .into_iter()
                    .map(BamlValue::decode)
                    .collect::<Result<_>>()?;
                BamlValue::List(values)
            }
            Some(HolderValue::UnionVariantValue(union_variant)) => {
                let value = union_variant
                    .value
                    .ok_or_else(|| anyhow!("union variant missing value"))?;
                BamlValue::decode(*value)?
            }
            Some(HolderValue::CheckedValue(checked)) => {
                let value = checked
                    .value
                    .ok_or_else(|| anyhow!("checked value missing inner value"))?;
                BamlValue::decode(*value)?
            }
            Some(HolderValue::StreamingStateValue(stream_state)) => {
                decode_streaming_state_value(*stream_state)?
            }
        })
    }
}

fn decode_object_value(object: CffiValueRawObject) -> Result<BamlValue> {
    match object.object {
        Some(RawObjectVariant::Media(raw)) => decode_media_raw_object(raw),
        Some(RawObjectVariant::Type(_)) => {
            Err(anyhow!("unexpected type handle returned as object value"))
        }
        None => Err(anyhow!("object value missing payload")),
    }
}

fn decode_media_raw_object(raw: CffiRawObject) -> Result<BamlValue> {
    match raw.object {
        Some(
            RawPointerVariant::MediaImage(pointer)
            | RawPointerVariant::MediaAudio(pointer)
            | RawPointerVariant::MediaPdf(pointer)
            | RawPointerVariant::MediaVideo(pointer),
        ) => decode_media_pointer(pointer.pointer),
        other => Err(anyhow!(
            "unsupported media object variant returned from runtime: {:?}",
            other
        )),
    }
}

fn decode_media_pointer(pointer: i64) -> Result<BamlValue> {
    if pointer == 0 {
        return Err(anyhow!("received null media pointer from runtime"));
    }

    let ptr = pointer as *const baml_types::BamlMedia;
    let media = unsafe {
        let arc = Arc::from_raw(ptr);
        let media = (*arc).clone();
        let _ = Arc::into_raw(arc);
        media
    };

    Ok(BamlValue::Media(media))
}

fn decode_streaming_state_value(stream_state: CffiValueStreamingState) -> Result<BamlValue> {
    match stream_state.value {
        Some(value) => BamlValue::decode(*value),
        None => Ok(BamlValue::Null),
    }
}

fn from_cffi_map_entry(entry: CffiMapEntry) -> Result<(String, BamlValue)> {
    let value = entry
        .value
        .ok_or_else(|| anyhow!("map entry value for key '{}' was null", entry.key))?;
    Ok((entry.key, BamlValue::decode(value)?))
}
