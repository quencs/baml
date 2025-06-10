use crate::package::{CurrentRenderPackage, Package};

pub enum LiteralType {
    String,
    Int,
    Float,
    Bool,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct TypeMetaGo {
    pub type_wrapper: TypeWrapper,
    pub wrap_stream_state: bool,
}

impl Default for TypeWrapper {
    fn default() -> Self {
        TypeWrapper::None
    }
}

impl TypeMetaGo {
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

impl Default for TypeMetaGo {
    fn default() -> Self {
        TypeMetaGo {
            type_wrapper: TypeWrapper::default(),
            wrap_stream_state: false,
        }
    }
}

trait WrapType {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String;
}

impl WrapType for TypeWrapper {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String {
        let (pkg, orig) = &params;
        match self {
            TypeWrapper::None => orig.clone(),
            TypeWrapper::Checked(inner) => format!(
                "{}Checked[{}]",
                Package::checked().relative_from(pkg),
                inner.wrap_type(params)
            ),
            TypeWrapper::Optional(inner) => format!("*{}", inner.wrap_type(params)),
        }
    }
}

impl WrapType for TypeMetaGo {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String {
        let pkg = params.0;
        let wrapped = self.type_wrapper.wrap_type(params);
        if self.wrap_stream_state {
            format!(
                "{}StreamState[{}]",
                Package::stream_state().relative_from(&pkg),
                wrapped
            )
        } else {
            wrapped
        }
    }
}

#[derive(Clone)]
pub enum MediaTypeGo {
    Image,
    Audio,
}

#[derive(Clone)]
pub enum TypeGo {
    String(TypeMetaGo),
    Int(TypeMetaGo),
    Float(TypeMetaGo),
    Bool(TypeMetaGo),
    Media(MediaTypeGo, TypeMetaGo),
    // unions become classes
    Class {
        package: Package,
        name: String,
        dynamic: bool,
        meta: TypeMetaGo,
    },
    Union {
        package: Package,
        name: String,
        meta: TypeMetaGo,
    },
    Enum {
        package: Package,
        name: String,
        dynamic: bool,
        meta: TypeMetaGo,
    },
    TypeAlias {
        name: String,
        package: Package,
        evaluates_to: Option<Box<TypeGo>>,
        meta: TypeMetaGo,
    },
    List(Box<TypeGo>, TypeMetaGo),
    Map(Box<TypeGo>, Box<TypeGo>, TypeMetaGo),
    Tuple(Vec<TypeGo>, TypeMetaGo),
    // For any type that we can't represent in Go, we'll use this
    Any {
        reason: String,
        meta: TypeMetaGo,
    },
}

impl TypeGo {
    // for unions, we need a default name for the type when the union is not named
    pub fn default_name_within_union(&self) -> String {
        match self {
            TypeGo::String(_) => "String".to_string(),
            TypeGo::Int(_) => "Int".to_string(),
            TypeGo::Float(_) => "Float".to_string(),
            TypeGo::Bool(_) => "Bool".to_string(),
            TypeGo::Media(media_type_go, _) => match media_type_go {
                MediaTypeGo::Image => "Image".to_string(),
                MediaTypeGo::Audio => "Audio".to_string(),
            },
            TypeGo::TypeAlias { name, .. } => name.clone(),
            TypeGo::Class { name, .. } => name.clone(),
            TypeGo::Union { name, .. } => name.clone(),
            TypeGo::Enum { name, .. } => name.clone(),
            TypeGo::List(type_go, _) => format!("List{}", type_go.default_name_within_union()),
            TypeGo::Map(key, value, _) => format!(
                "Map{}Key{}Value",
                key.default_name_within_union(),
                value.default_name_within_union()
            ),
            TypeGo::Tuple(type_gos, _) => format!(
                "Tuple{}{}",
                type_gos.len(),
                type_gos
                    .iter()
                    .map(|t| t.default_name_within_union())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TypeGo::Any { .. } => "Any".to_string(),
        }
    }

    pub fn zero_value(&self, pkg: &CurrentRenderPackage) -> String {
        if matches!(self.meta().type_wrapper, TypeWrapper::Optional(_)) {
            return "nil".to_string();
        }
        match self {
            TypeGo::String(_) => "\"\"".to_string(),
            TypeGo::Int(_) => "0".to_string(),
            TypeGo::Float(_) => "0.0".to_string(),
            TypeGo::Bool(_) => "false".to_string(),
            TypeGo::Media(..) |
            TypeGo::Class { .. } | TypeGo::Union { .. } | TypeGo::Enum { .. } => {
                format!("{}{{}}", self.serialize_type(pkg))
            }
            TypeGo::TypeAlias { evaluates_to, .. } => match evaluates_to {
                Some(evaluates_to) => evaluates_to.zero_value(pkg),
                None => format!("{}{{}}", self.serialize_type(pkg))
            },
            TypeGo::List(..) => "nil".to_string(),
            TypeGo::Map(..) => "nil".to_string(),
            TypeGo::Tuple(..) => "nil".to_string(),
            TypeGo::Any { .. } => "nil".to_string(),
        }
    }

    fn cast_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        format!("({param}).({})", self.serialize_type(pkg))
            .trim()
            .to_string()
    }

    pub fn cast_from_any(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        if self.meta().is_optional() {
            format!(
                r#"
                func(result any) {t} {{
                    if result == nil {{
                        return nil
                    }}
                    return {casted}
                }}({param})
            "#,
                t = self.serialize_type(pkg),
                casted = self.cast_from_any_skip_optional("result", pkg)
            )
        } else {
            self.cast_from_any_skip_optional(param, pkg)
        }
        .trim()
        .to_string()
    }

    pub fn cast_from_function(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        match self {
            TypeGo::List(..) | TypeGo::Map(..) => self.cast_from_any_skip_optional(param, pkg),
            TypeGo::TypeAlias { evaluates_to, meta, .. } => match evaluates_to {
                Some(evaluates_to) if evaluates_to.meta().is_optional() => 
                format!("({param}).({})", self.serialize_type(pkg)),
                _ => format!("*({param}).(*{})", self.serialize_type(pkg)),
            },
            _ if self.meta().is_optional() => self.cast_from_any_skip_optional(param, pkg),
            _ => format!("*({param}).(*{})", self.serialize_type(pkg)),
        }
    }

    fn decode_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        match self {
            TypeGo::List(inner, meta) if !meta.is_optional() => format!(
                "baml.DecodeList({param}, func(inner *cffi.CFFIValueHolder) {t} {{
                return {casted}
            }})",
                t = inner.serialize_type(pkg),
                casted = inner.decode_from_any("inner", pkg)
            ),
            TypeGo::Map(key, value, meta) if !meta.is_optional() => format!(
                "baml.DecodeMap({param}, func(inner *cffi.CFFIValueHolder) {t} {{
                return {casted}
            }})",
                t = value.serialize_type(pkg),
                casted = value.decode_from_any("inner", pkg)
            ),
            TypeGo::TypeAlias { name, package, evaluates_to, meta } if evaluates_to.as_ref().map(|e| e.meta().is_optional()).unwrap_or(false) => {
                format!(r#"
                func(param *cffi.CFFIValueHolder) {name} {{
                    decoded := baml.Decode(param)
                    if decoded == nil {{
                        return nil
                    }}
                    return decoded.({name})
                }}({param})
                "#, name= self.serialize_type(pkg))
            }
            _ if !self.meta().is_optional() => format!("*baml.Decode({param}).(*{})", self.serialize_type(pkg)),
            _ => format!("baml.Decode({param}).({})", self.serialize_type(pkg)),
        }
        .trim()
        .to_string()
    }

    pub fn decode_from_any(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        if self.meta().is_optional() {
            format!(
                r#"
                func(param *cffi.CFFIValueHolder) {t} {{
                    decoded := baml.Decode(param)
                    return {casted}
                }}({param})
            "#,
                t = self.serialize_type(pkg),
                casted = self.cast_from_any("decoded", pkg)
            )
        } else {
            self.decode_from_any_skip_optional(param, pkg)
        }
        .trim()
        .to_string()
    }

    pub fn meta(&self) -> &TypeMetaGo {
        match self {
            TypeGo::String(meta) => meta,
            TypeGo::Int(meta) => meta,
            TypeGo::Float(meta) => meta,
            TypeGo::Bool(meta) => meta,
            TypeGo::Media(_, meta) => meta,
            TypeGo::Class { meta, .. } => meta,
            TypeGo::TypeAlias { meta, .. } => meta,
            TypeGo::Union { meta, .. } => meta,
            TypeGo::Enum { meta, .. } => meta,
            TypeGo::List(_, meta) => meta,
            TypeGo::Map(_, _, meta) => meta,
            TypeGo::Tuple(_, meta) => meta,
            TypeGo::Any { meta, .. } => meta,
        }
    }

    pub fn meta_mut(&mut self) -> &mut TypeMetaGo {
        match self {
            TypeGo::String(meta) => meta,
            TypeGo::Int(meta) => meta,
            TypeGo::Float(meta) => meta,
            TypeGo::Bool(meta) => meta,
            TypeGo::Media(_, meta) => meta,
            TypeGo::Class { meta, .. } => meta,
            TypeGo::TypeAlias { meta, .. } => meta,
            TypeGo::Union { meta, .. } => meta,
            TypeGo::Enum { meta, .. } => meta,
            TypeGo::List(_, meta) => meta,
            TypeGo::Map(_, _, meta) => meta,
            TypeGo::Tuple(_, meta) => meta,
            TypeGo::Any { meta, .. } => meta,
        }
    }
}

pub trait SerializeType {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String;
}

impl SerializeType for TypeGo {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String {
        let meta = self.meta();
        let type_str = match self {
            TypeGo::String(_) => "string".to_string(),
            TypeGo::Int(_) => "int64".to_string(),
            TypeGo::Float(_) => "float64".to_string(),
            TypeGo::Bool(_) => "bool".to_string(),
            TypeGo::Media(media, _) => media.serialize_type(pkg),
            TypeGo::Class { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeGo::TypeAlias { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeGo::Union { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeGo::Enum { package, name, .. } => format!("{}{}", package.relative_from(pkg), name),
            TypeGo::List(inner, _) => format!("[]{}", inner.serialize_type(pkg)),
            TypeGo::Map(key, value, _) => {
                format!(
                    "map[{}]{}",
                    key.serialize_type(pkg),
                    value.serialize_type(pkg)
                )
            }
            TypeGo::Tuple(types, _) => format!(
                "({})",
                types
                    .iter()
                    .map(|t| t.serialize_type(pkg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            TypeGo::Any { .. } => "any".to_string(),
        };

        meta.wrap_type((pkg, type_str))
    }
}

impl SerializeType for MediaTypeGo {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String {
        match self {
            MediaTypeGo::Image => format!("{}.Image", Package::imported_base().relative_from(pkg)),
            MediaTypeGo::Audio => format!("{}.Audio", Package::imported_base().relative_from(pkg)),
        }
    }
}
