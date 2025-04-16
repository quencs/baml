use std::borrow::Cow;

use baml_types::{BamlValueWithMeta, HasFieldType};

use super::IntoRpcEvent;

impl<'a, T: HasFieldType> IntoRpcEvent<'a, baml_rpc::runtime_api::Value<'a>>
    for BamlValueWithMeta<T>
{
    fn into_rpc_event(&'a self) -> baml_rpc::runtime_api::Value<'a> {
        let type_ref = self.field_type().into_rpc_event();
        let value = match self {
            BamlValueWithMeta::String(s, _) => {
                baml_rpc::runtime_api::ValueContent::String(Cow::Borrowed(s))
            }
            BamlValueWithMeta::Int(v, _) => baml_rpc::runtime_api::ValueContent::Int(*v),
            BamlValueWithMeta::Float(v, _) => baml_rpc::runtime_api::ValueContent::Float(*v),
            BamlValueWithMeta::Bool(v, _) => baml_rpc::runtime_api::ValueContent::Boolean(*v),
            BamlValueWithMeta::Map(index_map, _) => baml_rpc::runtime_api::ValueContent::Map(
                index_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into_rpc_event()))
                    .collect(),
            ),
            BamlValueWithMeta::List(baml_value_with_metas, _) => todo!(),
            BamlValueWithMeta::Media(baml_media, _) => todo!(),
            BamlValueWithMeta::Enum(_, _, _) => todo!(),
            BamlValueWithMeta::Class(_, index_map, _) => todo!(),
            BamlValueWithMeta::Null(_) => todo!(),
        };

        baml_rpc::runtime_api::Value {
            r#type: type_ref,
            value,
        }
    }
}

impl<'a> IntoRpcEvent<'a, baml_rpc::ast::types::type_reference::TypeReference>
    for baml_types::FieldType
{
    fn into_rpc_event(&'a self) -> baml_rpc::ast::types::type_reference::TypeReference {
        let simplified = self.simplify();
        match simplified {
            baml_types::FieldType::Primitive(type_value) => todo!(),
            baml_types::FieldType::Enum(e) => todo!(),
            baml_types::FieldType::Literal(literal_value) => todo!(),
            baml_types::FieldType::Class(_) => todo!(),
            baml_types::FieldType::List(field_type) => todo!(),
            baml_types::FieldType::Map(field_type, field_type1) => todo!(),
            baml_types::FieldType::Union(field_types) => todo!(),
            baml_types::FieldType::Tuple(field_types) => todo!(),
            baml_types::FieldType::Optional(field_type) => todo!(),
            baml_types::FieldType::RecursiveTypeAlias(_) => todo!(),
            baml_types::FieldType::Arrow(arrow) => todo!(),
            baml_types::FieldType::WithMetadata {
                base,
                constraints,
                streaming_behavior,
            } => todo!(),
        }
    }
}

impl<'a> IntoRpcEvent<'a, baml_rpc::runtime_api::Media<'a>> for baml_types::BamlMedia {
    fn into_rpc_event(&'a self) -> baml_rpc::runtime_api::Media<'a> {
        baml_rpc::runtime_api::Media {
            mime_type: self.mime_type.clone(),
            value: self.content.into_rpc_event(),
        }
    }
}

impl<'a> IntoRpcEvent<'a, baml_rpc::runtime_api::MediaValue<'a>> for baml_types::BamlMediaContent {
    fn into_rpc_event(&'a self) -> baml_rpc::runtime_api::MediaValue<'a> {
        match self {
            baml_types::BamlMediaContent::Url(url) => {
                baml_rpc::runtime_api::MediaValue::Url(Cow::Borrowed(url.url.as_str()))
            }
            baml_types::BamlMediaContent::Base64(base64) => {
                baml_rpc::runtime_api::MediaValue::Base64(Cow::Borrowed(base64.base64.as_str()))
            }
            baml_types::BamlMediaContent::File(file_path) => {
                baml_rpc::runtime_api::MediaValue::FilePath(Cow::Owned(
                    file_path.relpath.display().to_string(),
                ))
            }
        }
    }
}
