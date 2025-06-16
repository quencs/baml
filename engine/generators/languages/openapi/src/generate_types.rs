use crate::r#type::{convert_ir_type, OpenApiMeta, TypeOpenApi, TypePrimitive};
use crate::{
    ComponentRequestBody, Components, FunctionName, MediaTypeSchema, OpenApiSchema, Path,
    PathRequestBody, Response, TypeName,
};
use indexmap::IndexMap;
use internal_baml_core::ir::repr;

pub use class::convert_ir_class;
pub use function::{convert_ir_function, FunctionOpenApi};

pub struct OpenApiUserData {
    pub types: IndexMap<TypeName, TypeOpenApi>,
    pub functions: IndexMap<FunctionName, FunctionOpenApi>,
}

impl OpenApiUserData {
    pub fn from_ir(ir: &repr::IntermediateRepr) -> Self {
        let functions = ir
            .walk_functions()
            .map(|function| {
                (
                    FunctionName(function.name().to_string()),
                    convert_ir_function(&ir, &function.elem()),
                )
            })
            .collect();

        let types = ir
            .walk_classes()
            .map(|class| {
                (
                    TypeName(class.name().to_string()),
                    convert_ir_class(&ir, &class.item.elem),
                )
            })
            .collect();
        OpenApiUserData { types, functions }
    }

    pub fn render(&self) -> String {
        let openapi_schema = OpenApiSchema {
            openapi: "3.0.0".to_string(),
            info: serde_json::json!({
                "title": "Baml API",
                "version": "0.1.0",
                "description": "Baml API",
            }),
            servers: serde_json::json!([{
                "url": "{address}",
                "variables": {
                    "address": {
                        "default": "http://localhost:2024"
                    }
                }
            }]),
            paths: self
                .functions
                .iter()
                .map(|(name, function)| {
                    (
                        format!("/call/{}", name.0),
                        function::render_path(name, function),
                    )
                })
                .collect(),
            components: Components {
                requestBodies: self
                    .functions
                    .iter()
                    .map(|(name, func)| {
                        let mut schema = func.return_type.clone();
                        schema.meta_mut().title = Some(format!("{}Request", name.0));
                        let component_request_body = ComponentRequestBody {
                            content: vec![(
                                "application/json".to_string(),
                                MediaTypeSchema {
                                    schema: TypeOpenApi::Inline {
                                        r#type: TypePrimitive::Object {
                                            properties: IndexMap::from_iter(
                                                func.args.clone().into_iter().chain(
                                                    std::iter::once((
                                                        "__baml_options__".to_string(),
                                                        TypeOpenApi::Ref {
                                                            r#ref:
                                                                "#/components/schemas/BamlOptions"
                                                                    .to_string(),
                                                            meta: OpenApiMeta {
                                                                nullable: true,
                                                                ..OpenApiMeta::default()
                                                            },
                                                        },
                                                    )),
                                                ),
                                            ),
                                            required: func
                                                .args
                                                .iter()
                                                .map(|(name, _)| name.clone())
                                                .collect(), // TODO: Omit optional args?
                                            additional_properties: false,
                                        },
                                        meta: OpenApiMeta {
                                            title: Some(format!("{}Request", name.0)),
                                            ..OpenApiMeta::default()
                                        }, // TODO: Correct?
                                    },
                                },
                            )]
                            .into_iter()
                            .collect(),
                            required: true,
                        };
                        (name.clone(), component_request_body)
                    })
                    .collect(),
                schemas: self.types.clone(),
            },
        };
        serde_yaml::to_string(&openapi_schema).expect("Should serialize")
    }
}

mod class {
    use super::*;
    use crate::r#type::{TypeOpenApi, TypePrimitive};

    pub fn convert_ir_class(ir: &repr::IntermediateRepr, class: &repr::Class) -> TypeOpenApi {
        let mut required = Vec::new();
        let properties = class
            .static_fields
            .iter()
            .map(|field| {
                if field.elem.r#type.elem.is_optional() {
                    (
                        field.elem.name.clone(),
                        convert_ir_type(&ir, &field.elem.r#type.elem),
                    )
                } else {
                    required.push(field.elem.name.clone());
                    (
                        field.elem.name.clone(),
                        convert_ir_type(&ir, &field.elem.r#type.elem),
                    )
                }
            })
            .collect();

        TypeOpenApi::Inline {
            r#type: TypePrimitive::Object {
                properties,
                required,
                additional_properties: false,
            },
            meta: OpenApiMeta::default(),
        }
    }

    pub fn builtin_classes() -> IndexMap<TypeName, TypeOpenApi> {
        IndexMap::from_iter(vec![(
            TypeName("BamlOptions".to_string()),
            TypeOpenApi::Inline {
                r#type: TypePrimitive::Object {
                    properties: IndexMap::new(),
                    required: Vec::new(),
                    additional_properties: false,
                },
                meta: OpenApiMeta::default(),
            },
        )])
    }
}

mod function {
    use super::*;
    use internal_baml_core::ir::repr;

    pub struct FunctionOpenApi {
        pub name: String,
        pub documentation: Option<String>,
        pub args: Vec<(String, TypeOpenApi)>,
        pub return_type: TypeOpenApi,
        // pub stream_return_type: TypeOpenApi, // TODO: Support streaming responses.
    }

    pub fn convert_ir_function(
        ir: &repr::IntermediateRepr,
        function: &repr::Function,
    ) -> FunctionOpenApi {
        let args = function
            .inputs
            .iter()
            .map(|(arg_name, arg_type)| (arg_name.clone(), convert_ir_type(&ir, &arg_type)))
            .collect();
        FunctionOpenApi {
            name: function.name.clone(),
            documentation: None,
            args,
            return_type: convert_ir_type(&ir, &function.output),
        }
    }

    pub fn render_path(name: &FunctionName, function: &FunctionOpenApi) -> IndexMap<String, Path> {
        let path = Path {
            request_body: PathRequestBody {
                ref_: format!("#/components/requestBodies/{}", name.0),
            },

            responses: IndexMap::from_iter(vec![(
                "200",
                Response {
                    description: "Successful operation".to_string(),
                    content: IndexMap::from_iter(vec![(
                        "application/json".to_string(),
                        MediaTypeSchema {
                            schema: function.return_type.clone(),
                        },
                    )]),
                },
            )]),
            operation_id: function.name.clone(),
        };
        IndexMap::from_iter(vec![("post".to_string(), path)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use internal_baml_core::ir::repr::make_test_ir;

    #[test]
    pub fn basic_example() {
        let ir = make_test_ir(
            r##"
          class Foo {
            age int
          }

          function Go(name: string) -> Foo {
            client GPT4
            prompt #"Not important"#
          }

          client<llm> GPT4 {
            provider openai
            options {
              model gpt-4
              api_key env.OPENAI_API_KEY
            }
          } 

        "##,
        )
        .expect("Valid IR");
        let file = serde_yaml::to_string(&OpenApiUserData::from_ir(&ir).render())
            .expect("Should serialize");
        eprintln!("{}", file);
    }
}
