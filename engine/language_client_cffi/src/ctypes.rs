use baml_runtime::HasFieldType;
use baml_types::{BamlMedia, BamlValue, BamlValueWithMeta};

#[allow(non_snake_case)]
#[path = "cffi/cffi_generated.rs"]
mod cffi_generated;

use cffi_generated::cffi::*;

impl From<cffi_generated::cffi::CFFIValueHolder<'_>> for BamlValue {
    fn from(value: cffi_generated::cffi::CFFIValueHolder) -> Self {
        let value_type = value.value_type();
        match value_type {
            CFFIValueUnion::NONE => BamlValue::Null,
            CFFIValueUnion::CFFIValueString => value
                .value_as_cffivalue_string()
                .and_then(|s| s.value().map(|s| BamlValue::String(s.to_string())))
                .expect("Failed to convert CFFIValueString to BamlValue"),
            CFFIValueUnion::CFFIValueInt => value
                .value_as_cffivalue_int()
                .map(|i| BamlValue::Int(i.value()))
                .expect("Failed to convert CFFIValueInt to BamlValue"),
            CFFIValueUnion::CFFIValueFloat => value
                .value_as_cffivalue_float()
                .map(|f| BamlValue::Float(f.value()))
                .expect("Failed to convert CFFIValueFloat to BamlValue"),
            CFFIValueUnion::CFFIValueBool => value
                .value_as_cffivalue_bool()
                .map(|b| BamlValue::Bool(b.value()))
                .expect("Failed to convert CFFIValueBool to BamlValue"),
            CFFIValueUnion::CFFIValueList => value
                .value_as_cffivalue_list()
                .and_then(|l| l.values())
                .map(|v| v.into_iter().map(|v| v.into()))
                .map(|l| BamlValue::List(l.collect()))
                .expect("Failed to convert CFFIValueList to BamlValue"),
            CFFIValueUnion::CFFIValueMap => value
                .value_as_cffivalue_map()
                .and_then(|m| m.entries())
                .map(|v| v.into_iter().map(|v| v.into()).collect())
                .map(|kv| BamlValue::Map(kv))
                .expect("Failed to convert CFFIValueMap to BamlValue"),
            CFFIValueUnion::CFFIValueClass => value
                .value_as_cffivalue_class()
                .expect("Failed to convert CFFIValueClass to BamlValue")
                .into(),
            CFFIValueUnion::CFFIValueEnum => value
                .value_as_cffivalue_enum()
                .expect("Failed to convert CFFIValueEnum to BamlValue")
                .into(),
            CFFIValueUnion::CFFIValueMedia => value
                .value_as_cffivalue_media()
                .map(|m| BamlValue::Media(m.into()))
                .expect("Failed to convert CFFIValueMedia to BamlValue"),
            CFFIValueUnion::CFFIValueTuple => value
                .value_as_cffivalue_tuple()
                .expect("Failed to convert CFFIValueTuple to BamlValue")
                .into(),
            CFFIValueUnion::CFFIValueUnionVariant => value
                .value_as_cffivalue_union_variant()
                .expect("Failed to convert CFFIValueUnionVariant to BamlValue")
                .into(),
            CFFIValueUnion::CFFIValueChecked => value
                .value_as_cffivalue_checked()
                .expect("Failed to convert CFFIValueChecked to BamlValue")
                .into(),
            CFFIValueUnion::CFFIValueStreamingState => value
                .value_as_cffivalue_streaming_state()
                .expect("Failed to convert CFFIValueStreamingState to BamlValue")
                .into(),
            other => {
                panic!("Unsupported value type: {:?}", other);
            }
        }
    }
}

impl From<CFFIMapEntry<'_>> for (String, BamlValue) {
    fn from(value: CFFIMapEntry) -> Self {
        let key = value
            .key()
            .expect("Failed to have CFFIMapEntry key")
            .to_string();
        let value = value
            .value()
            .expect("Failed to have CFFIMapEntry value")
            .into();
        (key, value)
    }
}

impl From<CFFIValueClass<'_>> for BamlValue {
    fn from(value: CFFIValueClass) -> Self {
        BamlValue::Class(
            value
                .name()
                .expect("Failed to have CFFIValueClass name")
                .to_string(),
            value
                .fields()
                .expect("Failed to have CFFIValueClass fields")
                .into_iter()
                .map(|v| v.into())
                .collect(),
        )
    }
}

impl From<CFFIValueEnum<'_>> for BamlValue {
    fn from(value: CFFIValueEnum) -> Self {
        BamlValue::Enum(
            value
                .name()
                .expect("Failed to have CFFIValueEnum name")
                .to_string(),
            value
                .value()
                .expect("Failed to have CFFIValueEnum value")
                .to_string(),
        )
    }
}

impl From<CFFIValueMedia<'_>> for BamlMedia {
    fn from(value: CFFIValueMedia<'_>) -> Self {
        let media_type = value
            .media_type()
            .expect("Failed to have CFFIMediaType")
            .into();
        let media_value = value
            .media_value()
            .expect("Failed to have CFFIMediaType media_value");
        let mime_type = media_value.mime_type().map(|s| s.to_string());
        match media_value.content_type() {
            CFFIMediaContentUnion::CFFIMediaContentBase64 => BamlMedia::base64(
                media_type,
                media_value
                    .content_as_cffimedia_content_base_64()
                    .expect("Failed to have CFFIMediaContentBase64")
                    .data()
                    .expect("Failed to have CFFIMediaContentBase64 data")
                    .to_string(),
                mime_type,
            ),
            CFFIMediaContentUnion::CFFIMediaContentFile => BamlMedia::file(
                media_type,
                "./cffi_place_holder.baml".into(),
                media_value
                    .content_as_cffimedia_content_file()
                    .expect("Failed to have CFFIMediaContentFile")
                    .path()
                    .expect("Failed to have CFFIMediaContentFile path")
                    .to_string()
                    .into(),
                mime_type,
            ),
            CFFIMediaContentUnion::CFFIMediaContentUrl => BamlMedia::url(
                media_type,
                media_value
                    .content_as_cffimedia_content_url()
                    .expect("Failed to have CFFIMediaContentUrl")
                    .url()
                    .expect("Failed to have CFFIMediaContentUrl url")
                    .to_string(),
                mime_type,
            ),
            _ => unimplemented!(
                "Unsupported media content type: {:?}",
                media_value.content_type()
            ),
        }
    }
}

impl From<CFFIMediaType<'_>> for baml_types::BamlMediaType {
    fn from(value: CFFIMediaType) -> Self {
        match value.type_() {
            MediaTypeEnum::Image => baml_types::BamlMediaType::Image,
            MediaTypeEnum::Audio => baml_types::BamlMediaType::Audio,
            MediaTypeEnum::Other => unimplemented!("Other media type is not supported"),
            _ => unimplemented!("Unsupported media type: {:?}", value.type_()),
        }
    }
}

impl From<CFFIValueTuple<'_>> for BamlValue {
    fn from(_: CFFIValueTuple) -> Self {
        unimplemented!("BamlValueTuple is not supported");
    }
}

impl From<CFFIValueUnionVariant<'_>> for BamlValue {
    fn from(value: CFFIValueUnionVariant) -> Self {
        value
            .value()
            .expect("Failed to have CFFIValueUnionVariant value")
            .into()
    }
}

impl From<CFFIValueChecked<'_>> for BamlValue {
    fn from(value: CFFIValueChecked) -> Self {
        unimplemented!("CFFIValueChecked is not supported");
    }
}

impl From<CFFIValueStreamingState<'_>> for BamlValue {
    fn from(value: CFFIValueStreamingState) -> Self {
        unimplemented!("CFFIValueStreamingState is not supported");
    }
}

pub fn serialize_baml_value_with_meta<'a, 'b, T>(
    value: &'b BamlValueWithMeta<T>,
    mut builder: &'a mut flatbuffers::FlatBufferBuilder<'b>,
) -> &'a [u8]
where
    BamlValueWithMeta<T>: HasFieldType,
{
    let value_holder = from_baml_value_with_meta(value, &mut builder);
    builder.finish(value_holder, None);
    builder.finished_data()
}

fn from_baml_value_with_meta<'a, 'b, T>(
    value: &'b BamlValueWithMeta<T>,
    mut builder: &'a mut flatbuffers::FlatBufferBuilder<'b>,
) -> flatbuffers::WIPOffset<CFFIValueHolder<'b>>
where
    BamlValueWithMeta<T>: HasFieldType,
{
    let (value_type, value_holder) = match value {
        BamlValueWithMeta::String(val, _) => {
            // Create a FlatBuffers string and get its offset.
            let str_offset = builder.create_string(val);

            // Build the CFFIValueString table.
            let value_string = CFFIValueString::create(
                &mut builder,
                &CFFIValueStringArgs {
                    value: Some(str_offset),
                },
            );
            (
                CFFIValueUnion::CFFIValueString,
                value_string.as_union_value(),
            )
        }
        BamlValueWithMeta::Int(val, _) => {
            let value_int = CFFIValueInt::create(&mut builder, &CFFIValueIntArgs { value: *val });

            (CFFIValueUnion::CFFIValueInt, value_int.as_union_value())
        }
        BamlValueWithMeta::Float(val, _) => {
            let value_float =
                CFFIValueFloat::create(&mut builder, &CFFIValueFloatArgs { value: *val });

            (CFFIValueUnion::CFFIValueFloat, value_float.as_union_value())
        }
        BamlValueWithMeta::Bool(val, _) => {
            let value_bool =
                CFFIValueBool::create(&mut builder, &CFFIValueBoolArgs { value: *val });

            (CFFIValueUnion::CFFIValueBool, value_bool.as_union_value())
        }
        BamlValueWithMeta::List(val, _) => {
            let mut items = Vec::new();
            for v in val.iter() {
                items.push(from_baml_value_with_meta(v, &mut builder));
            }

            let values = builder.create_vector_from_iter(items.into_iter());

            let value_list = CFFIValueList::create(
                &mut builder,
                &CFFIValueListArgs {
                    values: Some(values),
                },
            );

            (CFFIValueUnion::CFFIValueList, value_list.as_union_value())
        }
        BamlValueWithMeta::Map(val, _) => {
            let mut items = Vec::new();
            for (k, v) in val.iter() {
                let key = builder.create_string(k);
                let value = from_baml_value_with_meta(v, &mut builder);

                items.push(CFFIMapEntry::create(
                    &mut builder,
                    &CFFIMapEntryArgs {
                        key: Some(key),
                        value: Some(value),
                    },
                ));
            }

            let entries = builder.create_vector_from_iter(items.into_iter());

            let value_map = CFFIValueMap::create(
                &mut builder,
                &CFFIValueMapArgs {
                    entries: Some(entries),
                },
            );

            (CFFIValueUnion::CFFIValueMap, value_map.as_union_value())
        }
        BamlValueWithMeta::Class(class_name, fields, _) => {
            let mut items = Vec::new();
            for (k, v) in fields.iter() {
                let key = builder.create_string(k);
                let value = from_baml_value_with_meta(v, &mut builder);
                items.push(CFFIMapEntry::create(
                    &mut builder,
                    &CFFIMapEntryArgs {
                        key: Some(key),
                        value: Some(value),
                    },
                ));
            }

            let entries = builder.create_vector_from_iter(items.into_iter());

            let class_name = builder.create_string(class_name);
            let value_class = CFFIValueClass::create(
                &mut builder,
                &CFFIValueClassArgs {
                    name: Some(class_name),
                    fields: Some(entries),
                    dynamic_fields: None,
                },
            );

            (CFFIValueUnion::CFFIValueClass, value_class.as_union_value())
        }
        BamlValueWithMeta::Enum(enum_name, enum_value, _) => {
            let enum_name = builder.create_string(enum_name);
            let enum_value = builder.create_string(enum_value);
            let value_enum = CFFIValueEnum::create(
                &mut builder,
                &CFFIValueEnumArgs {
                    name: Some(enum_name),
                    value: Some(enum_value),
                    is_dynamic: false,
                },
            );

            (CFFIValueUnion::CFFIValueEnum, value_enum.as_union_value())
        }
        BamlValueWithMeta::Media(val, _) => {
            let media_type = match val.media_type {
                baml_types::BamlMediaType::Image => MediaTypeEnum::Image,
                baml_types::BamlMediaType::Audio => MediaTypeEnum::Audio,
            };
            let mime_type = val.mime_type.as_ref().map(|s| builder.create_string(s));
            let (media_content_type, media_content_value) = match &val.content {
                baml_types::BamlMediaContent::Base64(data) => {
                    let data = builder.create_string(&data.base64);
                    let content_base64 = CFFIMediaContentBase64::create(
                        &mut builder,
                        &CFFIMediaContentBase64Args { data: Some(data) },
                    );
                    (
                        CFFIMediaContentUnion::CFFIMediaContentBase64,
                        content_base64.as_union_value(),
                    )
                }
                baml_types::BamlMediaContent::File(path) => {
                    let path = builder.create_string(
                        path.relpath
                            .to_str()
                            .expect("Failed to convert BamlMediaContentFile path to string"),
                    );
                    let content_file = CFFIMediaContentFile::create(
                        &mut builder,
                        &CFFIMediaContentFileArgs { path: Some(path) },
                    );
                    (
                        CFFIMediaContentUnion::CFFIMediaContentFile,
                        content_file.as_union_value(),
                    )
                }
                baml_types::BamlMediaContent::Url(url) => {
                    let url = builder.create_string(&url.url);
                    let content_url = CFFIMediaContentUrl::create(
                        &mut builder,
                        &CFFIMediaContentUrlArgs { url: Some(url) },
                    );
                    (
                        CFFIMediaContentUnion::CFFIMediaContentUrl,
                        content_url.as_union_value(),
                    )
                }
            };

            let media_value = CFFIMediaValue::create(
                &mut builder,
                &CFFIMediaValueArgs {
                    content_type: media_content_type,
                    content: Some(media_content_value),
                    mime_type,
                },
            );

            let media_type = CFFIMediaType::create(
                &mut builder,
                &CFFIMediaTypeArgs {
                    type_: media_type,
                    other: None,
                },
            );
            let value_media = CFFIValueMedia::create(
                &mut builder,
                &CFFIValueMediaArgs {
                    media_type: Some(media_type),
                    media_value: Some(media_value),
                },
            );

            (CFFIValueUnion::CFFIValueMedia, value_media.as_union_value())
        }
        BamlValueWithMeta::Null(_) => {
            return CFFIValueHolder::create(
                &mut builder,
                &CFFIValueHolderArgs {
                    value_type: CFFIValueUnion::NONE,
                    value: None,
                },
            )
        }
    };

    let value_holder = CFFIValueHolder::create(
        &mut builder,
        &CFFIValueHolderArgs {
            value_type,
            value: Some(value_holder),
        },
    );

    value_holder
}
