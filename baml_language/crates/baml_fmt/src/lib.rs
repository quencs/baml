use baml_base::SourceFile;
use baml_lexer::lex_file;
use baml_parser::parse_file;
use baml_syntax::{
    SyntaxKind, SyntaxNode, ast::BlockElement as AstBlockElement, ast::ClassDef as AstClassDef,
    ast::EnumDef as AstEnumDef, ast::ExprFunctionBody as AstExprFunctionBody,
    ast::FunctionDef as AstFunctionDef, ast::IfExpr as AstIfExpr, ast::Item as AstItem,
    ast::LetStmt as AstLetStmt, ast::LlmFunctionBody as AstLlmFunctionBody,
    ast::Parameter as AstParameter, ast::ParameterList as AstParameterList,
    ast::SourceFile as AstSourceFile, ast::TypeExpr as AstTypeExpr,
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
        self.gen_type_expr_inner(SyntaxNode::clone(&type_expr.syntax()))
    }

    /// Inner recursive function for generating a string of the provided type expression.
    fn gen_type_expr_inner(&self, node: SyntaxNode) -> String {
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
                SyntaxKind::TYPE_EXPR => Some(self.gen_type_expr_inner(n.into_node().unwrap())),
                // ignore everything else - comments, whitespace, etc.
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Generates a string of the provided type arguments, not including any bracketing
    fn gen_type_args(&self, type_args: SyntaxNode) -> String {
        type_args
            .children_with_tokens()
            .filter_map(|n| match n.kind() {
                SyntaxKind::TYPE_EXPR => Some(self.gen_type_expr_inner(n.into_node().unwrap())),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Generates a string of the provided parameter list, including the parentheses. e.g. "(x: int, y: string)"
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

    // Generates a string of the provided parameter. eg. "x: int"
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
                let ty = f.gen_type_expr(field.ty().unwrap());

                f.push_format(name.text_range(), format!("{} {}", name.text(), ty), true);

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

        let parameters = self.gen_parameter_list(function_def.param_list().unwrap());
        let return_type = self.gen_type_expr(function_def.return_type().unwrap());
        self.push_format(
            keyword.text_range(),
            format!(
                "function {} {} -> {} {{",
                function_def.name().unwrap().text(),
                parameters,
                return_type
            ),
            true,
        );

        match (function_def.expr_body(), function_def.llm_body()) {
            (Some(expr_body), None) => self.format_expr_function_body(expr_body),
            (None, Some(llm_body)) => self.format_llm_function_body(llm_body),
            (Some(_), Some(_)) => unreachable!(),
            (None, None) => todo!(), // TODO: is this even possible?
        }
    }

    fn format_expr_function_body(&mut self, expr_body: AstExprFunctionBody) {
        self.nest(|f| {
            let block_expr = expr_body.block_expr().unwrap();
            for element in block_expr.elements() {
                match element {
                    AstBlockElement::Stmt(stmt) => f.format_stmt(stmt),
                    AstBlockElement::ExprNode(expr) => {
                        f.format_expr(expr, true);
                    }
                    AstBlockElement::ExprToken(expr_token) => todo!(),
                }
            }
        });
    }

    fn format_expr(&mut self, expr: SyntaxNode, indent: bool) {
        match expr.kind() {
            SyntaxKind::BINARY_EXPR => self.format_binary_expr(expr, indent),
            SyntaxKind::IF_EXPR => {
                self.format_if_expr(AstIfExpr::cast(SyntaxNode::clone(&expr)).unwrap(), indent)
            }
            SyntaxKind::PAREN_EXPR => self.format_paren_expr(expr, indent),
            SyntaxKind::EXPR
            | SyntaxKind::UNARY_EXPR
            | SyntaxKind::CALL_EXPR
            | SyntaxKind::BLOCK_EXPR
            | SyntaxKind::PATH_EXPR
            | SyntaxKind::FIELD_ACCESS_EXPR
            | SyntaxKind::INDEX_EXPR
            | SyntaxKind::ARRAY_LITERAL
            | SyntaxKind::OBJECT_LITERAL => todo!(),
            _ => unreachable!(),
        }
    }

    fn format_paren_expr(&mut self, expr: SyntaxNode, indent: bool) {}

    fn format_if_expr(&mut self, expr: AstIfExpr, indent: bool) {
        let condition = expr.condition().unwrap();
        let then_branch = expr.then_branch().unwrap();
    }

    fn format_binary_expr(&mut self, expr: SyntaxNode, indent: bool) {
        expr.children_with_tokens()
            .filter_map(|child| {
                let range = child.text_range();
                let text = child.clone().into_token().unwrap().text().to_string();
                match child.kind() {
                    // all permitted operators get added with a space around them
                    SyntaxKind::EQUALS
                    | SyntaxKind::PLUS_EQUALS
                    | SyntaxKind::MINUS_EQUALS
                    | SyntaxKind::STAR_EQUALS
                    | SyntaxKind::SLASH_EQUALS
                    | SyntaxKind::PERCENT_EQUALS
                    | SyntaxKind::AND_EQUALS
                    | SyntaxKind::PIPE_EQUALS
                    | SyntaxKind::CARET_EQUALS
                    | SyntaxKind::LESS_LESS_EQUALS
                    | SyntaxKind::GREATER_GREATER_EQUALS
                    | SyntaxKind::OR_OR
                    | SyntaxKind::AND_AND
                    | SyntaxKind::PIPE
                    | SyntaxKind::CARET
                    | SyntaxKind::AND
                    | SyntaxKind::EQUALS_EQUALS
                    | SyntaxKind::NOT_EQUALS
                    | SyntaxKind::LESS
                    | SyntaxKind::GREATER
                    | SyntaxKind::LESS_EQUALS
                    | SyntaxKind::GREATER_EQUALS
                    | SyntaxKind::LESS_LESS
                    | SyntaxKind::GREATER_GREATER
                    | SyntaxKind::PLUS
                    | SyntaxKind::MINUS
                    | SyntaxKind::STAR
                    | SyntaxKind::SLASH
                    | SyntaxKind::PERCENT => Some((range, format!(" {} ", text))),
                    // add literals as-is
                    SyntaxKind::STRING_LITERAL | SyntaxKind::RAW_STRING_LITERAL => {
                        Some((range, text.trim().to_string()))
                    }
                    SyntaxKind::INTEGER_LITERAL | SyntaxKind::FLOAT_LITERAL => {
                        Some((range, text.to_string()))
                    }
                    // add words as-is
                    SyntaxKind::WORD => Some((range, text.to_string())),
                    _ => None,
                }
            })
            .fold(indent, |indent, (range, text)| {
                self.push_format(range, text, indent);

                false
            });
    }

    fn format_stmt(&mut self, stmt: SyntaxNode) {
        match stmt.kind() {
            SyntaxKind::LET_STMT => self.format_let_stmt(AstLetStmt::cast(stmt).unwrap()),
            SyntaxKind::RETURN_STMT
            | SyntaxKind::WHILE_STMT
            | SyntaxKind::FOR_EXPR
            | SyntaxKind::BREAK_STMT
            | SyntaxKind::CONTINUE_STMT => (),
            _ => unreachable!(),
        }
    }

    fn format_let_stmt(&mut self, stmt: AstLetStmt) {
        let name = stmt.name().unwrap();
        let ty = stmt.ty().map(|t| self.gen_type_expr(t));
        // let initializer = stmt.initializer().map(|t| self.gen_expr(t));
    }

    fn format_llm_function_body(&mut self, llm_body: AstLlmFunctionBody) {
        todo!()
    }
}
