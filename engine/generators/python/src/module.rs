#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    module_path: Vec<String>,
}

impl Module {
    pub fn new(module_name: &str) -> Self {
        let parts: Vec<_> = module_name.split('.').map(|s| s.to_string()).collect();
        if parts.is_empty() {
            panic!("Module name cannot be empty");
        }
        // ensure the first part is baml_client
        if parts[0] != "baml_client" && parts[0] != "baml" {
            panic!("Module must start with baml_client");
        }
        Module { module_path: parts }
    }

    pub fn relative_from(&self, other: &CurrentRenderModule) -> String {
        // Python does wierd imports, so we return only the last part of the module
        // unless the other module is the same as self, in which case we return empty
        let other = other.get();
        if self.module_path == other.module_path {
            return "".to_string();
        }
        return format!("{}.", self.module_path.last().unwrap());
    }

    pub fn checked() -> Module {
        Module::new("baml_client.types")
    }

    pub fn stream_state() -> Module {
        Module::new("baml_client.partial_types")
    }

    pub fn imported_base() -> Module {
        Module::new("baml_py")
    }
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.module_path.join("."))
    }
}

#[derive(Clone)]
pub(crate) struct CurrentRenderModule {
    module: std::sync::Arc<std::sync::Mutex<std::sync::Arc<Module>>>,
}

impl CurrentRenderModule {
    pub fn new(module_name: &str) -> Self {
        Self {
            module: std::sync::Arc::new(std::sync::Mutex::new(std::sync::Arc::new(Module::new(
                module_name,
            )))),
        }
    }

    pub fn get(&self) -> std::sync::Arc<Module> {
        self.module.lock().unwrap().clone()
    }

    pub fn set(&self, module_name: &str) {
        match self.module.lock() {
            Ok(mut orig) => {
                *orig = std::sync::Arc::new(Module::new(module_name));
            }
            Err(e) => {
                panic!("Failed to get modul: {}", e);
            }
        }
    }

    pub fn name(&self) -> String {
        self.get().module_path.last().unwrap().clone()
    }
}
