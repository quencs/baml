use std::iter::Peekable;

use baml_base::SourceFile;
use baml_lexer::lex_file;
use baml_parser::parse_file;
use baml_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};
use rowan::{NodeOrToken, TextRange, TextSize};

/// Entry point for formatting a BAML source file.
/// Returns None if the file has parse errors.
#[salsa::tracked]
pub fn format_file(db: &dyn salsa::Database, file: SourceFile) -> Option<String> {
    let tokens = lex_file(db, file);
    let (green, errors) = parse_file(&tokens);

    // only format files with valid CST
    if !errors.is_empty() {
        return None;
    }

    let syntax_tree = SyntaxNode::new_root(green);
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
    fn push_format(&mut self, range: TextRange, text: &str, prepend_newline_and_indent: bool) {
        self.format_missing(range.start());
        if prepend_newline_and_indent {
            self.push_text(&format!("\n{}", self.gen_indent()));
        }
        self.push_text(text);
        self.last_pos = range.end();
    }

    /// Push a text to be added to the output.
    fn push_text(&mut self, text: &str) {
        self.output.push_str(text);
    }

    /// Prepends text from ranges missing in the AST if necessary.
    fn format_missing(&mut self, start: TextSize) {
        if self.last_pos < start {
            let mut current_pos = self.last_pos;

            // iterate through all tokens in the missing range
            let mut on_same_line = self.last_pos != TextSize::new(0); // first line of file is always a separate line comment
            let mut already_pushed_semicolon = false;
            let mut consecutive_newlines = 0;
            while current_pos < start {
                let token = self.root.token_at_offset(current_pos).right_biased();

                let Some(token) = token else {
                    break;
                };

                // check if token is within our target range
                if token.text_range().start() >= start {
                    break;
                }

                let kind = token.kind();

                // fix trivia if necessary
                match kind {
                    SyntaxKind::NEWLINE => {
                        on_same_line = false;
                        consecutive_newlines += 1;
                    }
                    SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT => {
                        if !on_same_line {
                            self.push_text(&format!("\n{}", self.gen_indent()));
                        } else {
                            self.push_text(" ");
                        }

                        self.push_text(token.text());
                    }
                    // get rid of duplicate semicolons
                    SyntaxKind::SEMICOLON => {
                        if !already_pushed_semicolon {
                            self.push_text(";");
                            already_pushed_semicolon = true;
                        }
                    }
                    _ => (), // throw away all other tokens
                }

                // if it's not a blank line, reset the consecutive newlines count
                if kind != SyntaxKind::NEWLINE && kind != SyntaxKind::WHITESPACE {
                    if consecutive_newlines > 1 {
                        self.push_text("\n");
                    }

                    consecutive_newlines = 0;
                }

                current_pos = token.text_range().end();
            }

            // clean up any remaining consecutive newlines
            if consecutive_newlines > 1 {
                self.push_text("\n");
            }
        }

        self.last_pos = start;
    }

    /// Generates a string of the current prepend_newline_and_indent level.
    fn gen_indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    /// Scan until we reach the specified kind, and call the provided function with the node.
    /// Returns true if the node was found, false if it was not found.
    fn format_node(
        &mut self,
        children: &mut Peekable<impl Iterator<Item = NodeOrToken<SyntaxNode, SyntaxToken>>>,
        kind: SyntaxKind,
        prepend_newline_and_indent: bool,
        f: impl FnOnce(&mut Formatter, SyntaxNode, bool),
    ) -> bool {
        let Some(node) = children.by_ref().find(|child| child.kind() == kind) else {
            return false;
        };

        f(self, node.into_node().unwrap(), prepend_newline_and_indent);
        true
    }

    /// Scan until we reach the specified kind, and call the provided function with the node.
    /// Also will halt early if the stop kind is reached, not consuming the stop kind token.
    /// Returns true if the node was found, false if it was not found or the stop kind was reached.
    fn format_node_stop(
        &mut self,
        children: &mut Peekable<impl Iterator<Item = NodeOrToken<SyntaxNode, SyntaxToken>>>,
        kind: SyntaxKind,
        prepend_newline_and_indent: bool,
        f: impl FnOnce(&mut Formatter, SyntaxNode, bool),
        stop_kind: SyntaxKind,
    ) -> bool {
        while let Some(child) = children.peek() {
            if child.kind() == stop_kind {
                break;
            } else if child.kind() == kind {
                f(
                    self,
                    children.next().unwrap().into_node().unwrap(),
                    prepend_newline_and_indent,
                );
                return true;
            } else {
                children.next();
            }
        }

        false
    }

    /// Scan until we reach the specified kind, and call the provided function with the token.
    /// Returns true if the token was found, false if it was not found.
    fn format_token(
        &mut self,
        children: &mut Peekable<impl Iterator<Item = NodeOrToken<SyntaxNode, SyntaxToken>>>,
        kind: SyntaxKind,
        prepend_newline_and_indent: bool,
        f: impl FnOnce(&mut Formatter, SyntaxToken, bool),
    ) -> bool {
        let Some(token) = children.by_ref().find(|child| child.kind() == kind) else {
            return false;
        };

        f(
            self,
            token.into_token().unwrap(),
            prepend_newline_and_indent,
        );
        true
    }

    /// Scan until we reach the specified kind, and call the provided function with the token.
    /// Also will halt early if the stop kind is reached, not consuming the stop kind token.
    /// Returns true if the token was found, false if it was not found or the stop kind was reached.
    fn format_token_stop(
        &mut self,
        children: &mut Peekable<impl Iterator<Item = NodeOrToken<SyntaxNode, SyntaxToken>>>,
        kind: SyntaxKind,
        prepend_newline_and_indent: bool,
        f: impl FnOnce(&mut Formatter, SyntaxToken, bool),
        stop_kind: SyntaxKind,
    ) -> bool {
        while let Some(child) = children.peek() {
            if child.kind() == stop_kind {
                break;
            } else if child.kind() == kind {
                f(
                    self,
                    children.next().unwrap().into_token().unwrap(),
                    prepend_newline_and_indent,
                );
                return true;
            } else {
                children.next();
            }
        }

        false
    }

    /// Formats a type expression.
    fn format_type_expr(&mut self, node: SyntaxNode, mut prepend_newline_and_indent: bool) {
        let ref mut children = node.children_with_tokens().peekable();

        // loop through and format the rest of the children
        while let Some(child) = children.next() {
            match child.kind() {
                // allow these tokens to be included in the output
                SyntaxKind::WORD
                | SyntaxKind::QUESTION
                | SyntaxKind::L_BRACKET
                | SyntaxKind::R_BRACKET
                | SyntaxKind::L_PAREN
                | SyntaxKind::R_PAREN => self.format_token_plaintext(
                    child.into_token().unwrap(),
                    prepend_newline_and_indent,
                ),
                // also allow union pipe, but give it spaces around it
                SyntaxKind::PIPE => {
                    self.push_text(" ");
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                    self.push_text(" ");
                }
                // allow comma for generics/tuples, but give a space after it
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                    self.push_text(" ");
                }
                // allow these for TypeScript-style types
                SyntaxKind::STRING_LITERAL
                | SyntaxKind::RAW_STRING_LITERAL
                | SyntaxKind::INTEGER_LITERAL
                | SyntaxKind::FLOAT_LITERAL => {
                    self.format_literal(child, prepend_newline_and_indent)
                }
                SyntaxKind::TYPE_ARGS => {
                    self.format_type_args(child.into_node().unwrap(), prepend_newline_and_indent)
                }
                // recurse for nesting in parentheses
                SyntaxKind::TYPE_EXPR => {
                    self.format_type_expr(child.into_node().unwrap(), prepend_newline_and_indent)
                }
                // ignore everything else - comments, whitespace, etc.
                _ => (),
            }

            prepend_newline_and_indent = false; // i think this breaks for whitespace, TODO: fix? there may not be a case in which a type expr should be newlined...
        }
    }

    /// Formats type arguments.
    fn format_type_args(&mut self, type_args: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = type_args.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::LESS,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::GREATER => self.format_token_plaintext(
                    child.into_token().unwrap(),
                    prepend_newline_and_indent,
                ),
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                    self.push_text(" ");
                }
                SyntaxKind::TYPE_EXPR => {
                    self.format_type_expr(child.into_node().unwrap(), prepend_newline_and_indent)
                }
                _ => (),
            }
        }
    }

    /// Formats a parameter list, including the parentheses. e.g. "(x: int, y: string)"
    fn format_parameter_list(
        &mut self,
        parameter_list: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = parameter_list.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_PAREN,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::R_PAREN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                SyntaxKind::PARAMETER => self.format_parameter(child.into_node().unwrap(), false),
                _ => (),
            }
        }
    }

    /// Formats a parameter. eg. "x: int"
    fn format_parameter(&mut self, parameter: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = parameter.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        self.format_token(
            children,
            SyntaxKind::COLON,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::TYPE_EXPR,
            false,
            Self::format_type_expr,
        );
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
        for child in self.root.children() {
            match child.kind() {
                SyntaxKind::ENUM_DEF => self.format_enum_def(child, true),
                SyntaxKind::CLASS_DEF => self.format_class_def(child, true),
                SyntaxKind::FUNCTION_DEF => self.format_function_def(child, true),
                SyntaxKind::CLIENT_DEF => self.format_client_def(child, true),
                SyntaxKind::TEST_DEF => self.format_test_def(child, true),
                SyntaxKind::RETRY_POLICY_DEF => self.format_retry_policy_def(child, true),
                SyntaxKind::TEMPLATE_STRING_DEF => self.format_template_string_def(child, true),
                SyntaxKind::TYPE_ALIAS_DEF => self.format_type_alias_def(child, true),
                _ => (),
            }
        }

        // grab hanging trivia at the end of the file
        self.format_missing(self.root.text_range().end());

        Some(self.output.clone())
    }

    fn format_type_alias_def(
        &mut self,
        type_alias_def: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = type_alias_def.children_with_tokens().peekable();

        // format type alias keyword, which is not actually a keyword
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format type alias name
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::EQUALS,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::TYPE_EXPR,
            false,
            Self::format_type_expr,
        );
    }

    fn format_template_string_def(
        &mut self,
        template_string_def: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = template_string_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_TEMPLATE_STRING,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );

        self.format_node(
            children,
            SyntaxKind::PARAMETER_LIST,
            false,
            Self::format_parameter_list,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::RAW_STRING_LITERAL,
            false,
            Self::format_string_literal,
        );
    }

    fn format_retry_policy_def(
        &mut self,
        retry_policy_def: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = retry_policy_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_RETRY_POLICY,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::CONFIG_BLOCK,
            false,
            Self::format_config_block,
        );
    }

    fn format_test_def(&mut self, test_def: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = test_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_TEST,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::CONFIG_BLOCK,
            false,
            Self::format_config_block,
        );
    }

    fn format_token_plaintext(&mut self, token: SyntaxToken, prepend_newline_and_indent: bool) {
        self.push_format(
            token.text_range(),
            &token.text().to_string(),
            prepend_newline_and_indent,
        );
    }

    fn format_client_def(&mut self, client_def: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = client_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_CLIENT,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        self.format_node(
            children,
            SyntaxKind::CLIENT_TYPE,
            false,
            Self::format_client_type,
        );
        self.push_text(" ");

        // format client name
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::CONFIG_BLOCK,
            false,
            Self::format_config_block,
        );
    }

    fn format_config_block(&mut self, config_block: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = config_block.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        self.nest(|f| {
            while f.format_node_stop(
                children,
                SyntaxKind::CONFIG_ITEM,
                true,
                Self::format_config_item,
                SyntaxKind::R_BRACE,
            ) {}

            f.format_missing(config_block.text_range().end()); // handle hanging trivia at the end of the config block
        });

        self.format_token(
            children,
            SyntaxKind::R_BRACE,
            true,
            Self::format_token_plaintext,
        );
    }

    fn format_config_item(&mut self, config_item: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = config_item.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // TODO: known issue: unquoted strings get an extra space here

        self.format_node_stop(
            children,
            SyntaxKind::CONFIG_BLOCK,
            false,
            Self::format_config_block,
            SyntaxKind::CONFIG_VALUE,
        );

        self.format_node(
            children,
            SyntaxKind::CONFIG_VALUE,
            false,
            Self::format_config_value,
        );
    }

    fn format_config_value(
        &mut self,
        config_value: SyntaxNode,
        mut prepend_newline_and_indent: bool,
    ) {
        let ref mut children = config_value.children_with_tokens().peekable();

        let mut seen_first_whitespace = false;
        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::WHITESPACE => {
                    if !seen_first_whitespace {
                        self.push_format(child.text_range(), " ", prepend_newline_and_indent); // collapse the first one to a single space
                        seen_first_whitespace = true;
                    } else {
                        self.format_token_plaintext(
                            child.into_token().unwrap(),
                            prepend_newline_and_indent,
                        );
                    }
                }
                SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => self
                    .format_string_literal(child.into_node().unwrap(), prepend_newline_and_indent),
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                }
                SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => {
                    self.format_literal(child, prepend_newline_and_indent)
                }
                SyntaxKind::NEWLINE => {
                    break;
                }
                _ => {
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                }
            }

            prepend_newline_and_indent = false;
        }
    }

    fn format_client_type(&mut self, client_type: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = client_type.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::LESS,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );

        self.format_token(
            children,
            SyntaxKind::GREATER,
            false,
            Self::format_token_plaintext,
        );
    }

    /// Format an AST enum definition.
    fn format_enum_def(&mut self, enum_def: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = enum_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_ENUM,
            true,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            false,
            Self::format_token_plaintext,
        );

        self.nest(|f| {
            while f.format_node_stop(
                children,
                SyntaxKind::ENUM_VARIANT,
                true,
                Self::format_enum_variant,
                SyntaxKind::BLOCK_ATTRIBUTE,
            ) {}

            while f.format_node_stop(
                children,
                SyntaxKind::BLOCK_ATTRIBUTE,
                true,
                Self::format_block_attribute,
                SyntaxKind::R_BRACE,
            ) {}

            f.format_missing(enum_def.text_range().end()); // handle hanging trivia at the end of the enum
        });

        self.format_token(
            children,
            SyntaxKind::R_BRACE,
            true,
            Self::format_token_plaintext,
        );
    }

    fn format_block_attribute(
        &mut self,
        block_attribute: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = block_attribute.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::AT_AT,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        // this is the only keyword allowed here
        self.format_token_stop(
            children,
            SyntaxKind::KW_DYNAMIC,
            false,
            Self::format_token_plaintext,
            SyntaxKind::WORD,
        );

        self.format_token_stop(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
            SyntaxKind::ATTRIBUTE_ARGS,
        );

        self.format_node(
            children,
            SyntaxKind::ATTRIBUTE_ARGS,
            false,
            Self::format_attribute_args,
        );
    }

    fn format_attribute_args(
        &mut self,
        attribute_args: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = attribute_args.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_PAREN,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::R_PAREN => self.format_token_plaintext(
                    child.into_token().unwrap(),
                    prepend_newline_and_indent,
                ),
                SyntaxKind::COMMA => {
                    self.push_format(child.text_range(), ", ", prepend_newline_and_indent)
                }
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, prepend_newline_and_indent)
                }
                _ => (),
            }
        }
    }

    fn format_enum_variant(&mut self, enum_variant: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = enum_variant.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        self.format_node(
            children,
            SyntaxKind::ATTRIBUTE,
            true,
            Self::format_attribute,
        );
    }

    /// Format an AST class definition.
    fn format_class_def(&mut self, class_def: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = class_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_CLASS,
            true,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            false,
            Self::format_token_plaintext,
        );

        self.nest(|f| {
            while f.format_node_stop(
                children,
                SyntaxKind::FIELD,
                true,
                Self::format_class_field,
                SyntaxKind::BLOCK_ATTRIBUTE,
            ) {}

            while f.format_node_stop(
                children,
                SyntaxKind::BLOCK_ATTRIBUTE,
                true,
                Self::format_block_attribute,
                SyntaxKind::R_BRACE,
            ) {}

            f.format_missing(class_def.text_range().end()); // handle hanging trivia at the end of the class
        });

        self.format_token(
            children,
            SyntaxKind::R_BRACE,
            true,
            Self::format_token_plaintext,
        );
    }

    fn format_class_field(&mut self, class_field: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = class_field.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::TYPE_EXPR,
            false,
            Self::format_type_expr,
        );

        self.format_node(
            children,
            SyntaxKind::ATTRIBUTE,
            true,
            Self::format_attribute,
        );
    }

    fn format_attribute(&mut self, attribute: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = attribute.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::AT,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );

        self.format_node(
            children,
            SyntaxKind::ATTRIBUTE_ARGS,
            false,
            Self::format_attribute_args,
        );
    }

    fn format_function_def(&mut self, function_def: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = function_def.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_FUNCTION,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format the function name
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_node(
            children,
            SyntaxKind::PARAMETER_LIST,
            false,
            Self::format_parameter_list,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::ARROW,
            false,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format the return type
        self.format_node(
            children,
            SyntaxKind::TYPE_EXPR,
            false,
            Self::format_type_expr,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::EXPR_FUNCTION_BODY => {
                    self.format_expr_function_body(child.into_node().unwrap(), false)
                }
                SyntaxKind::LLM_FUNCTION_BODY => {
                    self.format_llm_function_body(child.into_node().unwrap(), false)
                }
                _ => (),
            }
        }
    }

    fn format_llm_function_body(
        &mut self,
        llm_function_body: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = llm_function_body.children_with_tokens().peekable();

        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        let mut brace_child = None;
        self.nest(|f| {
            while let Some(child) = children.next() {
                match child.kind() {
                    SyntaxKind::CLIENT_FIELD => {
                        f.format_client_field(child.into_node().unwrap(), true)
                    }
                    SyntaxKind::PROMPT_FIELD => {
                        f.format_prompt_field(child.into_node().unwrap(), true)
                    }
                    SyntaxKind::R_BRACE => {
                        brace_child = Some(child); // hack to get the closing brace child outside of the nest block
                        break;
                    }
                    _ => (),
                }
            }

            f.format_missing(llm_function_body.text_range().end()); // handle hanging trivia at the end of the llm function body
        });

        if let Some(brace_child) = brace_child {
            self.format_token_plaintext(brace_child.into_token().unwrap(), true);
        }
    }

    fn format_client_field(&mut self, client_field: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = client_field.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_CLIENT,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
    }

    fn format_prompt_field(&mut self, prompt_field: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = prompt_field.children_with_tokens().peekable();

        // format prompt keyword, which for some reason is not actually a keyword
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                    self.format_string_literal(child.into_node().unwrap(), false)
                }
                _ => (),
            }
        }
    }

    fn format_expr_function_body(
        &mut self,
        expr_function_body: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = expr_function_body.children_with_tokens().peekable();
        self.format_node(
            children,
            SyntaxKind::BLOCK_EXPR,
            prepend_newline_and_indent,
            Self::format_block_expr,
        );
    }

    fn format_block_expr(&mut self, block_expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = block_expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        let mut brace_child = None;
        self.nest(|f| {
            while let Some(child) = children.next() {
                match child.kind() {
                    SyntaxKind::R_BRACE => {
                        brace_child = Some(child); // hack to get the closing brace child outside of the nest block
                        break;
                    }
                    kind if kind.is_valid_rhs_expr() => f.format_rhs_expr(child, true),
                    SyntaxKind::LET_STMT => f.format_let_stmt(child.into_node().unwrap(), true),
                    SyntaxKind::WHILE_STMT => f.format_while_stmt(child.into_node().unwrap(), true),
                    SyntaxKind::RETURN_STMT => {
                        f.format_return_stmt(child.into_node().unwrap(), true)
                    }
                    SyntaxKind::BREAK_STMT => f.format_break_stmt(child.into_node().unwrap(), true),
                    SyntaxKind::CONTINUE_STMT => {
                        f.format_continue_stmt(child.into_node().unwrap(), true)
                    }
                    SyntaxKind::FOR_EXPR => f.format_for_expr(child.into_node().unwrap(), true),
                    _ => (),
                }
            }

            f.format_missing(block_expr.text_range().end()); // handle hanging trivia at the end of the block expression
        });

        if let Some(brace_child) = brace_child {
            self.format_token_plaintext(brace_child.into_token().unwrap(), true);
        }
    }

    fn format_for_expr(&mut self, for_expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = for_expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_FOR,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format the initializer, condition, and update
        // right now this basically just dumps the whole thing without whitespace
        // and doesn't attempt to understand semantics, so it has some whitespace issues
        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::L_PAREN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                }
                SyntaxKind::R_PAREN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                }
                SyntaxKind::KW_IN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                SyntaxKind::LET_STMT => {
                    self.format_let_stmt(child.into_node().unwrap(), false);
                }
                SyntaxKind::SEMICOLON => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, false);
                    self.push_text(" ");
                }
                _ => (),
            }
        }
    }

    fn format_break_stmt(&mut self, break_stmt: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = break_stmt.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_BREAK,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
    }

    fn format_continue_stmt(
        &mut self,
        continue_stmt: SyntaxNode,
        prepend_newline_and_indent: bool,
    ) {
        let ref mut children = continue_stmt.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_CONTINUE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
    }

    /// Formats a value expression, (or a raw literal)
    fn format_rhs_expr(
        &mut self,
        expr: NodeOrToken<SyntaxNode, SyntaxToken>,
        prepend_newline_and_indent: bool,
    ) {
        match expr.kind() {
            SyntaxKind::BINARY_EXPR => {
                self.format_binary_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::IF_EXPR => {
                self.format_if_expr(expr.into_node().unwrap(), prepend_newline_and_indent);
            }
            SyntaxKind::PAREN_EXPR => {
                self.format_paren_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::UNARY_EXPR => {
                self.format_unary_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            // // words are added as-is, usually an identifier or bool literal
            SyntaxKind::WORD => {
                self.format_token_plaintext(expr.into_token().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::BLOCK_EXPR => {
                self.format_block_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::CALL_EXPR => {
                self.format_call_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::PATH_EXPR => {
                self.format_path_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::FIELD_ACCESS_EXPR => {
                self.format_field_access_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::ARRAY_LITERAL => {
                self.format_array_literal(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::MAP_LITERAL => {
                self.format_map_literal(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::OBJECT_LITERAL => {
                self.format_object_literal(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::INDEX_EXPR => {
                self.format_index_expr(expr.into_node().unwrap(), prepend_newline_and_indent)
            }
            SyntaxKind::INTEGER_LITERAL
            | SyntaxKind::FLOAT_LITERAL
            | SyntaxKind::STRING_LITERAL
            | SyntaxKind::RAW_STRING_LITERAL => {
                self.format_literal(expr, prepend_newline_and_indent)
            }
            SyntaxKind::EXPR => todo!(),
            _ => unreachable!(),
        }
    }

    fn format_index_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, prepend_newline_and_indent)
                }
                SyntaxKind::L_BRACKET => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    break;
                }
                _ => (),
            }
        }

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_BRACKET => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                _ => (),
            }
        }
    }

    fn format_map_literal(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_BRACE => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                // why does this not have its own kind?
                SyntaxKind::OBJECT_FIELD => {
                    self.format_object_field(child.into_node().unwrap(), false);
                }
                _ => (),
            }
        }
    }

    fn format_object_literal(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_BRACE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_BRACE => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                // why does this not have its own kind?
                SyntaxKind::OBJECT_FIELD => {
                    self.format_object_field(child.into_node().unwrap(), false);
                }
                _ => (),
            }
        }
    }

    fn format_object_field(&mut self, object_field: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = object_field.children_with_tokens().peekable();

        let mut seen_key = false;
        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL if !seen_key => {
                    self.format_string_literal(
                        child.into_node().unwrap(),
                        prepend_newline_and_indent,
                    );
                    seen_key = true;
                }
                SyntaxKind::WORD => {
                    self.format_token_plaintext(
                        child.into_token().unwrap(),
                        prepend_newline_and_indent,
                    );
                    seen_key = true;
                }
                SyntaxKind::COLON => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                _ => (),
            }
        }
    }

    fn format_array_literal(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_BRACKET,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_BRACKET => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                _ => (),
            }
        }
    }

    fn format_field_access_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();

        // format the base expression and dot
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, prepend_newline_and_indent)
                }
                SyntaxKind::DOT => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    break;
                }
                _ => (),
            }
        }

        // format the field name
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
    }

    fn format_call_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_node(
            children,
            SyntaxKind::PATH_EXPR,
            prepend_newline_and_indent,
            Self::format_path_expr,
        );
        self.format_node(
            children,
            SyntaxKind::CALL_ARGS,
            false,
            Self::format_call_args,
        );
    }

    fn format_path_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::WORD,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.format_token(
            children,
            SyntaxKind::DOT,
            false,
            Self::format_token_plaintext,
        );
        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );
    }

    fn format_call_args(&mut self, args: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = args.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_PAREN,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_PAREN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                SyntaxKind::COMMA => {
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                _ => (),
            }
        }
    }

    fn format_unary_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_operator() => self.format_token_plaintext(
                    child.into_token().unwrap(),
                    prepend_newline_and_indent,
                ),
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                _ => (),
            }
        }
    }

    fn format_paren_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::L_PAREN,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                SyntaxKind::R_PAREN => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                _ => (),
            }
        }
    }

    fn format_if_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_IF,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format the condition
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, false);
                    break;
                }
                _ => (),
            }
        }
        self.push_text(" ");

        // format the then branch
        self.format_node(
            children,
            SyntaxKind::BLOCK_EXPR,
            false,
            Self::format_block_expr,
        );

        while let Some(child) = children.next() {
            match child.kind() {
                SyntaxKind::KW_ELSE => {
                    self.push_text(" ");
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                    while let Some(child) = children.next() {
                        match child.kind() {
                            SyntaxKind::BLOCK_EXPR => {
                                self.format_block_expr(child.into_node().unwrap(), false);
                            }
                            SyntaxKind::IF_EXPR => {
                                self.format_if_expr(child.into_node().unwrap(), false);
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn format_binary_expr(&mut self, expr: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = expr.children_with_tokens().peekable();

        let mut seen_expr = !prepend_newline_and_indent; // track if we've seen an expression yet for newline purposes
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_operator() => {
                    self.push_text(" ");
                    self.format_token_plaintext(child.into_token().unwrap(), false);
                    self.push_text(" ");
                }
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, !seen_expr);
                    seen_expr = true;
                }
                _ => (),
            }
        }
    }

    fn format_string_literal(&mut self, literal: SyntaxNode, prepend_newline_and_indent: bool) {
        let mut within_quotes = false;
        for child in literal.children_with_tokens() {
            let range = child.text_range();
            match child.kind() {
                SyntaxKind::QUOTE => {
                    self.push_format(range, "\"", !within_quotes && prepend_newline_and_indent);
                    within_quotes = true;
                }
                SyntaxKind::HASH => {
                    self.push_format(range, "#", !within_quotes && prepend_newline_and_indent);
                    within_quotes = true;
                }
                _ if within_quotes => {
                    self.format_token_plaintext(child.into_token().unwrap(), false)
                }
                _ => (),
            }
        }
    }

    fn format_literal(
        &mut self,
        literal: NodeOrToken<SyntaxNode, SyntaxToken>,
        prepend_newline_and_indent: bool,
    ) {
        match literal.kind() {
            SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                self.format_string_literal(
                    literal.into_node().unwrap(),
                    prepend_newline_and_indent,
                );
            }
            SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => {
                self.format_token_plaintext(
                    literal.into_token().unwrap(),
                    prepend_newline_and_indent,
                );
            }
            _ => unreachable!(),
        }
    }

    fn format_while_stmt(&mut self, stmt: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = stmt.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_WHILE,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        // format the condition
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => {
                    self.format_rhs_expr(child, false);
                    break;
                }
                _ => (),
            }
        }
        self.push_text(" ");

        // format the body
        self.format_node(
            children,
            SyntaxKind::BLOCK_EXPR,
            false,
            Self::format_block_expr,
        );
    }

    fn format_return_stmt(&mut self, stmt: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = stmt.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_RETURN,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                _ => (),
            }
        }
    }

    fn format_let_stmt(&mut self, stmt: SyntaxNode, prepend_newline_and_indent: bool) {
        let ref mut children = stmt.children_with_tokens().peekable();
        self.format_token(
            children,
            SyntaxKind::KW_LET,
            prepend_newline_and_indent,
            Self::format_token_plaintext,
        );
        self.push_text(" ");

        self.format_token(
            children,
            SyntaxKind::WORD,
            false,
            Self::format_token_plaintext,
        );

        if self.format_token_stop(
            children,
            SyntaxKind::COLON,
            false,
            Self::format_token_plaintext,
            SyntaxKind::EQUALS,
        ) {
            self.push_text(" ");
            self.format_node(
                children,
                SyntaxKind::TYPE_EXPR,
                false,
                Self::format_type_expr,
            );
        }

        self.push_text(" ");
        self.format_token(
            children,
            SyntaxKind::EQUALS,
            false,
            Self::format_token_plaintext,
        );

        self.push_text(" ");
        while let Some(child) = children.next() {
            match child.kind() {
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                _ => (),
            }
        }
    }
}
