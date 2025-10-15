use baml_types::{ir_type::TypeGeneric, TypeValue};
use dir_writer::IntermediateRepr;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Package {
    package_path: Vec<String>,
}

impl Package {
    fn new(package: &str) -> Self {
        let parts: Vec<_> = package.split('.').map(|s| s.to_string()).collect();
        if parts.is_empty() {
            panic!("Package cannot be empty");
        }
        // ensure the first part is baml_client
        if parts[0] != "baml_client" && parts[0] != "baml" {
            panic!("Package must start with baml_client");
        }
        Package {
            package_path: parts,
        }
    }

    pub fn relative_from(&self, other: &CurrentRenderPackage) -> String {
        // For Rust, we use :: for module paths
        // Return the module path relative to current package
        let other = other.get();
        if self.package_path == other.package_path {
            return "".to_string();
        }

        // Convert baml_client.types to crate::types::
        let mut path = String::new();
        for (i, part) in self.package_path.iter().enumerate() {
            if i == 0 && part == "baml_client" {
                path.push_str("crate::");
            } else if i > 0 {
                path.push_str(part);
                path.push_str("::");
            }
        }
        path
    }

    pub fn current(&self) -> String {
        self.package_path.last().unwrap().clone()
    }

    pub fn types() -> Package {
        Package::new("baml_client.types")
    }

    pub fn stream_state() -> Package {
        Package::new("baml_client.stream_state")
    }

    pub fn checked() -> Package {
        Package::new("baml_client.checked")
    }

    pub fn functions() -> Package {
        Package::new("baml_client.functions")
    }
}

#[derive(Clone, Debug)]
pub struct CurrentRenderPackage {
    package: Arc<std::sync::Mutex<Arc<Package>>>,
    pub ir: Arc<IntermediateRepr>,
    recursive_aliases: Arc<HashSet<String>>,
}

impl CurrentRenderPackage {
    pub fn new(package_name: impl Into<String>, ir: Arc<IntermediateRepr>) -> Self {
        let package_name = package_name.into();
        let full_package = format!("baml_client.{}", package_name);
        let recursive_aliases = Arc::new(find_recursive_aliases(ir.as_ref()));
        Self {
            package: Arc::new(std::sync::Mutex::new(Arc::new(Package::new(&full_package)))),
            ir,
            recursive_aliases,
        }
    }

    pub fn get(&self) -> Arc<Package> {
        self.package.lock().unwrap().clone()
    }

    pub fn set(&self, package: &str) {
        match self.package.lock() {
            Ok(mut orig) => {
                *orig = Arc::new(Package::new(package));
            }
            Err(e) => {
                panic!("Failed to get package: {e}");
            }
        }
    }

    pub fn lookup(&self) -> &IntermediateRepr {
        self.ir.as_ref()
    }

    pub fn is_recursive_alias(&self, name: &str) -> bool {
        self.recursive_aliases.contains(name)
    }

    pub fn name(&self) -> String {
        self.get().current()
    }

    pub fn in_type_definition(&self) -> CurrentRenderPackage {
        let new_pkg = self.clone();
        new_pkg.set("baml_client.types");
        new_pkg
    }
}

fn find_recursive_aliases(ir: &IntermediateRepr) -> HashSet<String> {
    let mut recursive_aliases = HashSet::new();
    for cycle in ir.structural_recursive_alias_cycles() {
        let is_invalid_cycle = cycle.iter().all(|(_, field_type)| {
            field_type
                .find_if(
                    &|t| match t {
                        TypeGeneric::Class { .. } => true,
                        TypeGeneric::Enum { .. } => true,
                        TypeGeneric::Literal(..) => true,
                        TypeGeneric::Primitive(TypeValue::Null, ..) => false,
                        TypeGeneric::Primitive(..) => true,
                        _ => false,
                    },
                    true,
                )
                .is_empty()
        });

        if is_invalid_cycle {
            recursive_aliases.extend(cycle.keys().cloned());
        }
    }

    recursive_aliases
}
