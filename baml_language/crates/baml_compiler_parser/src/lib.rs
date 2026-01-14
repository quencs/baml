//! Parsing BAML source into Rowan syntax trees.
//!
//! Implements incremental parsing with error recovery.

use baml_base::SourceFile;
use baml_compiler_diagnostics::ParseError;
use baml_compiler_lexer::lex_file;
use baml_compiler_syntax::SyntaxNode;
use rowan::GreenNode;

mod parser;
pub use parser::{parse_file, parse_file_with_cache};

/// Tracked struct that holds both parse outputs together
#[salsa::tracked]
pub struct ParseResult<'db> {
    #[tracked]
    pub green: GreenNode,

    #[tracked]
    pub errors: Vec<ParseError>,
}

/// Tracked: parse file and return both green tree and errors.
///
/// Note: We can't make this take Vec<Token> directly because Salsa tracked
/// functions can only take Salsa-tracked types as input. So we take `SourceFile`,
/// call `lex_file` (tracked), then call `parse_file` (not tracked) with the tokens.
#[salsa::tracked]
pub fn parse_result(db: &dyn salsa::Database, file: SourceFile) -> ParseResult<'_> {
    let tokens = lex_file(db, file);
    let (green, errors) = parse_file(&tokens);
    ParseResult::new(db, green, errors)
}

/// Get the green tree from parsing a file
pub fn parse_green(db: &dyn salsa::Database, file: SourceFile) -> GreenNode {
    let result = parse_result(db, file);
    result.green(db)
}

/// Get parse errors from parsing a file
pub fn parse_errors(db: &dyn salsa::Database, file: SourceFile) -> Vec<ParseError> {
    let result = parse_result(db, file);
    result.errors(db)
}

/// Helper to build a red tree from the green tree.
pub fn syntax_tree(db: &dyn salsa::Database, file: SourceFile) -> SyntaxNode {
    let green = parse_green(db, file);
    SyntaxNode::new_root(green)
}

#[cfg(test)]
mod tests {
    use baml_compiler_syntax::SyntaxKind;

    use super::*;

    /// Regression test for nested generics like `map<string, map<string, string>>`.
    /// The lexer produces `>>` as a single token, but the parser needs to split it
    /// into two `>` tokens for nested generic type arguments.
    #[test]
    fn test_nested_map_function() {
        let source = r##"function Foo(m: map<string, map<string, string>>) -> map<string, map<string, string>> {
  client GPT35
  prompt #"test"#
}"##;

        let tokens = baml_compiler_lexer::lex_lossless(source, baml_base::FileId::new(0));
        let (green, errors) = parse_file(&tokens);

        // Assert no parse errors (>> should be split correctly)
        assert!(
            errors.is_empty(),
            "Expected no parse errors for nested generics, got: {errors:?}"
        );

        // Verify the function has the expected structure
        let root = SyntaxNode::new_root(green);
        let func_def = root
            .children()
            .find(|c| c.kind() == SyntaxKind::FUNCTION_DEF)
            .expect("Should have FUNCTION_DEF");

        // Should have return type and body
        assert!(
            func_def
                .children()
                .any(|c| c.kind() == SyntaxKind::TYPE_EXPR),
            "Should have return TYPE_EXPR"
        );
        assert!(
            func_def
                .children()
                .any(|c| c.kind() == SyntaxKind::LLM_FUNCTION_BODY),
            "Should have LLM_FUNCTION_BODY"
        );
    }
}
