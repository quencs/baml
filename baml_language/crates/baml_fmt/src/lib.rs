use baml_base::SourceFile;
use baml_lexer::lex_file;
use baml_parser::parse_file;
use baml_syntax::{
    SyntaxKind, SyntaxNode, ast::ClassDef as AstClassDef, ast::EnumDef as AstEnumDef,
    ast::FunctionDef as AstFunctionDef, ast::Item as AstItem, ast::Parameter as AstParameter,
    ast::ParameterList as AstParameterList, ast::SourceFile as AstSourceFile,
    ast::TypeExpr as AstTypeExpr,
};
use rowan::{TextRange, TextSize, ast::AstNode};

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

    // The same as push_format, but also prepends a newline and an indent.
    fn push_format_indent(&mut self, range: TextRange, text: String) {
        self.push_format(range, format!("\n{}{}", self.gen_indent(), text));
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

    /// Generates a string of the current indent level.
    fn gen_indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    /// Generates a string of the provided type expression.
    fn gen_type_expr(&self, type_expr: AstTypeExpr) -> String {
        self.gen_type_expr_inner(type_expr.syntax())
    }

    /// Inner recursive function for generating a string of the provided type expression.
    fn gen_type_expr_inner(&self, node: &SyntaxNode) -> String {
        node.children_with_tokens()
            .filter_map(|n| match n.kind() {
                // allow these tokens to be included in the output
                SyntaxKind::WORD
                | SyntaxKind::QUESTION
                | SyntaxKind::L_BRACKET
                | SyntaxKind::R_BRACKET
                | SyntaxKind::L_PAREN
                | SyntaxKind::R_PAREN => Some(n.to_string()),
                // also allow union pipe, but give it spaces around it
                SyntaxKind::PIPE => Some(" | ".to_string()),
                // allow comma for generics/tuples, but give a space after it
                SyntaxKind::COMMA => Some(", ".to_string()),
                // allow these for TypeScript-style types
                SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                    Some(n.to_string().trim().to_string())
                }
                SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => Some(n.to_string()),
                SyntaxKind::TYPE_ARGS => {
                    Some(format!("<{}>", self.gen_type_args(n.into_node().unwrap())))
                }
                // recurse for nesting in parentheses
                SyntaxKind::TYPE_EXPR => Some(self.gen_type_expr_inner(&n.into_node().unwrap())),
                // ignore everything else - comments, whitespace, etc.
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn gen_type_args(&self, type_args: SyntaxNode) -> String {
        type_args
            .children_with_tokens()
            .filter_map(|n| match n.kind() {
                SyntaxKind::TYPE_EXPR => Some(self.gen_type_expr_inner(&n.into_node().unwrap())),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn gen_parameter_list(&self, parameter_list: AstParameterList) -> String {
        format!(
            "({})",
            parameter_list
                .params()
                .map(|p| self.gen_parameter(p))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn gen_parameter(&self, parameter: AstParameter) -> String {
        let name = parameter.name().unwrap();
        let ty = self.gen_type_expr(parameter.ty().unwrap());
        format!("{}: {}", name.text(), ty)
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
        for item in file.items() {
            self.format_item(item);
        }

        // grab hanging trivia at the end of the file
        self.format_missing(self.root.text_range().end());

        Some(self.output.clone())
    }

    /// Format an AST item.
    fn format_item(&mut self, item: AstItem) {
        match item {
            AstItem::Enum(enum_def) => self.format_enum_def(enum_def),
            AstItem::Class(class_def) => self.format_class_def(class_def),
            AstItem::Function(function_def) => self.format_function_def(function_def),
            _ => (),
        }
    }

    /// Format an AST enum definition.
    fn format_enum_def(&mut self, enum_def: AstEnumDef) {
        let keyword = enum_def.keyword_tok().unwrap();
        self.push_format_indent(
            keyword.text_range(),
            format!("enum {} {{", enum_def.name().unwrap().text()),
        );

        self.nest(|f| {
            for variant in enum_def.variants() {
                let variant_name = variant.name().unwrap();
                f.push_format_indent(variant_name.text_range(), variant_name.text().to_string());

                for attribute in variant.attributes() {
                    let at = attribute.at_tok().unwrap();

                    // TODO: handle attributes with arguments, this will require updating the AST
                    f.push_format(
                        at.text_range(),
                        format!(" @{}", attribute.name().unwrap().text()),
                    );
                }
            }

            for attribute in enum_def.block_attributes() {
                let at_at = attribute.at_at_tok().unwrap();

                // TODO: handle block attributes with arguments, this will require updating the AST
                f.push_format_indent(
                    at_at.text_range(),
                    format!("@@{}", attribute.name().unwrap().text()),
                );
            }
        });

        let r_brace = enum_def.r_brace_tok().unwrap();
        self.push_format_indent(r_brace.text_range(), "}".to_string());
    }

    /// Format an AST class definition.
    fn format_class_def(&mut self, class_def: AstClassDef) {
        let keyword = class_def.keyword_tok().unwrap();
        self.push_format_indent(
            keyword.text_range(),
            format!("class {} {{", class_def.name().unwrap().text()),
        );

        self.nest(|f| {
            for field in class_def.fields() {
                let name = field.name().unwrap();
                let ty = f.gen_type_expr(field.ty().unwrap());

                f.push_format_indent(name.text_range(), format!("{} {}", name.text(), ty));

                for attribute in field.attributes() {
                    let at = attribute.at_tok().unwrap();

                    // TODO: handle attributes with arguments, this will require updating the AST
                    f.push_format(
                        at.text_range(),
                        format!(" @{}", attribute.name().unwrap().text()),
                    );
                }
            }

            for attribute in class_def.block_attributes() {
                let at_at = attribute.at_at_tok().unwrap();

                // TODO: handle block attributes with arguments, this will require updating the AST
                f.push_format_indent(
                    at_at.text_range(),
                    format!("@@{}", attribute.name().unwrap().text()),
                );
            }
        });

        let r_brace = class_def.r_brace_tok().unwrap();
        self.push_format_indent(r_brace.text_range(), "}".to_string());
    }

    fn format_function_def(&mut self, function_def: AstFunctionDef) {
        let keyword = function_def.keyword_tok().unwrap();

        let parameters = self.gen_parameter_list(function_def.param_list().unwrap());
        let return_type = self.gen_type_expr(function_def.return_type().unwrap());
        self.push_format_indent(
            keyword.text_range(),
            format!(
                "function {} {} -> {} {{",
                function_def.name().unwrap().text(),
                parameters,
                return_type
            ),
        );
    }
}
