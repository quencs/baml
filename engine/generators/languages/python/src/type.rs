use crate::package::{CurrentRenderModule, Module};

#[derive(Clone, PartialEq, Debug)]
pub enum LiteralType {
    String,
    Int,
    Float,
    Bool,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TypeWrapper {
    None,
    Checked(Box<TypeWrapper>),
    Optional(Box<TypeWrapper>),
}

impl TypeWrapper {
    pub fn as_checked(self) -> TypeWrapper {
        TypeWrapper::Checked(Box::new(self))
    }

    pub fn as_optional(self) -> TypeWrapper {
        TypeWrapper::Optional(Box::new(self))
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TypeMetaPython {
    pub type_wrapper: TypeWrapper,
    pub wrap_stream_state: bool,
}

impl Default for TypeWrapper {
    fn default() -> Self {
        TypeWrapper::None
    }
}

impl TypeMetaPython {
    pub fn is_optional(&self) -> bool {
        matches!(self.type_wrapper, TypeWrapper::Optional(_))
    }

    pub fn make_checked(&mut self) -> &mut Self {
        self.type_wrapper = TypeWrapper::Checked(Box::new(std::mem::take(&mut self.type_wrapper)));
        self
    }

    pub fn make_optional(&mut self) -> &mut Self {
        self.type_wrapper = TypeWrapper::Optional(Box::new(std::mem::take(&mut self.type_wrapper)));
        self
    }

    pub fn set_stream_state(&mut self) -> &mut Self {
        self.wrap_stream_state = true;
        self
    }
}

impl Default for TypeMetaPython {
    fn default() -> Self {
        TypeMetaPython {
            type_wrapper: TypeWrapper::default(),
            wrap_stream_state: false,
        }
    }
}

trait WrapType {
    fn wrap_type(&self, params: (&CurrentRenderModule, String)) -> String;
}

impl WrapType for TypeWrapper {
    fn wrap_type(&self, params: (&CurrentRenderModule, String)) -> String {
        let (pkg, orig) = &params;
        match self {
            TypeWrapper::None => orig.clone(),
            TypeWrapper::Checked(inner) => format!(
                "{}Checked[{}]",
                Module::checked().relative_from(pkg),
                inner.wrap_type(params)
            ),
            TypeWrapper::Optional(inner) => format!("Optional[{}]", inner.wrap_type(params)),
        }
    }
}

impl WrapType for TypeMetaPython {
    fn wrap_type(&self, params: (&CurrentRenderModule, String)) -> String {
        let module = params.0;
        let wrapped = self.type_wrapper.wrap_type(params);
        if self.wrap_stream_state {
            format!(
                "{}StreamState[{}]",
                Module::stream_state().relative_from(&pkg),
                wrapped
            )
        } else {
            wrapped
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum MediaTypePython {
    Image,
    Audio,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TypePython {
    String(TypeMetaPython),
    Int(TypeMetaPython),
    Float(TypeMetaPython),
    Bool(TypeMetaPython),
    Media(MediaTypePython, TypeMetaPython),
    // unions become classes
    Class {
        package: Module,
        name: String,
        dynamic: bool,
        meta: TypeMetaPython,
    },
    Union {
        package: Module,
        name: String,
        meta: TypeMetaPython,
    },
    Enum {
        package: Module,
        name: String,
        dynamic: bool,
        meta: TypeMetaPython,
    },
    List(Box<TypePython>, TypeMetaPython),
    Map(Box<TypePython>, Box<TypePython>, TypeMetaPython),
    Tuple(Vec<TypePython>, TypeMetaPython),
    // For any type that we can't represent in Go, we'll use this
    Any {
        reason: String,
        meta: TypeMetaPython,
    },
}

impl TypePython {
    // for unions, we need a default name for the type when the union is not named
    // pub fn default_name_within_union(&self) -> String {
    //     match self {
    //         TypePython::String(_) => "String".to_string(),
    //         TypePython::Int(_) => "Int".to_string(),
    //         TypePython::Float(_) => "Float".to_string(),
    //         TypePython::Bool(_) => "Bool".to_string(),
    //         TypePython::Media(media_type_go, _) => match media_type_go {
    //             MediaTypePython::Image => "Image".to_string(),
    //             MediaTypePython::Audio => "Audio".to_string(),
    //         },
    //         TypePython::Class { name, .. } => name.clone(),
    //         TypePython::Union { name, .. } => name.clone(),
    //         TypePython::Enum { name, .. } => name.clone(),
    //         TypePython::List(type_go, _) => format!("List{}", type_go.default_name_within_union()),
    //         TypePython::Map(key, value, _) => format!(
    //             "Map{}Key{}Value",
    //             key.default_name_within_union(),
    //             value.default_name_within_union()
    //         ),
    //         TypePython::Tuple(type_gos, _) => format!(
    //             "Tuple{}{}",
    //             type_gos.len(),
    //             type_gos
    //                 .iter()
    //                 .map(|t| t.default_name_within_union())
    //                 .collect::<Vec<_>>()
    //                 .join(", ")
    //         ),
    //         TypePython::Any { .. } => "Any".to_string(),
    //     }
    // }

    // pub fn zero_value(&self, pkg: &CurrentRenderModule) -> String {
    //     if matches!(self.meta().type_wrapper, TypeWrapper::Optional(_)) {
    //         return "nil".to_string();
    //     }
    //     match self {
    //         TypePython::String(_) => "\"\"".to_string(),
    //         TypePython::Int(_) => "0".to_string(),
    //         TypePython::Float(_) => "0.0".to_string(),
    //         TypePython::Bool(_) => "false".to_string(),
    //         TypePython::Media(..)
    //         | TypePython::Class { .. }
    //         | TypePython::Union { .. }
    //         | TypePython::Enum { .. } => {
    //             format!("{}{{}}", self.serialize_type(pkg))
    //         }
    //         TypePython::List(..) => "nil".to_string(),
    //         TypePython::Map(..) => "nil".to_string(),
    //         TypePython::Tuple(..) => "nil".to_string(),
    //         TypePython::Any { .. } => "nil".to_string(),
    //     }
    // }

    // fn cast_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderModule) -> String {
    //     format!("({param}).({})", self.serialize_type(pkg))
    //         .trim()
    //         .to_string()
    // }

    // fn cast_return_value(&self, pkg: &CurrentRenderModule) -> String {
    //     if self.meta().wrap_stream_state {
    //         format!(
    //             "{}{{Value: nil, State: StreamStatePending}}",
    //             self.serialize_type(pkg)
    //         )
    //     } else {
    //         self.zero_value(pkg)
    //     }
    // }

    // pub fn cast_from_any(&self, param: &str, pkg: &CurrentRenderModule) -> String {
    //     if self.meta().is_optional() {
    //         format!(
    //             r#"
    //             func(result any) {t} {{
    //                 if result == nil {{
    //                     return {return_value}
    //                 }}
    //                 return {casted}
    //             }}({param})
    //         "#,
    //             t = self.serialize_type(pkg),
    //             casted = self.cast_from_any_skip_optional("result", pkg),
    //             return_value = self.cast_return_value(pkg)
    //         )
    //     } else {
    //         self.cast_from_any_skip_optional(param, pkg)
    //     }
    //     .trim()
    //     .to_string()
    // }

    // pub fn cast_from_function(&self, param: &str, pkg: &CurrentRenderModule) -> String {
    //     match self {
    //         TypePython::List(..) | TypePython::Map(..) => {
    //             self.cast_from_any_skip_optional(param, pkg)
    //         }
    //         _ if self.meta().is_optional() => self.cast_from_any_skip_optional(param, pkg),
    //         _ => format!("*({param}).(*{})", self.serialize_type(pkg)),
    //     }
    // }

    // fn decode_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderModule) -> String {
    //     match self {
    //         TypePython::List(inner, meta) if !meta.is_optional() => format!(
    //             "baml.DecodeList({param}, func(inner *cffi.CFFIValueHolder) {t} {{
    //             return {casted}
    //         }})",
    //             t = inner.serialize_type(pkg),
    //             casted = inner.decode_from_any("inner", pkg)
    //         ),
    //         TypePython::Map(key, value, meta) if !meta.is_optional() => format!(
    //             "baml.DecodeMap({param}, func(inner *cffi.CFFIValueHolder) {t} {{
    //             return {casted}
    //         }})",
    //             t = value.serialize_type(pkg),
    //             casted = value.decode_from_any("inner", pkg)
    //         ),
    //         _ if !self.meta().is_optional() => {
    //             format!("*baml.Decode({param}).(*{})", self.serialize_type(pkg))
    //         }
    //         _ => format!("baml.Decode({param}).({})", self.serialize_type(pkg)),
    //     }
    //     .trim()
    //     .to_string()
    // }

    // pub fn decode_from_any(&self, param: &str, pkg: &CurrentRenderModule) -> String {
    //     if self.meta().is_optional() {
    //         format!(
    //             r#"
    //             func(param *cffi.CFFIValueHolder) {t} {{
    //                 decoded := baml.Decode(param)
    //                 return {casted}
    //             }}({param})
    //         "#,
    //             t = self.serialize_type(pkg),
    //             casted = self.cast_from_any("decoded", pkg)
    //         )
    //     } else {
    //         self.decode_from_any_skip_optional(param, pkg)
    //     }
    //     .trim()
    //     .to_string()
    // }

    pub fn meta(&self) -> &TypeMetaPython {
        match self {
            TypePython::String(meta) => meta,
            TypePython::Int(meta) => meta,
            TypePython::Float(meta) => meta,
            TypePython::Bool(meta) => meta,
            TypePython::Media(_, meta) => meta,
            TypePython::Class { meta, .. } => meta,
            TypePython::Union { meta, .. } => meta,
            TypePython::Enum { meta, .. } => meta,
            TypePython::List(_, meta) => meta,
            TypePython::Map(_, _, meta) => meta,
            TypePython::Tuple(_, meta) => meta,
            TypePython::Any { meta, .. } => meta,
        }
    }

    pub fn meta_mut(&mut self) -> &mut TypeMetaPython {
        match self {
            TypePython::String(meta) => meta,
            TypePython::Int(meta) => meta,
            TypePython::Float(meta) => meta,
            TypePython::Bool(meta) => meta,
            TypePython::Media(_, meta) => meta,
            TypePython::Class { meta, .. } => meta,
            TypePython::Union { meta, .. } => meta,
            TypePython::Enum { meta, .. } => meta,
            TypePython::List(_, meta) => meta,
            TypePython::Map(_, _, meta) => meta,
            TypePython::Tuple(_, meta) => meta,
            TypePython::Any { meta, .. } => meta,
        }
    }
}

pub trait SerializeType {
    fn serialize_type(&self, pkg: &CurrentRenderModule) -> String;
}

impl SerializeType for TypePython {
    fn serialize_type(&self, module: &CurrentRenderModule) -> String {
        let meta = self.meta();
        let type_str = match self {
            TypePython::String(_) => "str".to_string(),
            TypePython::Int(_) => "int".to_string(),
            TypePython::Float(_) => "float".to_string(),
            TypePython::Bool(_) => "bool".to_string(),
            TypePython::Media(media, _) => media.serialize_type(module),
            TypePython::Class { package, name, .. } => {
                format!("{}{}", package.relative_from(module), name)
            }
            TypePython::Union { package, name, .. } => {
                format!("{}{}", package.relative_from(module), name)
            }
            TypePython::Enum { package, name, .. } => {
                format!("{}{}", package.relative_from(module), name)
            }
            TypePython::List(inner, _) => format!("List[{}]", inner.serialize_type(pkg)),
            TypePython::Map(key, value, _) => {
                format!(
                    "Dict[{}]{}",
                    key.serialize_type(pkg),
                    value.serialize_type(pkg)
                )
            }
            TypePython::Tuple(types, _) => format!(
                "Tuple[{}]",
                types
                    .iter()
                    .map(|t| t.serialize_type(pkg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            TypePython::Any { .. } => "any".to_string(),
        };

        let serialized_string = meta.wrap_type((module, type_str));
        serialized_string
    }
}

impl SerializeType for MediaTypePython {
    fn serialize_type(&self, module: &CurrentRenderModule) -> String {
        match self {
            MediaTypePython::Image => {
                format!("{}.Image", Module::imported_base().relative_from(module))
            }
            MediaTypePython::Audio => {
                format!("{}.Audio", Module::imported_base().relative_from(pkg))
            }
        }
    }
}
