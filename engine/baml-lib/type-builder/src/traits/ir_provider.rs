use internal_baml_core::ir::repr::IntermediateRepr;

pub trait IRProvider: Send + Sync {
    fn get_ir(&self) -> &IntermediateRepr;
}
