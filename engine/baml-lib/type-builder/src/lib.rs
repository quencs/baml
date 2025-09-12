mod core;
mod ir_aware;
pub mod traits;

// Re-export commonly used types at the crate root
pub use core::{Meta, WithMeta};
pub use ir_aware::{
    ClassBuilder, ClassPropertyBuilder, EnumBuilder, EnumValueBuilder, TypeBuilder,
};
