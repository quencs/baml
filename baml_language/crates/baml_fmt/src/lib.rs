use std::{collections::HashMap, iter::Peekable};

use baml_base::SourceFile;
use baml_lexer::lex_file;
use baml_parser::parse_file;
use baml_syntax::{
    SyntaxKind, SyntaxNode, ast::ClassDef as AstClassDef, ast::EnumDef as AstEnumDef,
    ast::EnumVariant as AstEnumVariant, ast::Field as AstField, ast::Item as AstItem,
    ast::SourceFile as AstSourceFile,
};
use rowan::{TextRange, TextSize, ast::AstNode};

/// Entry point for formatting a BAML source file.
/// Returns None if the file has parse errors.
#[salsa::tracked]
pub fn format_file(db: &dyn salsa::Database, file: SourceFile) -> Option<String> {
    let tokens = lex_file(db, file);
    let (green, errors) = parse_file(&tokens);

    // Only format files with valid CST
    if !errors.is_empty() {
        return None;
    }

    let syntax_tree = SyntaxNode::new_root(green.clone());
    let formatter = Formatter::new(syntax_tree);

    formatter.format()
}

struct Formatter {
    indent_level: usize,
    last_pos: TextSize,
    output: String,
    root: SyntaxNode,
}

impl Formatter {
    /// Create a new formatter.
    fn new(root: SyntaxNode) -> Self {
        Self {
            indent_level: 0,
            last_pos: TextSize::new(0),
            output: String::new(),
            root,
        }
    }

    /// Push a formatted range to be added to the output, also prepends text from ranges missing in the AST if necessary.
    fn push_format(&mut self, range: TextRange, text: String) {
        self.format_missing(range.start());
        self.push_text(text);
        self.last_pos = range.end();
    }

    fn push_format_indent(&mut self, range: TextRange, text: String) {
        self.format_missing(range.start());
        self.push_text(format!("\n{}{}", self.gen_indent(), text));
        self.last_pos = range.end();
    }

    /// Push a text to be added to the output.
    fn push_text(&mut self, text: String) {
        self.output.push_str(&text);
    }

    /// Prepends text from ranges missing in the AST if necessary.
    fn format_missing(&mut self, start: TextSize) {
        if self.last_pos < start {
            let mut current_pos = self.last_pos;

            // iterate through all tokens in the missing range
            let mut on_same_line = self.last_pos != TextSize::new(0); // first line of file is always a separate line comment
            while current_pos < start {
                let token = self.root.token_at_offset(current_pos).right_biased();

                if let Some(token) = token {
                    // check if token is within our target range and fix trivia if necessary
                    if token.text_range().start() < start {
                        match token.kind() {
                            SyntaxKind::NEWLINE => on_same_line = false,
                            SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT => {
                                if !on_same_line {
                                    self.push_text(format!("\n{}", self.gen_indent()));
                                } else {
                                    self.push_text(" ".to_string());
                                }

                                self.push_text(token.text().to_string());
                            }
                            _ => (), // throw away all other tokens
                        }
                        current_pos = token.text_range().end();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        self.last_pos = start;
    }

    fn gen_indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn nest<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.indent_level += 1;
        f(self);
        self.indent_level -= 1;
    }

    /// Format the provided syntax tree. Consumes the formatter.
    fn format(mut self) -> Option<String> {
        let file = AstSourceFile::cast(self.root.clone())?;
        let mut first_item = true;
        for item in file.items() {
            self.format_item(item, first_item);
            first_item = false;
        }

        // TODO: do cleanup and grab hanging trivia here

        Some(self.output.clone())
    }

    /// Format an AST item.
    fn format_item(&mut self, item: AstItem, first_item: bool) {
        match item {
            AstItem::Enum(enum_def) => self.format_enum_def(enum_def, first_item),
            _ => todo!(),
        }
    }

    /// Format an AST enum definition.
    fn format_enum_def(&mut self, enum_def: AstEnumDef, first_item: bool) {
        // make sure to add a double newline between items if we're not the first item
        let keyword = enum_def.keyword().unwrap();
        self.push_format_indent(
            keyword.text_range(),
            format!("enum {} {{", enum_def.name().unwrap().text()),
        );

        self.nest(|f| {
            for variant in enum_def.variants() {
                let variant_name = variant.name().unwrap();
                f.push_format_indent(variant_name.text_range(), variant_name.text().to_string());
            }
        });

        let r_brace = enum_def.r_brace().unwrap();
        self.push_format_indent(r_brace.text_range(), "}".to_string());
    }
}
