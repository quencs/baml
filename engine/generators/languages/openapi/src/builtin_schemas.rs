use indexmap::IndexMap;

/// A collection of schemas that are built in to BAML.
/// (Not generated from user BAML files).
///
use crate::{
    r#type::{OpenApiMeta, TypeOpenApi, TypePrimitive},
    TypeName,
};

// BamlImage:
// oneOf:
// - type: object
//   title: BamlImageBase64
//   properties:
//     base64:
//       type: string
//     media_type:
//       type: string
//   required:
//   - base64
// - type: object
//   title: BamlImageUrl
//   properties:
//     url:
//       type: string
//     media_type:
//       type: string
//   required:
//   - url

// BamlAudio:
// oneOf:
// - type: object
//   title: BamlAudioBase64
//   properties:
//     base64:
//       type: string
//     media_type:
//       type: string
//   required:
//   - base64
// - type: object
//   title: BamlAudioUrl
//   properties:
//     url:
//       type: string
//     media_type:
//       type: string
//   required:
//   - url
// BamlOptions:
// type: object
// nullable: false
// properties:
//   client_registry:
//     type: object
//     nullable: false
//     properties:
//       clients:
//         type: array
//         items:
//           $ref: '#/components/schemas/ClientProperty'
//       primary:
//         type: string
//         nullable: false
//     required:
//     - clients
// ClientProperty:
// type: object
// properties:
//   name:
//     type: string
//   provider:
//     type: string
//   retry_policy:
//     type: string
//     nullable: false
//   options:
//     type: object
//     additionalProperties: true
// required:
// - name
// - provider
// - options
// Check:
// type: object
// properties:
//   name:
//     type: string
//   expr:
//     type: string
//   status:
//     type: string

pub fn builtin_schemas() -> IndexMap<TypeName, TypeOpenApi> {
    IndexMap::from_iter(vec![
        (
            TypeName("BamlOptions".to_string()),
            TypeOpenApi::Inline {
                r#type: TypePrimitive::Object {
                    properties: IndexMap::from_iter(vec![(
                        "client_registry".to_string(),
                        TypeOpenApi::Ref {
                            r#ref: "#/components/schemas/ClientRegistry".to_string(),
                            meta: OpenApiMeta::default(),
                        },
                    )]),
                    required: vec!["client_registry".to_string()],
                    additional_properties: false,
                },
                meta: OpenApiMeta::default(),
            },
        ),
        (
            TypeName("BamlImage".to_string()),
            TypeOpenApi::Union {
                one_of: vec![
                    TypeOpenApi::Inline {
                        r#type: TypePrimitive::Object {
                            properties: IndexMap::from_iter(vec![
                                ("base64".to_string(), type_string()),
                                ("media_type".to_string(), type_string()),
                            ]),
                            required: vec!["base64".to_string()],
                            additional_properties: false,
                        },
                        meta: OpenApiMeta::default(),
                    },
                    TypeOpenApi::Inline {
                        r#type: TypePrimitive::Object {
                            properties: IndexMap::from_iter(vec![
                                ("url".to_string(), type_string()),
                                ("media_type".to_string(), type_string()),
                            ]),
                            required: vec!["url".to_string()],
                            additional_properties: false,
                        },
                        meta: OpenApiMeta::default(),
                    },
                ],
                meta: OpenApiMeta::default(),
            },
        ),
        (
            TypeName("BamlAudio".to_string()),
            TypeOpenApi::Union {
                one_of: vec![
                    TypeOpenApi::Inline {
                        r#type: TypePrimitive::Object {
                            properties: IndexMap::from_iter(vec![
                                ("base64".to_string(), type_string()),
                                ("media_type".to_string(), type_string()),
                            ]),
                            required: vec!["base64".to_string()],
                            additional_properties: false,
                        },
                        meta: OpenApiMeta::default(),
                    },
                    TypeOpenApi::Inline {
                        r#type: TypePrimitive::Object {
                            properties: IndexMap::from_iter(vec![
                                ("url".to_string(), type_string()),
                                ("media_type".to_string(), type_string()),
                            ]),
                            required: vec!["url".to_string()],
                            additional_properties: false,
                        },
                        meta: OpenApiMeta::default(),
                    },
                ],
                meta: OpenApiMeta::default(),
            },
        ),
        (
            TypeName("BamlCheck".to_string()),
            TypeOpenApi::Inline {
                r#type: TypePrimitive::Object {
                    properties: IndexMap::from_iter(vec![
                        ("name".to_string(), type_string()),
                        ("expr".to_string(), type_string()),
                        ("status".to_string(), type_string()),
                    ]),
                    required: vec!["name".to_string(), "expr".to_string()],
                    additional_properties: false,
                },
                meta: OpenApiMeta::default(),
            },
        ),
        (
            TypeName("ClientProperty".to_string()),
            TypeOpenApi::Inline {
                r#type: TypePrimitive::Object {
                    properties: IndexMap::from_iter(vec![
                        ("name".to_string(), type_string()),
                        ("provider".to_string(), type_string()),
                        (
                            "options".to_string(),
                            TypeOpenApi::Inline {
                                r#type: TypePrimitive::Object {
                                    properties: IndexMap::new(),
                                    required: Vec::new(),
                                    additional_properties: true,
                                },
                                meta: OpenApiMeta::default(),
                            },
                        ),
                    ]),
                    required: vec!["name".to_string(), "provider".to_string()],
                    additional_properties: false,
                },
                meta: OpenApiMeta::default(),
            },
        ),
    ])
}

/// Helper function to create a TypeOpenApi String with default meta.
fn type_string() -> TypeOpenApi {
    TypeOpenApi::Inline {
        r#type: TypePrimitive::String,
        meta: OpenApiMeta::default(),
    }
}
