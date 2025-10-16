use super::IRProvider;
use internal_baml_parser_database::ParserDatabase;

/// Extended trait for IR providers that also have runtime capabilities
/// This is implemented by InternalBamlRuntime to provide full type builder functionality
pub trait RuntimeProvider: IRProvider {
    /// Get access to the parser database for BAML validation
    fn get_db(&self) -> &ParserDatabase;

    /// Clone the parser database for scoped modifications
    fn clone_db(&self) -> ParserDatabase;
}
