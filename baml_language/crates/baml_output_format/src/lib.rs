//! Output format types and rendering for BAML schemas.
//!
//! This crate provides:
//! - `OutputFormatContent` - Container for all type schemas
//! - `OutputFormatOptions` - Options for rendering output format
//! - `render` - Render output format to a string

mod render;
mod render_options;
mod types;

pub use render::{RenderError, render};
pub use render_options::{
    HoistClasses, INLINE_RENDER_ENUM_MAX_VALUES, MapStyle, OutputFormatOptions, RenderSetting,
};
pub use types::{
    Class, ClassField, Enum, EnumVariant, Name, OutputFormatBuilder, OutputFormatContent,
};
