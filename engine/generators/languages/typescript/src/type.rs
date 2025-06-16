use baml_types::baml_value::TypeLookups;

use crate::package::{CurrentRenderPackage, Package};

#[derive(Clone, PartialEq, Debug, Default)]
pub enum TypeWrapper {
    #[default]
    None,
    Checked(Box<TypeWrapper>),
    Optional(Box<TypeWrapper>),
}

impl TypeWrapper {
    pub fn wrap_with_checked(self) -> TypeWrapper {
        TypeWrapper::Checked(Box::new(self))
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct TypeMetaTS {
    pub type_wrapper: TypeWrapper,
    pub wrap_stream_state: bool,
}

impl TypeMetaTS {
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

trait WrapType {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String;
}

impl WrapType for TypeWrapper {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String {
        let (pkg, orig) = &params;
        match self {
            TypeWrapper::None => orig.clone(),
            TypeWrapper::Checked(inner) => format!(
                "{}Checked<{}>",
                Package::checked().relative_from(pkg),
                inner.wrap_type(params)
            ),
            TypeWrapper::Optional(inner) => format!("{} | null", inner.wrap_type(params)),
        }
    }
}

impl WrapType for TypeMetaTS {
    fn wrap_type(&self, params: (&CurrentRenderPackage, String)) -> String {
        let pkg = params.0;
        let wrapped = self.type_wrapper.wrap_type(params);
        if self.wrap_stream_state {
            format!(
                "{}StreamState<{}>",
                Package::stream_state().relative_from(pkg),
                wrapped
            )
        } else {
            wrapped
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum MediaTypeTS {
    Image,
    Audio,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TypeTS {
    String(TypeMetaTS),
    Int(TypeMetaTS),
    Float(TypeMetaTS),
    Bool(TypeMetaTS),
    Media(MediaTypeTS, TypeMetaTS),
    // unions become classes
    Class {
        package: Package,
        name: String,
        dynamic: bool,
        meta: TypeMetaTS,
    },
    Union {
        package: Package,
        name: String,
        meta: TypeMetaTS,
    },
    Enum {
        package: Package,
        name: String,
        dynamic: bool,
        meta: TypeMetaTS,
    },
    TypeAlias {
        name: String,
        package: Package,
        meta: TypeMetaTS,
    },
    List(Box<TypeTS>, TypeMetaTS),
    Map(Box<TypeTS>, Box<TypeTS>, TypeMetaTS),
    // For any type that we can't represent in Go, we'll use this
    Any {
        reason: String,
        meta: TypeMetaTS,
    },
}

impl TypeTS {
    // for unions, we need a default name for the type when the union is not named
    pub fn default_name_within_union(&self) -> String {
        match self {
            TypeTS::String(_) => "string".to_string(),
            TypeTS::Int(_) => "number".to_string(),
            TypeTS::Float(_) => "number".to_string(),
            TypeTS::Bool(_) => "boolean".to_string(),
            TypeTS::Media(media_type, _) => match media_type {
                MediaTypeTS::Image => "Image".to_string(),
                MediaTypeTS::Audio => "Audio".to_string(),
            },
            TypeTS::TypeAlias { name, .. } => name.clone(),
            TypeTS::Class { name, .. } => name.clone(),
            TypeTS::Union { name, .. } => name.clone(),
            TypeTS::Enum { name, .. } => name.clone(),
            TypeTS::List(type_, _) => format!("Array<{}>", type_.default_name_within_union()),
            TypeTS::Map(key, value, _) => format!(
                "Record<{}, {}>",
                key.default_name_within_union(),
                value.default_name_within_union()
            ),
            TypeTS::Any { .. } => "any".to_string(),
        }
    }

    pub fn zero_value(&self, pkg: &CurrentRenderPackage) -> String {
        if matches!(self.meta().type_wrapper, TypeWrapper::Optional(_)) {
            return "null".to_string();
        }
        match self {
            TypeTS::String(_) => "\"\"".to_string(),
            TypeTS::Int(_) => "0".to_string(),
            TypeTS::Float(_) => "0.0".to_string(),
            TypeTS::Bool(_) => "false".to_string(),
            TypeTS::Media(..) | TypeTS::Class { .. } | TypeTS::Union { .. } => {
                format!("{{}}")
            }
            TypeTS::Enum { .. } => {
                format!("null")
            }
            TypeTS::TypeAlias { name, package, .. } => {
                let lookup = pkg.lookup();
                match lookup.expand_recursive_type(name) {
                    Ok(expansion) => {
                        if package == &Package::types() {
                            crate::ir_to_ts::type_to_ts(expansion, lookup).zero_value(pkg)
                        } else {
                            crate::ir_to_ts::stream_type_to_ts(
                                &expansion.partialize(lookup),
                                lookup,
                            )
                            .zero_value(pkg)
                        }
                    }
                    Err(_) => format!("{}{{}}", self.serialize_type(pkg)),
                }
            }
            TypeTS::List(..) => "[]".to_string(),
            TypeTS::Map(..) => "{{}}".to_string(),
            TypeTS::Any { .. } => "null".to_string(),
        }
    }

    fn cast_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        format!("({param}).({})", self.serialize_type(pkg))
            .trim()
            .to_string()
    }

    fn cast_return_value(&self, pkg: &CurrentRenderPackage) -> String {
        if self.meta().wrap_stream_state {
            format!(
                "{}{{Value: null, State: StreamStatePending}}",
                self.serialize_type(pkg)
            )
        } else {
            self.zero_value(pkg)
        }
    }

    pub fn cast_from_any(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        if self.meta().is_optional() {
            format!(
                r#"
                func(result any) {t} {{
                    if result == null {{
                        return {return_value}
                    }}
                    return {casted}
                }}({param})
            "#,
                t = self.serialize_type(pkg),
                casted = self.cast_from_any_skip_optional("result", pkg),
                return_value = self.cast_return_value(pkg)
            )
        } else {
            self.cast_from_any_skip_optional(param, pkg)
        }
        .trim()
        .to_string()
    }

    pub fn cast_from_function(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        match self {
            TypeTS::List(..) | TypeTS::Map(..) => self.cast_from_any_skip_optional(param, pkg),
            TypeTS::TypeAlias { name, .. } => {
                let lookup = pkg.lookup();
                match lookup.expand_recursive_type(name) {
                    Ok(expansion) if expansion.is_optional() => format!(
                        r#"
                            func(result any) {t} {{
                                if result == null {{
                                    return null
                                }}
                                return (result).({t})
                            }}({param})
                        "#,
                        t = self.serialize_type(pkg),
                    )
                    .trim()
                    .to_string(),
                    _ => format!("*({param}).(*{})", self.serialize_type(pkg)),
                }
            }
            _ if self.meta().is_optional() => self.cast_from_any_skip_optional(param, pkg),
            _ => format!("*({param}).(*{})", self.serialize_type(pkg)),
        }
    }

    fn decode_from_any_skip_optional(&self, param: &str, pkg: &CurrentRenderPackage) -> String {
        match self {
            TypeTS::List(inner, meta) if !meta.is_optional() => format!(
                "baml.DecodeList({param}, func(inner *cffi.CFFIValueHolder) {t} {{
                return {casted}
            }})",
                t = inner.serialize_type(pkg),
                casted = inner.decode_from_any("inner", pkg)
            ),
            TypeTS::Map(key, value, meta) if !meta.is_optional() => format!(
                "baml.DecodeMap({param}, func(inner *cffi.CFFIValueHolder) {t} {{
                return {casted}
            }})",
                t = value.serialize_type(pkg),
                casted = value.decode_from_any("inner", pkg)
            ),
            TypeTS::TypeAlias { name, .. } => {
                if pkg
                    .lookup()
                    .expand_recursive_type(name)
                    .map(|e| e.is_optional())
                    .unwrap_or(false)
                {
                    format!(
                        r#"
                    func(param *cffi.CFFIValueHolder) {name} {{
                        decoded := baml.Decode(param)
                        if decoded == null {{
                            return null
                        }}
                        return decoded.({name})
                    }}({param})
                    "#,
                        name = self.serialize_type(pkg)
                    )
                } else {
                    format!("*baml.Decode({param}).(*{})", self.serialize_type(pkg))
                }
            }
            _ if !self.meta().is_optional() => {
                format!("*baml.Decode({param}).(*{})", self.serialize_type(pkg))
            }
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

    pub fn meta(&self) -> &TypeMetaTS {
        match self {
            TypeTS::String(meta) => meta,
            TypeTS::Int(meta) => meta,
            TypeTS::Float(meta) => meta,
            TypeTS::Bool(meta) => meta,
            TypeTS::Media(_, meta) => meta,
            TypeTS::Class { meta, .. } => meta,
            TypeTS::TypeAlias { meta, .. } => meta,
            TypeTS::Union { meta, .. } => meta,
            TypeTS::Enum { meta, .. } => meta,
            TypeTS::List(_, meta) => meta,
            TypeTS::Map(_, _, meta) => meta,
            TypeTS::Any { meta, .. } => meta,
        }
    }

    pub fn meta_mut(&mut self) -> &mut TypeMetaTS {
        match self {
            TypeTS::String(meta) => meta,
            TypeTS::Int(meta) => meta,
            TypeTS::Float(meta) => meta,
            TypeTS::Bool(meta) => meta,
            TypeTS::Media(_, meta) => meta,
            TypeTS::Class { meta, .. } => meta,
            TypeTS::TypeAlias { meta, .. } => meta,
            TypeTS::Union { meta, .. } => meta,
            TypeTS::Enum { meta, .. } => meta,
            TypeTS::List(_, meta) => meta,
            TypeTS::Map(_, _, meta) => meta,
            TypeTS::Any { meta, .. } => meta,
        }
    }

    pub fn with_meta(mut self, meta: TypeMetaTS) -> Self {
        *(self.meta_mut()) = meta;
        self
    }
}

pub trait SerializeType {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String;
}

impl SerializeType for TypeTS {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String {
        let meta = self.meta();
        let type_str = match self {
            TypeTS::String(_) => "string".to_string(),
            TypeTS::Int(_) => "number".to_string(),
            TypeTS::Float(_) => "number".to_string(),
            TypeTS::Bool(_) => "boolean".to_string(),
            TypeTS::Media(media, _) => match media {
                MediaTypeTS::Image => "Image".to_string(),
                MediaTypeTS::Audio => "Audio".to_string(),
            },
            TypeTS::Class { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeTS::TypeAlias { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeTS::Union { package, name, .. } => {
                format!("{}{}", package.relative_from(pkg), name)
            }
            TypeTS::Enum { package, name, .. } => format!("{}{}", package.relative_from(pkg), name),
            TypeTS::List(inner, _) => format!("Array<{}>", inner.serialize_type(pkg)),
            TypeTS::Map(key, value, _) => {
                format!(
                    "Record<{}, {}>",
                    key.serialize_type(pkg),
                    value.serialize_type(pkg)
                )
            }
            TypeTS::Any { .. } => "undefined".to_string(),
        };

        meta.wrap_type((pkg, type_str))
    }
}

impl SerializeType for MediaTypeTS {
    fn serialize_type(&self, pkg: &CurrentRenderPackage) -> String {
        match self {
            MediaTypeTS::Image => format!("{}.Image", Package::imported_base().relative_from(pkg)),
            MediaTypeTS::Audio => format!("{}.Audio", Package::imported_base().relative_from(pkg)),
        }
    }
}
