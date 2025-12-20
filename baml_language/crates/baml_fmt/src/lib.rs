use baml_base::SourceFile;
use baml_lexer::lex_file;
use baml_parser::parse_file;
use baml_syntax::{
    SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{
        BlockElement as AstBlockElement, BlockExpr as AstBlockExpr, ClassDef as AstClassDef,
        EnumDef as AstEnumDef, FunctionDef as AstFunctionDef, IfExpr as AstIfExpr, Item as AstItem,
        LetStmt as AstLetStmt, LlmFunctionBody as AstLlmFunctionBody,
        ParameterList as AstParameterList, ReturnStmt as AstReturnStmt,
        SourceFile as AstSourceFile, TypeExpr as AstTypeExpr, WhileStmt as AstWhileStmt,
    },
};
use rowan::{NodeOrToken, TextRange, TextSize, ast::AstNode};

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
    fn push_format(&mut self, range: TextRange, text: String, indent: bool) {
        self.format_missing(range.start());
        if indent {
            self.push_text(format!("\n{}", self.gen_indent()));
        }
        self.push_text(text);
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
            let mut already_pushed_semicolon = false;
            while current_pos < start {
                let token = self.root.token_at_offset(current_pos).right_biased();

                let Some(token) = token else {
                    break;
                };

                // check if token is within our target range
                if token.text_range().start() >= start {
                    break;
                }

                // fix trivia if necessary
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
                    // get rid of duplicate semicolons
                    SyntaxKind::SEMICOLON => {
                        if !already_pushed_semicolon {
                            self.push_text(";".to_string());
                            already_pushed_semicolon = true;
                        }
                    }
                    _ => (), // throw away all other tokens
                }

                current_pos = token.text_range().end();
            }
        }

        self.last_pos = start;
    }

    /// Generates a string of the current indent level.
    fn gen_indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    /// Formats a type expression.
    fn format_type_expr(&mut self, type_expr: AstTypeExpr, indent: bool) {
        self.format_type_expr_inner(SyntaxNode::clone(&type_expr.syntax()), indent);
    }

    /// Inner recursive function for formatting a type expression.
    fn format_type_expr_inner(&mut self, node: SyntaxNode, indent: bool) {
        node.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    // allow these tokens to be included in the output
                    SyntaxKind::WORD
                    | SyntaxKind::QUESTION
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::R_BRACKET
                    | SyntaxKind::L_PAREN
                    | SyntaxKind::R_PAREN => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), child.to_string(), i);
                    })),
                    // also allow union pipe, but give it spaces around it
                    SyntaxKind::PIPE => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), " | ".to_string(), i);
                    })),
                    // allow comma for generics/tuples, but give a space after it
                    SyntaxKind::COMMA => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ", ".to_string(), i);
                    })),
                    // allow these for TypeScript-style types
                    SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_string_literal(child.into_node().unwrap(), i);
                        }))
                    }
                    SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_number_literal(child.into_token().unwrap(), i);
                        }))
                    }
                    SyntaxKind::TYPE_ARGS => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_type_args(child.into_node().unwrap(), i);
                    })),
                    // recurse for nesting in parentheses
                    SyntaxKind::TYPE_EXPR => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_type_expr_inner(child.into_node().unwrap(), i);
                    })),
                    // ignore everything else - comments, whitespace, etc.
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);

                false
            });
    }

    /// Formats type arguments.
    fn format_type_args(&mut self, type_args: SyntaxNode, indent: bool) {
        type_args
            .children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::COMMA => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ", ".to_string(), i);
                    })),
                    SyntaxKind::LESS => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "<".to_string(), i);
                    })),
                    SyntaxKind::GREATER => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ">".to_string(), i);
                    })),
                    SyntaxKind::TYPE_EXPR => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_type_expr_inner(child.into_node().unwrap(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    /// Formats a parameter list, including the parentheses. e.g. "(x: int, y: string)"
    fn format_parameter_list(&mut self, parameter_list: AstParameterList, indent: bool) {
        parameter_list
            .syntax()
            .children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::L_PAREN => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "(".to_string(), i);
                    })),
                    SyntaxKind::R_PAREN => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ")".to_string(), i);
                    })),
                    SyntaxKind::PARAMETER => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_parameter(child.into_node().unwrap(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    // Formats a parameter. eg. "x: int"
    fn format_parameter(&mut self, parameter: SyntaxNode, indent: bool) {
        parameter
            .children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::WORD => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(
                            child.text_range(),
                            child.into_token().unwrap().text().to_string(),
                            i,
                        );
                    })),
                    SyntaxKind::COLON => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ": ".to_string(), i);
                    })),
                    SyntaxKind::TYPE_EXPR => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_type_expr_inner(child.into_node().unwrap(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
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
        self.push_format(
            keyword.text_range(),
            format!("enum {} {{", enum_def.name().unwrap().text()),
            true,
        );

        self.nest(|f| {
            for variant in enum_def.variants() {
                let variant_name = variant.name().unwrap();
                f.push_format(
                    variant_name.text_range(),
                    variant_name.text().to_string(),
                    true,
                );

                for attribute in variant.attributes() {
                    let at = attribute.at_tok().unwrap();

                    // TODO: handle attributes with arguments, this will require updating the AST
                    f.push_format(
                        at.text_range(),
                        format!(" @{}", attribute.name().unwrap().text()),
                        false,
                    );
                }
            }

            for attribute in enum_def.block_attributes() {
                let at_at = attribute.at_at_tok().unwrap();

                // TODO: handle block attributes with arguments, this will require updating the AST
                f.push_format(
                    at_at.text_range(),
                    format!("@@{}", attribute.name().unwrap().text()),
                    true,
                );
            }
        });

        let r_brace = enum_def.r_brace_tok().unwrap();
        self.push_format(r_brace.text_range(), "}".to_string(), true);
    }

    /// Format an AST class definition.
    fn format_class_def(&mut self, class_def: AstClassDef) {
        let keyword = class_def.keyword_tok().unwrap();
        self.push_format(
            keyword.text_range(),
            format!("class {} {{", class_def.name().unwrap().text()),
            true,
        );

        self.nest(|f| {
            for field in class_def.fields() {
                let name = field.name().unwrap();

                f.push_format(name.text_range(), format!("{}: ", name.text()), true);
                f.format_type_expr(field.ty().unwrap(), false);

                for attribute in field.attributes() {
                    let at = attribute.at_tok().unwrap();

                    // TODO: handle attributes with arguments, this will require updating the AST
                    f.push_format(
                        at.text_range(),
                        format!(" @{}", attribute.name().unwrap().text()),
                        false,
                    );
                }
            }

            for attribute in class_def.block_attributes() {
                let at_at = attribute.at_at_tok().unwrap();

                // TODO: handle block attributes with arguments, this will require updating the AST
                f.push_format(
                    at_at.text_range(),
                    format!("@@{}", attribute.name().unwrap().text()),
                    true,
                );
            }
        });

        let r_brace = class_def.r_brace_tok().unwrap();
        self.push_format(r_brace.text_range(), "}".to_string(), true);
    }

    fn format_function_def(&mut self, function_def: AstFunctionDef) {
        let keyword = function_def.keyword_tok().unwrap();
        self.push_format(keyword.text_range(), "function".to_string(), true);

        let name = function_def.name().unwrap();
        self.push_format(name.text_range(), format!(" {}", name.text()), false);

        let parameters = function_def.param_list().unwrap();
        self.format_parameter_list(parameters, false);

        let arrow = function_def.arrow_tok().unwrap();
        self.push_format(arrow.text_range(), " -> ".to_string(), false);

        let return_type = function_def.return_type().unwrap();
        self.format_type_expr(return_type, false);

        match (function_def.expr_body(), function_def.llm_body()) {
            (Some(expr_body), None) => {
                self.format_block_expr(expr_body.block_expr().unwrap(), false);
            }
            (None, Some(llm_body)) => self.format_llm_function_body(llm_body),
            (Some(_), Some(_)) => unreachable!(),
            (None, None) => todo!(), // TODO: is this even possible?
        }
    }

    fn format_block_expr(&mut self, block_expr: AstBlockExpr, indent: bool) {
        let l_brace = block_expr.l_brace_tok().unwrap();
        self.push_format(l_brace.text_range(), " {".to_string(), indent);

        self.nest(|f| {
            for element in block_expr.elements() {
                match element {
                    AstBlockElement::Stmt(stmt) => f.format_stmt(stmt),
                    AstBlockElement::ExprNode(expr) => {
                        f.format_rhs_expr(expr.into(), true);
                    }
                    AstBlockElement::ExprToken(expr_token) => {
                        f.format_rhs_expr(expr_token.into(), true);
                    }
                }
            }
        });

        let r_brace = block_expr.r_brace_tok().unwrap();
        self.push_format(r_brace.text_range(), "}".to_string(), true);
    }

    /// Formats an expression, (or a raw literal)
    fn format_rhs_expr(&mut self, expr: NodeOrToken<SyntaxNode, SyntaxToken>, indent: bool) {
        match expr.kind() {
            SyntaxKind::BINARY_EXPR => self.format_binary_expr(expr.into_node().unwrap(), indent),
            SyntaxKind::IF_EXPR => {
                self.format_if_expr(AstIfExpr::cast(expr.into_node().unwrap()).unwrap(), indent);
            }
            SyntaxKind::PAREN_EXPR => self.format_paren_expr(expr.into_node().unwrap(), indent),
            SyntaxKind::UNARY_EXPR => self.format_unary_expr(expr.into_node().unwrap(), indent),
            // words are added as-is, usually an identifier or bool literal
            SyntaxKind::WORD => self.push_format(
                expr.text_range(),
                expr.into_token().unwrap().text().to_string(),
                indent,
            ),
            SyntaxKind::BLOCK_EXPR => self.format_block_expr(
                AstBlockExpr::cast(expr.into_node().unwrap()).unwrap(),
                indent,
            ),
            SyntaxKind::CALL_EXPR => self.format_call_expr(expr.into_node().unwrap(), indent),
            SyntaxKind::PATH_EXPR => self.format_path_expr(expr.into_node().unwrap(), indent),
            SyntaxKind::FIELD_ACCESS_EXPR => {
                self.format_field_access_expr(expr.into_node().unwrap(), indent)
            }
            SyntaxKind::ARRAY_LITERAL => {
                self.format_array_literal(expr.into_node().unwrap(), indent)
            }
            SyntaxKind::MAP_LITERAL => self.format_map_literal(expr.into_node().unwrap(), indent),
            SyntaxKind::OBJECT_LITERAL => {
                self.format_object_literal(expr.into_node().unwrap(), indent)
            }
            SyntaxKind::INDEX_EXPR => self.format_index_expr(expr.into_node().unwrap(), indent),
            SyntaxKind::INTEGER_LITERAL
            | SyntaxKind::FLOAT_LITERAL
            | SyntaxKind::STRING_LITERAL
            | SyntaxKind::RAW_STRING_LITERAL => self.format_literal(expr, indent),
            SyntaxKind::EXPR => todo!(),
            _ => unreachable!(),
        }
    }

    fn format_index_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    kind if kind.is_valid_rhs_expr() => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_rhs_expr(child, i);
                        }))
                    }
                    SyntaxKind::L_BRACKET => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "[".to_string(), i);
                    })),
                    SyntaxKind::R_BRACKET => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "]".to_string(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_map_literal(&mut self, expr: SyntaxNode, indent: bool) {
        // these are currently broken, will get to this later
        todo!()
    }

    fn format_object_literal(&mut self, expr: SyntaxNode, indent: bool) {
        todo!()
    }

    fn format_array_literal(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::L_BRACKET => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "[".to_string(), i);
                    })),
                    SyntaxKind::R_BRACKET => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "]".to_string(), i);
                    })),
                    kind if kind.is_valid_rhs_expr() => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_rhs_expr(child, i);
                        }))
                    }
                    SyntaxKind::COMMA => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ", ".to_string(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_field_access_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    kind if kind.is_valid_rhs_expr() => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_rhs_expr(child, i);
                        }))
                    }
                    SyntaxKind::DOT => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ".".to_string(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_call_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::PATH_EXPR => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_path_expr(child.into_node().unwrap(), i);
                    })),
                    SyntaxKind::CALL_ARGS => Some(Box::new(move |f: &mut Formatter, i| {
                        f.format_call_args(child.into_node().unwrap(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_path_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::WORD => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(
                            child.text_range(),
                            child.into_token().unwrap().text().to_string(),
                            i,
                        );
                    })),
                    SyntaxKind::DOT => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ".".to_string(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_call_args(&mut self, args: SyntaxNode, indent: bool) {
        args.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                match child.kind() {
                    SyntaxKind::L_PAREN => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), "(".to_string(), i);
                    })),
                    SyntaxKind::R_PAREN => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ")".to_string(), i);
                    })),
                    kind if kind.is_valid_rhs_expr() => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_rhs_expr(child, i);
                        }))
                    }
                    SyntaxKind::COMMA => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(child.text_range(), ", ".to_string(), i);
                    })),
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);
                false
            });
    }

    fn format_unary_expr(&mut self, expr: SyntaxNode, indent: bool) {
        for child in expr.children_with_tokens() {
            let range = child.text_range();
            match child.kind() {
                kind if kind.is_operator() => self.push_format(
                    range,
                    child.into_token().unwrap().text().to_string(),
                    indent,
                ),
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, indent),
                _ => (),
            }
        }
    }

    fn format_paren_expr(&mut self, expr: SyntaxNode, indent: bool) {
        for child in expr.children_with_tokens() {
            let range = child.text_range();
            match child.kind() {
                // parentheses added as-is, possible indent for left parenthesis
                SyntaxKind::L_PAREN => self.push_format(range, "(".to_string(), indent),
                SyntaxKind::R_PAREN => self.push_format(range, ")".to_string(), false),
                // expressions and literals added
                kind if kind.is_valid_rhs_expr() => self.format_rhs_expr(child, false),
                // allow literals as well
                kind if kind.is_literal() => self.format_literal(child, false),
                _ => (), // ignore everything else
            }
        }
    }

    fn format_if_expr(&mut self, expr: AstIfExpr, indent: bool) {
        let keyword = expr.keyword_tok().unwrap();
        self.push_format(keyword.text_range(), "if ".to_string(), indent);

        let condition = expr.condition().unwrap();
        self.format_rhs_expr(condition.into(), false);

        let then_branch = expr.then_branch().unwrap();
        self.format_block_expr(then_branch, false);

        if let Some(else_branch) = expr.else_branch() {
            let else_keyword = expr.else_keyword_tok().unwrap();
            self.push_format(else_keyword.text_range(), " else".to_string(), false);

            match else_branch.kind() {
                // normal else block
                SyntaxKind::BLOCK_EXPR => {
                    self.format_block_expr(AstBlockExpr::cast(else_branch).unwrap(), false);
                }
                // else if block, recurse
                SyntaxKind::IF_EXPR => {
                    self.format_if_expr(AstIfExpr::cast(else_branch).unwrap(), false);
                }
                _ => unreachable!(),
            }
        }
    }

    fn format_binary_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| -> Option<Box<dyn FnOnce(&mut Formatter, bool)>> {
                let range = child.text_range();
                match child.kind() {
                    // all permitted operators get added with a space around them
                    kind if kind.is_operator() => Some(Box::new(move |f: &mut Formatter, i| {
                        f.push_format(
                            range,
                            format!(" {} ", child.into_token().unwrap().text()),
                            i,
                        );
                    })),
                    kind if kind.is_valid_rhs_expr() => {
                        Some(Box::new(move |f: &mut Formatter, i| {
                            f.format_rhs_expr(child, i);
                        }))
                    }
                    _ => None,
                }
            })
            .fold(indent, |indent, f| {
                f(self, indent);

                false
            });
    }

    fn format_number_literal(&mut self, literal: SyntaxToken, indent: bool) {
        self.push_format(literal.text_range(), literal.text().to_string(), indent);
    }

    fn format_string_literal(&mut self, literal: SyntaxNode, indent: bool) {
        let mut within_quotes = false;
        for child in literal.children_with_tokens() {
            let range = child.text_range();
            match child.kind() {
                SyntaxKind::QUOTE => {
                    self.push_format(range, "\"".to_string(), !within_quotes && indent);
                    within_quotes = true;
                }
                SyntaxKind::HASH => {
                    self.push_format(range, "#".to_string(), !within_quotes && indent);
                    within_quotes = true;
                }
                SyntaxKind::WHITESPACE => {
                    if within_quotes {
                        self.push_format(range, " ".to_string(), false);
                    }
                }
                SyntaxKind::NEWLINE => {
                    if within_quotes {
                        self.push_format(range, "\n".to_string(), false); // TODO: do we want to accept this for normal strings?
                    }
                }
                SyntaxKind::WORD => {
                    self.push_format(range, child.into_token().unwrap().text().to_string(), false);
                }
                _ => (),
            }
        }
    }

    fn format_literal(&mut self, literal: NodeOrToken<SyntaxNode, SyntaxToken>, indent: bool) {
        match literal.kind() {
            SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                self.format_string_literal(literal.into_node().unwrap(), indent);
            }
            SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => {
                self.format_number_literal(literal.into_token().unwrap(), indent);
            }
            _ => unreachable!(),
        }
    }

    fn format_stmt(&mut self, stmt: SyntaxNode) {
        match stmt.kind() {
            SyntaxKind::LET_STMT => self.format_let_stmt(AstLetStmt::cast(stmt).unwrap(), true),
            SyntaxKind::RETURN_STMT => {
                self.format_return_stmt(AstReturnStmt::cast(stmt).unwrap(), true)
            }
            SyntaxKind::WHILE_STMT => {
                self.format_while_stmt(AstWhileStmt::cast(stmt).unwrap(), true)
            }
            SyntaxKind::FOR_EXPR | SyntaxKind::BREAK_STMT | SyntaxKind::CONTINUE_STMT => (),
            _ => unreachable!(),
        }
    }

    fn format_while_stmt(&mut self, stmt: AstWhileStmt, indent: bool) {
        let keyword = stmt.keyword_tok().unwrap();
        self.push_format(keyword.text_range(), "while ".to_string(), indent);

        let condition = stmt.condition().unwrap();
        self.format_rhs_expr(condition.into(), false);

        let body = stmt.body().unwrap();
        self.format_block_expr(body, false);
    }

    fn format_return_stmt(&mut self, stmt: AstReturnStmt, indent: bool) {
        let keyword = stmt.keyword_tok().unwrap();
        self.push_format(keyword.text_range(), "return ".to_string(), indent);

        let value = stmt.value().unwrap();
        self.format_rhs_expr(value.into(), false);
    }

    fn format_let_stmt(&mut self, stmt: AstLetStmt, indent: bool) {
        let keyword = stmt.keyword_tok().unwrap();
        self.push_format(keyword.text_range(), "let ".to_string(), indent);

        let name = stmt.name().unwrap();
        self.push_format(name.text_range(), name.text().to_string(), false);

        if let Some(type_annotation) = stmt.ty() {
            self.push_format(
                type_annotation.syntax().text_range(),
                ": ".to_string(),
                false,
            );
            self.format_type_expr(type_annotation.into(), false);
        }

        let equals = stmt.equals_tok().unwrap();
        self.push_format(equals.text_range(), " = ".to_string(), false);

        let expr = stmt.initializer().unwrap();
        self.format_rhs_expr(expr, false);
    }

    fn format_llm_function_body(&mut self, llm_body: AstLlmFunctionBody) {
        todo!()
    }
}
