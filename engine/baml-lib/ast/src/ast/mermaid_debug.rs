use std::collections::HashMap;

use super::{
    App, Argument, ArgumentsList, Assignment, Ast, Attribute, BlockArgs, ClassConstructor,
    ClassConstructorField, ExprFn, Expression, ExpressionBlock, Field, FieldType, Header,
    Identifier, RawString, Stmt, TemplateString, Top, TopLevelAssignment, TypeExpressionBlock,
    ValueExprBlock, WithIdentifier, WithName, WithSpan,
};

/// A debug utility for converting AST structures to Mermaid diagrams
#[derive(Debug)]
pub struct MermaidDiagramGenerator {
    /// Counter for generating unique node IDs
    node_counter: u32,
    /// Map from node addresses to generated IDs for deduplication
    node_ids: HashMap<String, String>,
    /// The accumulated Mermaid diagram content
    content: Vec<String>,
}

impl MermaidDiagramGenerator {
    pub fn new() -> Self {
        Self {
            node_counter: 0,
            node_ids: HashMap::new(),
            content: vec!["graph TD".to_string()],
        }
    }

    /// Generate a Mermaid diagram for the entire AST
    pub fn generate_ast_diagram(ast: &Ast) -> String {
        let mut generator = Self::new();
        generator.visit_ast(ast);
        generator.content.join("\n")
    }

    /// Get a unique node ID for a given pointer/key
    fn get_node_id(&mut self, key: &str, label: &str) -> String {
        if let Some(id) = self.node_ids.get(key) {
            id.clone()
        } else {
            let id = format!("n{}", self.node_counter);
            self.node_counter += 1;
            self.node_ids.insert(key.to_string(), id.clone());

            // Escape quotes and special characters in label for Mermaid
            let escaped_label = label
                .replace('"', "&quot;")
                .replace('\n', "<br/>")
                .replace('(', "&#40;")
                .replace(')', "&#41;");

            self.content
                .push(format!("    {}[\"{}\"]; ", id, escaped_label));
            id
        }
    }

    /// Connect two nodes in the diagram
    fn connect(&mut self, from: &str, to: &str, label: Option<&str>) {
        if let Some(label) = label {
            let escaped_label = label.replace('"', "&quot;");
            self.content
                .push(format!("    {} -->|\"{}\"| {};", from, escaped_label, to));
        } else {
            self.content.push(format!("    {} --> {};", from, to));
        }
    }

    /// Visit the root AST node
    fn visit_ast(&mut self, ast: &Ast) {
        let ast_key = format!("ast_{:p}", ast);
        let ast_id = self.get_node_id(&ast_key, "AST Root");

        for (idx, top) in ast.tops.iter().enumerate() {
            let top_id = self.visit_top(top, idx);
            self.connect(&ast_id, &top_id, Some("contains"));
        }
    }

    /// Visit a top-level AST node
    fn visit_top(&mut self, top: &Top, index: usize) -> String {
        let top_key = format!("top_{}_{:p}", index, top);

        match top {
            Top::Enum(type_expr) => {
                let label = format!("Enum: {}", type_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let type_expr_id = self.visit_type_expression_block(type_expr);
                self.connect(&top_id, &type_expr_id, None);
                top_id
            }
            Top::Class(type_expr) => {
                let label = format!("Class: {}", type_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let type_expr_id = self.visit_type_expression_block(type_expr);
                self.connect(&top_id, &type_expr_id, None);
                top_id
            }
            Top::Function(value_expr) => {
                let label = format!("Function: {}", value_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let value_expr_id = self.visit_value_expression_block(value_expr);
                self.connect(&top_id, &value_expr_id, None);
                top_id
            }
            Top::TypeAlias(assignment) => {
                let label = format!("TypeAlias: {}", assignment.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let assignment_id = self.visit_assignment(assignment);
                self.connect(&top_id, &assignment_id, None);
                top_id
            }
            Top::Client(value_expr) => {
                let label = format!("Client: {}", value_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let value_expr_id = self.visit_value_expression_block(value_expr);
                self.connect(&top_id, &value_expr_id, None);
                top_id
            }
            Top::TemplateString(template) => {
                let label = format!("TemplateString: {}", template.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let template_id = self.visit_template_string(template);
                self.connect(&top_id, &template_id, None);
                top_id
            }
            Top::Generator(value_expr) => {
                let label = format!("Generator: {}", value_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let value_expr_id = self.visit_value_expression_block(value_expr);
                self.connect(&top_id, &value_expr_id, None);
                top_id
            }
            Top::TestCase(value_expr) => {
                let label = format!("TestCase: {}", value_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let value_expr_id = self.visit_value_expression_block(value_expr);
                self.connect(&top_id, &value_expr_id, None);
                top_id
            }
            Top::RetryPolicy(value_expr) => {
                let label = format!("RetryPolicy: {}", value_expr.identifier().name());
                let top_id = self.get_node_id(&top_key, &label);
                let value_expr_id = self.visit_value_expression_block(value_expr);
                self.connect(&top_id, &value_expr_id, None);
                top_id
            }
            Top::TopLevelAssignment(assignment) => {
                let label = format!("Assignment: {}", assignment.stmt.identifier.name());
                let top_id = self.get_node_id(&top_key, &label);
                let assignment_id = self.visit_top_level_assignment(assignment);
                self.connect(&top_id, &assignment_id, None);
                top_id
            }
            Top::ExprFn(expr_fn) => {
                let label = format!("ExprFn: {}", expr_fn.name.name());
                let top_id = self.get_node_id(&top_key, &label);
                let expr_fn_id = self.visit_expr_fn(expr_fn);
                self.connect(&top_id, &expr_fn_id, None);
                top_id
            }
        }
    }

    /// Visit a type expression block (enum or class)
    fn visit_type_expression_block(&mut self, type_expr: &TypeExpressionBlock) -> String {
        let key = format!("type_expr_{:p}", type_expr);
        let label = format!("TypeExpr<br/>name: {}", type_expr.name.name());
        let type_expr_id = self.get_node_id(&key, &label);

        // Visit input arguments if present
        if let Some(input) = &type_expr.input {
            let input_id = self.visit_block_args(input);
            self.connect(&type_expr_id, &input_id, Some("input"));
        }

        // Visit fields
        for (idx, field) in type_expr.fields.iter().enumerate() {
            let field_id = self.visit_field_type(field, idx);
            self.connect(&type_expr_id, &field_id, Some("field"));
        }

        // Visit attributes
        for (idx, attr) in type_expr.attributes.iter().enumerate() {
            let attr_id = self.visit_attribute(attr, idx);
            self.connect(&type_expr_id, &attr_id, Some("attr"));
        }

        type_expr_id
    }

    /// Visit a value expression block (function, client, etc.)
    fn visit_value_expression_block(&mut self, value_expr: &ValueExprBlock) -> String {
        let key = format!("value_expr_{:p}", value_expr);
        let label = format!(
            "ValueExpr<br/>name: {}<br/>type: {:?}",
            value_expr.name.name(),
            value_expr.block_type
        );
        let value_expr_id = self.get_node_id(&key, &label);

        // Visit input arguments if present
        if let Some(input) = &value_expr.input {
            let input_id = self.visit_block_args(input);
            self.connect(&value_expr_id, &input_id, Some("input"));
        }

        // Visit fields (expressions)
        for (idx, field) in value_expr.fields.iter().enumerate() {
            let field_id = self.visit_field_expression(field, idx);
            self.connect(&value_expr_id, &field_id, Some("field"));
        }

        // Visit attributes
        for (idx, attr) in value_expr.attributes.iter().enumerate() {
            let attr_id = self.visit_attribute(attr, idx);
            self.connect(&value_expr_id, &attr_id, Some("attr"));
        }

        value_expr_id
    }

    /// Visit block arguments
    fn visit_block_args(&mut self, block_args: &BlockArgs) -> String {
        let key = format!("block_args_{:p}", block_args);
        let label = "BlockArgs";
        let block_args_id = self.get_node_id(&key, label);

        for (idx, (identifier, block_arg)) in block_args.args.iter().enumerate() {
            let arg_key = format!("block_arg_{}_{:p}", idx, block_arg);
            let arg_label = format!(
                "Arg: {}<br/>type: {}",
                identifier.name(),
                block_arg.field_type.name()
            );
            let arg_id = self.get_node_id(&arg_key, &arg_label);
            self.connect(&block_args_id, &arg_id, None);
        }

        block_args_id
    }

    /// Visit a field with FieldType
    fn visit_field_type(&mut self, field: &Field<FieldType>, index: usize) -> String {
        let key = format!("field_type_{}_{:p}", index, field);
        let mut label = format!("Field: {}", field.name.name());

        if let Some(field_type) = &field.expr {
            label.push_str(&format!("<br/>type: {}", field_type.name()));
        }

        let field_id = self.get_node_id(&key, &label);

        // Visit attributes
        for (idx, attr) in field.attributes.iter().enumerate() {
            let attr_id = self.visit_attribute(attr, idx);
            self.connect(&field_id, &attr_id, Some("attr"));
        }

        field_id
    }

    /// Visit a field with Expression
    fn visit_field_expression(&mut self, field: &Field<Expression>, index: usize) -> String {
        let key = format!("field_expr_{}_{:p}", index, field);
        let label = format!("Field: {}", field.name.name());
        let field_id = self.get_node_id(&key, &label);

        // Visit the expression if present
        if let Some(expr) = &field.expr {
            let expr_id = self.visit_expression(expr);
            self.connect(&field_id, &expr_id, Some("value"));
        }

        // Visit attributes
        for (idx, attr) in field.attributes.iter().enumerate() {
            let attr_id = self.visit_attribute(attr, idx);
            self.connect(&field_id, &attr_id, Some("attr"));
        }

        field_id
    }

    /// Visit an attribute
    fn visit_attribute(&mut self, attr: &Attribute, index: usize) -> String {
        let key = format!("attr_{}_{:p}", index, attr);
        let label = format!("@{}", attr.name.name());
        let attr_id = self.get_node_id(&key, &label);

        // Visit arguments
        let args_id = self.visit_arguments_list(&attr.arguments);
        if !attr.arguments.arguments.is_empty() {
            self.connect(&attr_id, &args_id, Some("args"));
        }

        attr_id
    }

    /// Visit an arguments list
    fn visit_arguments_list(&mut self, args: &ArgumentsList) -> String {
        let key = format!("args_{:p}", args);
        let label = "Arguments";
        let args_id = self.get_node_id(&key, label);

        for (idx, arg) in args.arguments.iter().enumerate() {
            let arg_id = self.visit_argument(arg, idx);
            self.connect(&args_id, &arg_id, None);
        }

        args_id
    }

    /// Visit an argument
    fn visit_argument(&mut self, arg: &Argument, index: usize) -> String {
        let key = format!("arg_{}_{:p}", index, arg);
        let label = "Argument";
        let arg_id = self.get_node_id(&key, label);

        let expr_id = self.visit_expression(&arg.value);
        self.connect(&arg_id, &expr_id, Some("value"));

        arg_id
    }

    /// Visit an expression
    fn visit_expression(&mut self, expr: &Expression) -> String {
        let key = format!("expr_{:p}", expr);

        let expr_id = match expr {
            Expression::BoolValue(val, _) => {
                let label = format!("Bool: {}", val);
                self.get_node_id(&key, &label)
            }
            Expression::NumericValue(val, _) => {
                let label = format!("Number: {}", val);
                self.get_node_id(&key, &label)
            }
            Expression::StringValue(val, _) => {
                let label = format!("String: \"{}\"", val.chars().take(20).collect::<String>());
                self.get_node_id(&key, &label)
            }
            Expression::Identifier(ident) => {
                let label = format!("Identifier: {}", ident.name());
                self.get_node_id(&key, &label)
            }
            Expression::RawStringValue(raw) => {
                let preview = raw.value().chars().take(30).collect::<String>();
                let label = format!("RawString: \"{}...\"", preview);
                self.get_node_id(&key, &label)
            }
            Expression::ClassConstructor(constructor, _) => {
                let label = "ClassConstructor".to_string();
                let expr_id = self.get_node_id(&key, &label);
                let constructor_id = self.visit_class_constructor(constructor);
                self.connect(&expr_id, &constructor_id, Some("constructor"));
                expr_id
            }
            Expression::Array(exprs, _) => {
                let label = "Array".to_string();
                let expr_id = self.get_node_id(&key, &label);
                for (idx, expr) in exprs.iter().enumerate() {
                    let child_id = self.visit_expression(expr);
                    self.connect(&expr_id, &child_id, Some(&format!("item_{}", idx)));
                }
                expr_id
            }
            Expression::Map(map, _) => {
                let label = "Map".to_string();
                let expr_id = self.get_node_id(&key, &label);
                for (idx, (key_expr, value_expr)) in map.iter().enumerate() {
                    let key_id = self.visit_expression(key_expr);
                    let value_id = self.visit_expression(value_expr);
                    self.connect(&expr_id, &key_id, Some(&format!("key_{}", idx)));
                    self.connect(&expr_id, &value_id, Some(&format!("value_{}", idx)));
                }
                expr_id
            }
            Expression::JinjaExpressionValue(jinja, _) => {
                let label = format!("Jinja: {}", jinja);
                self.get_node_id(&key, &label)
            }
            Expression::Lambda(args, body, _) => {
                let label = "Lambda".to_string();
                let expr_id = self.get_node_id(&key, &label);
                let args_id = self.visit_arguments_list(args);
                let body_id = self.visit_expression_block(body);
                self.connect(&expr_id, &args_id, Some("args"));
                self.connect(&expr_id, &body_id, Some("body"));
                expr_id
            }
            Expression::App(app) => {
                let label = format!("App: {}", app.name.name());
                let expr_id = self.get_node_id(&key, &label);
                for (idx, arg) in app.args.iter().enumerate() {
                    let arg_id = self.visit_expression(arg);
                    self.connect(&expr_id, &arg_id, Some(&format!("arg_{}", idx)));
                }
                expr_id
            }
            Expression::ExprBlock(block, _) => {
                let label = "ExprBlock".to_string();
                let expr_id = self.get_node_id(&key, &label);
                let block_id = self.visit_expression_block(block);
                self.connect(&expr_id, &block_id, Some("block"));
                expr_id
            }
            Expression::If(cond, then_expr, else_expr, _) => {
                let label = "If".to_string();
                let expr_id = self.get_node_id(&key, &label);
                let cond_id = self.visit_expression(cond);
                let then_id = self.visit_expression(then_expr);
                self.connect(&expr_id, &cond_id, Some("condition"));
                self.connect(&expr_id, &then_id, Some("then"));
                if let Some(else_expr) = else_expr {
                    let else_id = self.visit_expression(else_expr);
                    self.connect(&expr_id, &else_id, Some("else"));
                }
                expr_id
            }
            Expression::Not(expr, _) => {
                let label = "Not".to_string();
                let expr_id = self.get_node_id(&key, &label);
                let child_id = self.visit_expression(expr);
                self.connect(&expr_id, &child_id, Some("expr"));
                expr_id
            }
        };

        expr_id
    }

    /// Visit a class constructor
    fn visit_class_constructor(&mut self, constructor: &ClassConstructor) -> String {
        let key = format!("constructor_{:p}", constructor);
        let label = format!("ClassConstructor: {}", constructor.class_name.name());
        let constructor_id = self.get_node_id(&key, &label);

        for (idx, field) in constructor.fields.iter().enumerate() {
            let field_id = self.visit_class_constructor_field(field, idx);
            self.connect(&constructor_id, &field_id, Some("field"));
        }

        constructor_id
    }

    /// Visit a class constructor field
    fn visit_class_constructor_field(
        &mut self,
        field: &ClassConstructorField,
        index: usize,
    ) -> String {
        let key = format!("constructor_field_{}_{:p}", index, field);
        let (label, expr) = match field {
            ClassConstructorField::Named(name, expr) => (format!("Field: {}", name.name()), expr),
            ClassConstructorField::Spread(expr) => ("Spread".to_string(), expr),
        };
        let field_id = self.get_node_id(&key, &label);

        let expr_id = self.visit_expression(expr);
        self.connect(&field_id, &expr_id, Some("value"));

        field_id
    }

    /// Visit an expression block
    fn visit_expression_block(&mut self, block: &ExpressionBlock) -> String {
        let key = format!("expr_block_{:p}", block);
        let label = "ExpressionBlock";
        let block_id = self.get_node_id(&key, label);

        // Visit statements
        for (idx, stmt) in block.stmts.iter().enumerate() {
            let stmt_id = self.visit_stmt(stmt, idx);
            self.connect(&block_id, &stmt_id, Some("stmt"));
        }

        // Visit the final expression
        let expr_id = self.visit_expression(&block.expr);
        self.connect(&block_id, &expr_id, Some("expr"));

        // Visit headers that apply to the final expression
        for (idx, header) in block.expr_headers.iter().enumerate() {
            let header_id = self.visit_header(header, idx);
            self.connect(&expr_id, &header_id, Some("annotation"));
        }

        block_id
    }

    /// Visit a header
    fn visit_header(&mut self, header: &Header, index: usize) -> String {
        let key = format!("header_{}_{:p}", index, header);
        let label = format!("Header L{}: {}", header.level, header.title);
        self.get_node_id(&key, &label)
    }

    /// Visit a statement
    fn visit_stmt(&mut self, stmt: &Stmt, index: usize) -> String {
        let key = format!("stmt_{}_{:p}", index, stmt);

        match stmt {
            Stmt::Let(let_stmt) => {
                let label = format!("Let: {}", let_stmt.identifier.name());
                let stmt_id = self.get_node_id(&key, &label);

                // Visit annotations
                for (idx, annotation) in let_stmt.annotations.iter().enumerate() {
                    let annotation_id = self.visit_header(annotation, idx);
                    self.connect(&stmt_id, &annotation_id, Some("annotation"));
                }

                let expr_id = self.visit_expression(&let_stmt.expr);
                self.connect(&stmt_id, &expr_id, Some("value"));
                stmt_id
            }
            Stmt::ForLoop(for_stmt) => {
                let label = format!("For: {}", for_stmt.identifier.name());
                let stmt_id = self.get_node_id(&key, &label);

                // Visit annotations
                for (idx, annotation) in for_stmt.annotations.iter().enumerate() {
                    let annotation_id = self.visit_header(annotation, idx);
                    self.connect(&stmt_id, &annotation_id, Some("annotation"));
                }

                let iterable_id = self.visit_expression(&for_stmt.iterator);
                let body_id = self.visit_expression_block(&for_stmt.body);
                self.connect(&stmt_id, &iterable_id, Some("iterable"));
                self.connect(&stmt_id, &body_id, Some("body"));
                stmt_id
            }
        }
    }

    /// Visit an assignment
    fn visit_assignment(&mut self, assignment: &Assignment) -> String {
        let key = format!("assignment_{:p}", assignment);
        let label = format!("Assignment: {}", assignment.identifier().name());
        let assignment_id = self.get_node_id(&key, &label);

        let type_id = self.visit_field_type_value(&assignment.value);
        self.connect(&assignment_id, &type_id, Some("value"));

        assignment_id
    }

    /// Visit a field type value
    fn visit_field_type_value(&mut self, field_type: &FieldType) -> String {
        let key = format!("field_type_value_{:p}", field_type);
        let label = format!("Type: {}", field_type.name());
        self.get_node_id(&key, &label)
    }

    /// Visit a top-level assignment
    fn visit_top_level_assignment(&mut self, assignment: &TopLevelAssignment) -> String {
        let key = format!("top_assignment_{:p}", assignment);
        let label = format!("TopLevelAssignment: {}", assignment.stmt.identifier.name());
        let assignment_id = self.get_node_id(&key, &label);

        let expr_id = self.visit_expression(&assignment.stmt.expr);
        self.connect(&assignment_id, &expr_id, Some("value"));

        assignment_id
    }

    /// Visit an expression function
    fn visit_expr_fn(&mut self, expr_fn: &ExprFn) -> String {
        let key = format!("expr_fn_{:p}", expr_fn);
        let label = format!("ExprFn: {}", expr_fn.name.name());
        let expr_fn_id = self.get_node_id(&key, &label);

        // Visit arguments
        let args_id = self.visit_block_args(&expr_fn.args);
        self.connect(&expr_fn_id, &args_id, Some("args"));

        // Visit the body expression block
        let body_id = self.visit_expression_block(&expr_fn.body);
        self.connect(&expr_fn_id, &body_id, Some("body"));

        expr_fn_id
    }

    /// Visit a template string
    fn visit_template_string(&mut self, template: &TemplateString) -> String {
        let key = format!("template_{:p}", template);
        let label = format!("TemplateString: {}", template.identifier().name());
        let template_id = self.get_node_id(&key, &label);

        // Visit input parameters if present
        if let Some(input) = &template.input {
            let input_id = self.visit_block_args(input);
            self.connect(&template_id, &input_id, Some("input"));
        }

        // Visit the template value
        let value_id = self.visit_expression(&template.value);
        self.connect(&template_id, &value_id, Some("value"));

        // Visit attributes
        for (idx, attr) in template.attributes.iter().enumerate() {
            let attr_id = self.visit_attribute(attr, idx);
            self.connect(&template_id, &attr_id, Some("attr"));
        }

        template_id
    }
}

impl Default for MermaidDiagramGenerator {
    fn default() -> Self {
        Self::new()
    }
}
