use std::sync::Arc;
use dir_writer::IntermediateRepr;

#[derive(Clone)]
pub struct CurrentRenderPackage {
    pub package_name: String,
    pub ir: Arc<IntermediateRepr>,
    current_scope: String,
}

impl CurrentRenderPackage {
    pub fn new(package_name: impl Into<String>, ir: Arc<IntermediateRepr>) -> Self {
        let package_name = package_name.into();
        Self {
            package_name: package_name.clone(),
            ir,
            current_scope: package_name,
        }
    }

    pub fn set(&mut self, scope: impl Into<String>) {
        self.current_scope = scope.into();
    }

    pub fn current_scope(&self) -> &str {
        &self.current_scope
    }
}