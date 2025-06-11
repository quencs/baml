use crate::r#type::{convert_ir_type, OpenApiMeta, TypeOpenApi};
use crate::{FunctionName, TypeName};
use indexmap::IndexMap;
use internal_baml_core::ir::repr;

pub use class::convert_ir_class;
pub use function::{convert_ir_function, FunctionOpenApi};

struct OpenApiUserData {
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
            info: serde_json::json!({
                "title": "Baml API",
                "version": "0.1.0",
                "description": "Baml API",
            }),
            openapi: "3.0.0".to_string(),
            servers: serde_json::json!([]),
            paths: IndexMap::new(),
            components: Components {
                requestBodies: IndexMap::new(),
                schemas: IndexMap::new(),
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
}
