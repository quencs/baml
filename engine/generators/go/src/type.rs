pub enum LiteralType {
    String,
    Int,
    Float,
    Bool,
}

pub enum TypeWrapper {
    None,
    Checked(Box<TypeWrapper>),
    Optional(Box<TypeWrapper>),
}

pub struct TypeMetaGo {
    type_wrapper: TypeWrapper,
    wrap_stream_state: bool,
}

impl Default for TypeWrapper {
    fn default() -> Self {
        TypeWrapper::None
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
    fn wrap_type(&self, params: (&Package, String)) -> String;
}

impl WrapType for TypeWrapper {
    fn wrap_type(&self, params: (&Package, String)) -> String {
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
    fn wrap_type(&self, params: (&Package, String)) -> String {
        let pkg = params.0.clone();
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Package {
    package_path: Vec<String>,
}

impl Package {
    pub fn new(package: &str) -> Self {
        let parts: Vec<_> = package.split('.').map(|s| s.to_string()).collect();
        if parts.is_empty() {
            panic!("Package cannot be empty");
        }
        // ensure the first part is baml_client
        if parts[0] != "baml_client" && parts[0] != "baml" {
            panic!("Package must start with baml_client");
        }
        Package { package_path: parts }
    }

    pub fn relative_from(&self, other: &Package) -> String {
        // Go does wierd imports, so we return only the last part of the package
        // unless the other package is the same as self, in which case we return empty
        if self.package_path == other.package_path {
            return "".to_string();
        }
        return format!("{}.", self.package_path.last().unwrap());
    }

    pub fn checked() -> Package {
        Package::new("baml_client.types")
    }

    pub fn stream_state() -> Package {
        Package::new("baml_client.stream_types")
    }

    pub fn imported_base() -> Package {
        Package::new("baml")
    }
}


impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.package_path.join("."))
    }
}

pub enum MediaTypeGo {
    Image,
    Audio,
}

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
        meta: TypeMetaGo,
    },
    List(Box<TypeGo>, TypeMetaGo),
    Map(Box<TypeGo>, Box<TypeGo>, TypeMetaGo),
    Tuple(Vec<TypeGo>, TypeMetaGo),
    Checked(Box<TypeGo>, TypeMetaGo),
    // For any type that we can't represent in Go, we'll use this
    Any {
        reason: String,
        meta: TypeMetaGo,
    },
}

impl TypeGo {
    pub fn meta(&self) -> &TypeMetaGo {
        match self {
            TypeGo::String(meta) => meta,
            TypeGo::Int(meta) => meta,
            TypeGo::Float(meta) => meta,
            TypeGo::Bool(meta) => meta,
            TypeGo::Media(_, meta) => meta,
            TypeGo::Class { meta, .. } => meta,
            TypeGo::Union { meta, .. } => meta,
            TypeGo::Enum { meta, .. } => meta,
            TypeGo::List(_, meta) => meta,
            TypeGo::Map(_, _, meta) => meta,
            TypeGo::Tuple(_, meta) => meta,
            TypeGo::Checked(_, meta) => meta,
            TypeGo::Any { meta, .. } => meta,
        }
    }
}

pub trait SerializeType {
    fn serialize_type(&self, pkg: &Package) -> String;
}

impl SerializeType for TypeGo {
    fn serialize_type(&self, pkg: &Package) -> String {
        let meta = self.meta();
        let type_str = match self {
            TypeGo::String(_) => "string".to_string(),
            TypeGo::Int(_) => "int".to_string(),
            TypeGo::Float(_) => "float".to_string(),
            TypeGo::Bool(_) => "bool".to_string(),
            TypeGo::Media(media, _) => media.serialize_type(pkg),
            TypeGo::Class {
                package, name, ..
            } => format!("{}{}", package.relative_from(pkg), name),
            TypeGo::Union {
                package, name, ..
            } => format!("{}{}", package.relative_from(pkg), name),
            TypeGo::Enum {
                package, name, ..
            } => format!("{}{}", package.relative_from(pkg), name),
            TypeGo::List(inner, _) => format!("[]{}", inner.serialize_type(pkg)),
            TypeGo::Map(key, value, _) => {
                format!("map[{}]{}", key.serialize_type(pkg), value.serialize_type(pkg))
            }
            TypeGo::Tuple(types, _) => format!(
                "({})",
                types
                    .iter()
                    .map(|t| t.serialize_type(pkg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            TypeGo::Checked(inner, _) => {
                format!(
                    "{}Checked[{}]",
                    Package::checked().relative_from(pkg),
                    inner.serialize_type(pkg)
                )
            }
            TypeGo::Any { .. } => "any".to_string(),
        };

        meta.type_wrapper.wrap_type((pkg, type_str))
    }
}

impl SerializeType for MediaTypeGo {
    fn serialize_type(&self, pkg: &Package) -> String {
        match self {
            MediaTypeGo::Image => format!("{}.Image", Package::imported_base().relative_from(pkg)),
            MediaTypeGo::Audio => format!("{}.Audio", Package::imported_base().relative_from(pkg)),
        }
    }
}
