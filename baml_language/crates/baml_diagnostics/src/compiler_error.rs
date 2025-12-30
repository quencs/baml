pub mod error_format;
pub mod name_error;
pub mod parse_error;
pub mod type_error;

use std::{collections::HashMap, fmt};

use ariadne::{Report, ReportKind, Source};
use baml_base::{FileId, Span};
pub use name_error::NameError;
pub use parse_error::ParseError;
pub use type_error::TypeError;

/// A cache for ariadne that can display filenames instead of file IDs.
pub struct SourceCache {
    sources: HashMap<FileId, Source<String>>,
    filenames: HashMap<FileId, String>,
}

impl SourceCache {
    /// Create a new source cache with sources and optional filenames.
    pub fn new(
        sources: HashMap<FileId, String>,
        filenames: Option<HashMap<FileId, String>>,
    ) -> Self {
        let sources = sources
            .into_iter()
            .map(|(id, text)| (id, Source::from(text)))
            .collect();
        SourceCache {
            sources,
            filenames: filenames.unwrap_or_default(),
        }
    }
}

/// A wrapper type for displaying file IDs as filenames.
struct FileIdDisplay {
    file_id: FileId,
    filename: Option<String>,
}

impl fmt::Display for FileIdDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.filename {
            write!(f, "{name}")
        } else {
            write!(f, "{}", self.file_id)
        }
    }
}

#[allow(refining_impl_trait)]
impl ariadne::Cache<FileId> for SourceCache {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&Source<Self::Storage>, Box<dyn fmt::Debug + '_>> {
        self.sources
            .get(id)
            .ok_or_else(|| Box::new(format!("Unknown file ID: {id}")) as Box<dyn fmt::Debug>)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<Box<dyn fmt::Display + 'a>> {
        // Return the filename if available, otherwise fall back to the file ID
        let filename = self.filenames.get(id).cloned();
        Some(Box::new(FileIdDisplay {
            file_id: *id,
            filename,
        }))
    }
}

/// Every compiler error that can occur in the compiler.
/// It is parameterized by several types that are owned by the different compiler phases,
/// which we don't want to collect in this module. We only care that those values can
/// be displayed (for rendering in the message).
pub enum CompilerError<Ty> {
    ParseError(ParseError),
    TypeError(TypeError<Ty>),
    NameError(NameError),
}

pub struct ErrorCode(u32);

/// Error codes are formated like E0001, (E followed by a number padded to 4 digits).
impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "E{:04}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorMode {
    Color,
    NoColor,
}

pub fn render_error<'a, Ty: std::fmt::Display>(
    color_mode: &ColorMode,
    err: CompilerError<Ty>,
) -> Report<'a, Span> {
    let (report_builder, code) = error_format::error_report_and_code(err);
    report_builder
        .with_config(ariadne::Config::default().with_color(*color_mode == ColorMode::Color))
        .with_note(format!("Error code: {code}"))
        .finish()
}

const TYPE_MISMATCH: ErrorCode = ErrorCode(1);
const UNKNOWN_TYPE: ErrorCode = ErrorCode(2);
const UNKNOWN_VARIABLE: ErrorCode = ErrorCode(3);
const INVALID_OPERATOR: ErrorCode = ErrorCode(4);
const ARGUMENT_COUNT_MISMATCH: ErrorCode = ErrorCode(5);
const NOT_CALLABLE: ErrorCode = ErrorCode(6);
const NO_SUCH_FIELD: ErrorCode = ErrorCode(7);
const NOT_INDEXABLE: ErrorCode = ErrorCode(8);

const UNEXPECTED_EOF: ErrorCode = ErrorCode(9);
const UNEXPECTED_TOKEN: ErrorCode = ErrorCode(10);
const DUPLICATE_NAME: ErrorCode = ErrorCode(11);
const NON_EXHAUSTIVE_MATCH: ErrorCode = ErrorCode(62);
const UNREACHABLE_ARM: ErrorCode = ErrorCode(63);

/// Render an ariadne Report to a String.
///
/// The `sources` map should contain the source text for each `FileId` referenced
/// in the report's spans. The optional `filenames` map provides display names for
/// file IDs - if not provided, file IDs are displayed as numbers.
pub fn render_report_to_string(
    report: &Report<'_, Span>,
    sources: &HashMap<FileId, String>,
    filenames: Option<&HashMap<FileId, String>>,
) -> String {
    let mut output = Vec::new();

    // Create a custom cache that can display filenames
    let mut cache = SourceCache::new(sources.clone(), filenames.cloned());

    report.write(&mut cache, &mut output).unwrap_or_else(|_| {
        // If writing fails, provide a fallback
        output.clear();
        output.extend_from_slice(b"<error rendering diagnostic>");
    });

    String::from_utf8_lossy(&output).into_owned()
}

/// Convenience function to render a `ParseError` directly to a string.
///
/// This combines `render_error` and `render_report_to_string` for the common case
/// of rendering parse errors. Pass `filenames` to display file names instead of IDs.
pub fn render_parse_error(
    error: &ParseError,
    sources: &HashMap<FileId, String>,
    filenames: Option<&HashMap<FileId, String>>,
    color: bool,
) -> String {
    let color_mode = if color {
        ColorMode::Color
    } else {
        ColorMode::NoColor
    };
    let compiler_error: CompilerError<String> = CompilerError::ParseError(error.clone());
    let report = render_error(&color_mode, compiler_error);
    render_report_to_string(&report, sources, filenames)
}

/// Convenience function to render a `TypeError` directly to a string.
///
/// This combines `render_error` and `render_report_to_string` for the common case
/// of rendering type errors. The type parameter `Ty` must implement `Display` and `Clone`.
/// Pass `filenames` to display file names instead of IDs.
pub fn render_type_error<Ty: std::fmt::Display + Clone>(
    error: &TypeError<Ty>,
    sources: &HashMap<FileId, String>,
    filenames: Option<&HashMap<FileId, String>>,
    color: bool,
) -> String {
    let color_mode = if color {
        ColorMode::Color
    } else {
        ColorMode::NoColor
    };
    let compiler_error: CompilerError<Ty> = CompilerError::TypeError(error.clone());
    let report = render_error(&color_mode, compiler_error);
    render_report_to_string(&report, sources, filenames)
}

/// Convenience function to render a `NameError` directly to a string.
///
/// This combines `render_error` and `render_report_to_string` for the common case
/// of rendering name resolution errors. Pass `filenames` to display file names instead of IDs.
pub fn render_name_error(
    error: &NameError,
    sources: &HashMap<FileId, String>,
    filenames: Option<&HashMap<FileId, String>>,
    color: bool,
) -> String {
    let color_mode = if color {
        ColorMode::Color
    } else {
        ColorMode::NoColor
    };
    // Use String as the type parameter since NameError doesn't use it
    let compiler_error: CompilerError<String> = CompilerError::NameError(error.clone());
    let report = render_error(&color_mode, compiler_error);
    render_report_to_string(&report, sources, filenames)
}
