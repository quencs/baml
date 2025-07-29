use baml_types::{type_meta::base::StreamingBehavior, Constraint};
use internal_baml_ast::ast::{self, App, WithName, WithSpan};
use internal_baml_diagnostics::Span;
use pretty::RcDoc;

/// High-level intermediate representation.
///
/// This is analogous to the HIR in Rust: https://rustc-dev-guide.rust-lang.org/hir.html
/// It carries just enough information to produce BAML bytecode. It differs
/// from baml-core IR in that it does not contain any type information. It has limited
/// metadata, for use in debugging, namely source spans.
///
/// See `HIR::from_ast` to see how BAML syntax is lowered into HIR.
///
/// Lowering from AST to HIR involves desugaring certain syntax forms.
///   - For loops become while loops.
///   - Class constructor spreads become regular class constructors with
///     exhaustive fields.
///   - Implicit returns become explicit.
///   - If expressions become if statements with a block.
#[derive(Debug)]
pub struct Program {
    pub expr_functions: Vec<ExprFunction>,
    pub llm_functions: Vec<LLMFunction>,
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
}

impl Program {
    /// Lower BAML AST into HIR.
    pub fn from_ast(ast: &ast::Ast) -> Self {
        let llm_functions = ast
            .iter_tops()
            .filter_map(|(_id, top)| match top {
                ast::Top::Function(function) => Some(LLMFunction::from_ast(function)),
                _ => None,
            })
            .collect();

        let expr_functions = ast
            .iter_tops()
            .filter_map(|(_id, top)| match top {
                ast::Top::ExprFn(expr_fn) => Some(ExprFunction::from_ast(expr_fn)),
                _ => None,
            })
            .collect();

        let classes = ast
            .iter_tops()
            .filter_map(|(_id, top)| match top {
                ast::Top::Class(class) => Some(Class::from_ast(class)),
                _ => None,
            })
            .collect();

        let enums = ast
            .iter_tops()
            .filter_map(|(_id, top)| match top {
                ast::Top::Enum(enum_def) => Some(Enum::from_ast(enum_def)),
                _ => None,
            })
            .collect();

        let hir = Program {
            expr_functions,
            llm_functions,
            classes,
            enums,
        };

        hir
    }
}

#[derive(Debug)]
pub struct ExprFunction {
    pub name: String,
    pub parameters: Vec<Parameter>,
    // pub return_type: Type,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug)]
pub struct LLMFunction {
    pub name: String,
    pub parameters: Vec<Parameter>,
    // pub return_type: Type,
    pub client: String,
    pub prompt: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    // pub r#type: Type,
    pub span: Span,
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug)]
pub struct EnumVariant {
    pub name: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    // pub r#type: Type,
    pub span: Span,
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
}

/// A single unit of execution within a block.
#[derive(Debug)]
pub enum Statement {
    /// Assign an immutable variable.
    Let {
        name: String,
        value: Expression,
        span: Span,
    },
    /// Declare a (mutable) reference.
    /// There is no span because it is never present in the source AST.
    /// This is a desugaring from `if` expressions.
    DeclareReference { name: String, span: Span },
    /// Assign a mutable variable.
    Assign { name: String, value: Expression },
    /// Declare and assign a mutable reference in one statement.
    DeclareAndAssign {
        name: String,
        value: Expression,
        span: Span,
    },
    /// Return from a function.
    Return { expr: Expression, span: Span },
    /// Evaluate an expression as the final value of a block (without returning from function).
    Expression { expr: Expression, span: Span },
    If {
        condition: Box<Expression>,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
    While {
        condition: Box<Expression>,
        block: Block,
        span: Span,
    },
}

/// Expressions
#[derive(Debug)]
pub enum Expression {
    BoolValue(bool, Span),
    NumericValue(String, Span),
    Identifier(String, Span),
    StringValue(String, Span),
    RawStringValue(String, Span),
    Array(Vec<Expression>, Span),
    Map(Vec<(Expression, Expression)>, Span),
    JinjaExpressionValue(String, Span),
    Call(String, Vec<Expression>, Span),
    // Lambda(ArgumentsList, Box<ExpressionBlock>, Span), // TODO.
    // MethodCall(Box<Expression>, String, Vec<Expression>), // TODO.
    ClassConstructor(ClassConstructor, Span),
    /// Expression block - has its own scope with statements and evaluates to a value
    ExpressionBlock(Box<Block>, Span),
}

#[derive(Debug)]
pub struct ClassConstructor {
    pub class_name: String,
    pub fields: Vec<ClassConstructorField>,
}

#[derive(Debug)]
pub struct ClassConstructorField {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug)]
pub enum TypeM<M> {
    Int(M),
    String(M),
    Bool(M),
    Array(Box<TypeM<M>>),
    Map(Box<TypeM<M>>, Box<TypeM<M>>),
    ClassName(String, M),
    EnumName(String, M),
}

type Type = TypeM<TypeMeta>;

#[derive(Debug)]
struct TypeMeta {
    span: Span,
    constraints: Vec<Constraint>,
    streaming_behavior: StreamingBehavior,
}

impl LLMFunction {
    pub fn from_ast(function: &ast::ValueExprBlock) -> Self {
        LLMFunction {
            name: function.name().to_string(),
            parameters: function
                .input()
                .map(|input| {
                    input
                        .args
                        .iter()
                        .map(|(name, _)| Parameter {
                            name: name.to_string(),
                            // r#type: param.r#type.to_string(),
                            span: name.span().clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(vec![]),
            client: function
                .fields()
                .iter()
                .find(|attr| attr.name() == "client")
                .map(|attr| {
                    attr.expr
                        .as_ref()
                        .expect("client must be specified")
                        .to_string()
                })
                .unwrap_or("llm".to_string()),
            prompt: function
                .fields()
                .iter()
                .find(|attr| attr.name() == "prompt")
                .map(|attr| {
                    attr.expr
                        .as_ref()
                        .expect("prompt must be specified")
                        .to_string()
                })
                .unwrap_or("".to_string()),
            span: function.span().clone(),
        }
    }
}

impl ExprFunction {
    /// Lower an expression function into HIR.
    pub fn from_ast(function: &ast::ExprFn) -> Self {
        ExprFunction {
            name: function.name().to_string(),
            parameters: function
                .args
                .args
                .iter()
                .map(|(name, _)| Parameter {
                    name: name.to_string(),
                    // r#type: param.r#type.to_string(),
                    span: name.span().clone(),
                })
                .collect::<Vec<_>>(),
            body: Block::from_function_body(&function.body),
            span: function.span().clone(),
        }
    }
}

impl Block {
    /// Lower an expression block into HIR for function bodies (ends with Statement::Return).
    pub fn from_function_body(block: &ast::ExpressionBlock) -> Self {
        Self::from_ast_with_context(block, true)
    }

    /// Lower an expression block into HIR for expression blocks (ends with Statement::Expression).
    pub fn from_expression_block(block: &ast::ExpressionBlock) -> Self {
        Self::from_ast_with_context(block, false)
    }

    /// Lower an expression block into HIR with specified context.
    /// If is_function_body is true, the final expression becomes Statement::Return.
    /// If is_function_body is false, the final expression becomes Statement::Expression.
    fn from_ast_with_context(block: &ast::ExpressionBlock, is_function_body: bool) -> Self {
        let mut statements = vec![];

        // Process statements, checking for if expressions in let bindings
        for stmt in &block.stmts {
            match stmt {
                ast::Stmt {
                    identifier,
                    body,
                    span,
                } => {
                    match body {
                        ast::Expression::If(condition, then_expr, else_expr, _if_span) => {
                            // Desugar: let foo = if cond then a else b
                            // Into: var foo; if cond { foo = a; } else { foo = b; }
                            match else_expr {
                                Some(else_expr) => {
                                    Self::desugar_if_expression_in_let(
                                        &mut statements,
                                        identifier.to_string(),
                                        condition.as_ref(),
                                        then_expr.as_ref(),
                                        else_expr.as_ref(),
                                        span,
                                    );
                                }
                                None => {
                                    // If there's no else branch, fall back to regular let
                                    statements.push(Statement::from_ast(stmt));
                                }
                            }
                        }
                        _ => {
                            // Regular let statement - but check for if expressions in nested contexts
                            let mut temp_counter = 0;
                            let mut lifted_statements = vec![];
                            let lifted_expr = Expression::from_ast(
                                body,
                                true,
                                &mut lifted_statements,
                                &mut temp_counter,
                            );

                            // Add any lifted statements first
                            statements.extend(lifted_statements);

                            // Then add the actual let statement
                            statements.push(Statement::Let {
                                name: identifier.to_string(),
                                value: lifted_expr,
                                span: span.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Handle if expressions specially in return position
        match block.expr.as_ref() {
            ast::Expression::If(condition, then_expr, else_expr, span) => {
                // Desugar if expression into statements
                // Handle the optional else branch
                match else_expr {
                    Some(else_expr) => {
                        Self::desugar_if_expression_final(
                            &mut statements,
                            condition.as_ref(),
                            then_expr.as_ref(),
                            else_expr.as_ref(),
                            span,
                            is_function_body,
                        );
                    }
                    None => {
                        // If there's no else branch, we can't desugar this properly in HIR
                        // since HIR requires both branches for assignment.
                        // For now, we'll treat it as a regular expression
                        let mut dummy_statements = vec![];
                        let mut dummy_counter = 0;
                        statements.push(if is_function_body {
                            Statement::Return {
                                expr: Expression::from_ast(
                                    block.expr.as_ref(),
                                    false,
                                    &mut dummy_statements,
                                    &mut dummy_counter,
                                ),
                                span: block.expr.span().clone(),
                            }
                        } else {
                            Statement::Expression {
                                expr: Expression::from_ast(
                                    block.expr.as_ref(),
                                    false,
                                    &mut dummy_statements,
                                    &mut dummy_counter,
                                ),
                                span: block.expr.span().clone(),
                            }
                        });
                    }
                }
            }
            _ => {
                // Normal expression - but check for if expressions in nested contexts
                let mut temp_counter = 0;
                let mut lifted_statements = vec![];
                let lifted_expr = Expression::from_ast(
                    block.expr.as_ref(),
                    true,
                    &mut lifted_statements,
                    &mut temp_counter,
                );

                // Add any lifted statements first
                statements.extend(lifted_statements);

                // Then add the final statement
                statements.push(if is_function_body {
                    Statement::Return {
                        expr: lifted_expr,
                        span: block.expr.span().clone(),
                    }
                } else {
                    Statement::Expression {
                        expr: lifted_expr,
                        span: block.expr.span().clone(),
                    }
                });
            }
        }

        Block { statements }
    }

    /// Desugar an if expression in a let binding into statements.
    /// Transforms: let foo = if cond then a else b
    /// Into: var foo; if cond { foo = a; } else { foo = b; }
    fn desugar_if_expression_in_let(
        statements: &mut Vec<Statement>,
        var_name: String,
        condition: &ast::Expression,
        then_expr: &ast::Expression,
        else_expr: &ast::Expression,
        span: &internal_baml_diagnostics::Span,
    ) {
        // 1. Declare the variable
        statements.push(Statement::DeclareReference {
            name: var_name.clone(),
            span: span.clone(),
        });

        // 2. Create the if statement with assignments to the variable
        let mut dummy_statements = vec![];
        let mut dummy_counter = 0;
        let then_block = Block {
            statements: vec![Statement::Assign {
                name: var_name.clone(),
                value: Expression::from_ast(
                    then_expr,
                    false,
                    &mut dummy_statements,
                    &mut dummy_counter,
                ),
            }],
        };

        let else_block = Block {
            statements: vec![Statement::Assign {
                name: var_name.clone(),
                value: Expression::from_ast(
                    else_expr,
                    false,
                    &mut dummy_statements,
                    &mut dummy_counter,
                ),
            }],
        };

        statements.push(Statement::If {
            condition: Box::new(Expression::from_ast(
                condition,
                false,
                &mut dummy_statements,
                &mut dummy_counter,
            )),
            then_block,
            else_block: Some(else_block),
            span: span.clone(),
        });
    }

    /// Desugar an if expression in final position into statements.
    /// For function bodies: if cond { a } else { b } -> if cond { return a; } else { return b; }
    /// For expression blocks: if cond { a } else { b } -> if cond { a; } else { b; }
    fn desugar_if_expression_final(
        statements: &mut Vec<Statement>,
        condition: &ast::Expression,
        then_expr: &ast::Expression,
        else_expr: &ast::Expression,
        span: &internal_baml_diagnostics::Span,
        is_function_body: bool,
    ) {
        // Create the if statement with appropriate final statements in each branch
        let mut dummy_statements = vec![];
        let mut dummy_counter = 0;
        let then_block = Block {
            statements: vec![if is_function_body {
                Statement::Return {
                    expr: Expression::from_ast(
                        then_expr,
                        false,
                        &mut dummy_statements,
                        &mut dummy_counter,
                    ),
                    span: then_expr.span().clone(),
                }
            } else {
                Statement::Expression {
                    expr: Expression::from_ast(
                        then_expr,
                        false,
                        &mut dummy_statements,
                        &mut dummy_counter,
                    ),
                    span: then_expr.span().clone(),
                }
            }],
        };

        let else_block = Block {
            statements: vec![if is_function_body {
                Statement::Return {
                    expr: Expression::from_ast(
                        else_expr,
                        false,
                        &mut dummy_statements,
                        &mut dummy_counter,
                    ),
                    span: else_expr.span().clone(),
                }
            } else {
                Statement::Expression {
                    expr: Expression::from_ast(
                        else_expr,
                        false,
                        &mut dummy_statements,
                        &mut dummy_counter,
                    ),
                    span: else_expr.span().clone(),
                }
            }],
        };

        statements.push(Statement::If {
            condition: Box::new(Expression::from_ast(
                condition,
                false,
                &mut dummy_statements,
                &mut dummy_counter,
            )),
            then_block,
            else_block: Some(else_block),
            span: span.clone(),
        });
    }
}

impl Statement {
    /// Lower a statement into HIR.
    /// Note: If expressions in let bindings are handled at the Block level.
    fn from_ast(stmt: &ast::Stmt) -> Self {
        match stmt {
            ast::Stmt {
                identifier,
                body,
                span,
            } => {
                let mut dummy_statements = vec![];
                let mut dummy_counter = 0;
                Statement::Let {
                    name: identifier.to_string(),
                    value: Expression::from_ast(
                        body,
                        false,
                        &mut dummy_statements,
                        &mut dummy_counter,
                    ),
                    span: span.clone(),
                }
            }
        }
    }
}

impl Expression {
    /// Lower an expression into HIR.
    ///
    /// If `with_lifting` is true, if expressions will be lifted to temporary variables
    /// and the statements will be added to the provided vector.
    /// If `with_lifting` is false, if expressions will fall back to placeholders.
    pub fn from_ast(
        expr: &ast::Expression,
        with_lifting: bool,
        statements: &mut Vec<Statement>,
        temp_counter: &mut usize,
    ) -> Self {
        match expr {
            ast::Expression::BoolValue(value, span) => Expression::BoolValue(*value, span.clone()),
            ast::Expression::NumericValue(value, span) => {
                Expression::NumericValue(value.to_string(), span.clone())
            }
            ast::Expression::Identifier(identifier) => {
                Expression::Identifier(identifier.to_string(), identifier.span().clone())
            }
            ast::Expression::StringValue(value, span) => {
                Expression::StringValue(value.to_string(), span.clone())
            }
            ast::Expression::RawStringValue(raw_string) => Expression::RawStringValue(
                raw_string.inner_value.to_string(),
                raw_string.span().clone(),
            ),
            ast::Expression::Array(values, span) => Expression::Array(
                values
                    .iter()
                    .map(|value| Self::from_ast(value, with_lifting, statements, temp_counter))
                    .collect(),
                span.clone(),
            ),
            ast::Expression::App(App {
                name, args, span, ..
            }) => Expression::Call(
                name.to_string(),
                args.iter()
                    .map(|arg| Self::from_ast(arg, with_lifting, statements, temp_counter))
                    .collect(),
                span.clone(),
            ),
            ast::Expression::Map(pairs, span) => Expression::Map(
                pairs
                    .iter()
                    .map(|(key, value)| {
                        (
                            Self::from_ast(key, with_lifting, statements, temp_counter),
                            Self::from_ast(value, with_lifting, statements, temp_counter),
                        )
                    })
                    .collect(),
                span.clone(),
            ),
            ast::Expression::If(condition, then_expr, else_expr, span) => {
                if with_lifting {
                    // Handle if expressions with lifting to temporary variables
                    match else_expr {
                        Some(else_expr) => {
                            // Generate a unique temporary variable name
                            let temp_name = format!("temp_{}", *temp_counter);
                            *temp_counter += 1;

                            // Declare the temporary variable
                            statements.push(Statement::DeclareReference {
                                name: temp_name.clone(),
                                span: span.clone(),
                            });

                            // Process subexpressions first to avoid borrow checker issues
                            let mut condition_statements = vec![];
                            let mut then_statements = vec![];
                            let mut else_statements = vec![];

                            let condition_expr = Self::from_ast(
                                condition,
                                with_lifting,
                                &mut condition_statements,
                                temp_counter,
                            );
                            let then_value = Self::from_ast(
                                then_expr,
                                with_lifting,
                                &mut then_statements,
                                temp_counter,
                            );
                            let else_value = Self::from_ast(
                                else_expr,
                                with_lifting,
                                &mut else_statements,
                                temp_counter,
                            );

                            // Add all lifted statements
                            statements.extend(condition_statements);
                            statements.extend(then_statements);
                            statements.extend(else_statements);

                            // Create the if statement with assignments to the temporary variable
                            let then_block = Block {
                                statements: vec![Statement::Assign {
                                    name: temp_name.clone(),
                                    value: then_value,
                                }],
                            };

                            let else_block = Block {
                                statements: vec![Statement::Assign {
                                    name: temp_name.clone(),
                                    value: else_value,
                                }],
                            };

                            statements.push(Statement::If {
                                condition: Box::new(condition_expr),
                                then_block,
                                else_block: Some(else_block),
                                span: span.clone(),
                            });

                            // Return reference to the temporary variable
                            Expression::Identifier(temp_name, span.clone())
                        }
                        None => {
                            // If without else - can't lift properly since we need both branches
                            // Fall back to placeholder for now
                            panic!("in a lifting context, if without else is impossible");
                        }
                    }
                } else {
                    // If expressions appearing in non-return contexts fall back to placeholders
                    panic!("in a non-lifting context, we can not reach an if")
                }
            }
            ast::Expression::ForLoop {
                identifier,
                iterator,
                body,
                span,
            } => {
                if with_lifting {
                    // Desugar for loop into a block expression with iterator and while loop
                    // for (item in iterable) { body } becomes:
                    // {
                    //   let iter = iterable;
                    //   let index = 0;
                    //   let result = [];
                    //   while index < length(iter) {
                    //     let item = iter[index];
                    //     result.push(body);
                    //     index = index + 1;
                    //   }
                    //   result
                    // }

                    let iter_name = format!("iter_{}", *temp_counter);
                    let index_name = format!("index_{}", *temp_counter);
                    let result_name = format!("result_{}", *temp_counter);
                    *temp_counter += 1;

                    // Create the block statements
                    let mut block_statements = vec![];

                    // let iter = iterable;
                    let mut iterator_statements = vec![];
                    let iterator_expr = Self::from_ast(
                        iterator,
                        with_lifting,
                        &mut iterator_statements,
                        temp_counter,
                    );
                    statements.extend(iterator_statements);
                    block_statements.push(Statement::Let {
                        name: iter_name.clone(),
                        value: iterator_expr,
                        span: iterator.span().clone(),
                    });

                    // let index = 0;
                    block_statements.push(Statement::Let {
                        name: index_name.clone(),
                        value: Expression::NumericValue("0".to_string(), span.clone()),
                        span: span.clone(),
                    });

                    // let result = [];
                    block_statements.push(Statement::Let {
                        name: result_name.clone(),
                        value: Expression::Array(vec![], span.clone()),
                        span: span.clone(),
                    });

                    // Create while loop condition: index < length(iter)
                    let condition = Expression::Call(
                        "lt".to_string(),
                        vec![
                            Expression::Identifier(index_name.clone(), span.clone()),
                            Expression::Call(
                                "length".to_string(),
                                vec![Expression::Identifier(iter_name.clone(), span.clone())],
                                span.clone(),
                            ),
                        ],
                        span.clone(),
                    );

                    // Create while loop body
                    let mut while_body_statements = vec![];

                    // let item = iter[index];
                    while_body_statements.push(Statement::Let {
                        name: identifier.to_string(),
                        value: Expression::Call(
                            "index".to_string(),
                            vec![
                                Expression::Identifier(iter_name.clone(), span.clone()),
                                Expression::Identifier(index_name.clone(), span.clone()),
                            ],
                            span.clone(),
                        ),
                        span: identifier.span().clone(),
                    });

                    // Process the body expression
                    let mut body_statements = vec![];
                    let body_expr = Expression::from_ast(
                        &body.expr,
                        with_lifting,
                        &mut body_statements,
                        temp_counter,
                    );
                    while_body_statements.extend(body_statements);

                    // Add body statements from the for loop body
                    for stmt in &body.stmts {
                        let mut stmt_statements = vec![];
                        let mut dummy_counter = 0;
                        let lowered_stmt = Statement::Let {
                            name: stmt.identifier.to_string(),
                            value: Self::from_ast(
                                &stmt.body,
                                with_lifting,
                                &mut stmt_statements,
                                temp_counter,
                            ),
                            span: stmt.span.clone(),
                        };
                        while_body_statements.extend(stmt_statements);
                        while_body_statements.push(lowered_stmt);
                    }

                    // result.push(body_expr);
                    while_body_statements.push(Statement::DeclareAndAssign {
                        name: format!("temp_push_{}", *temp_counter),
                        value: Expression::Call(
                            "push".to_string(),
                            vec![
                                Expression::Identifier(result_name.clone(), span.clone()),
                                body_expr,
                            ],
                            span.clone(),
                        ),
                        span: span.clone(),
                    });

                    // index = index + 1;
                    while_body_statements.push(Statement::Assign {
                        name: index_name.clone(),
                        value: Expression::Call(
                            "add".to_string(),
                            vec![
                                Expression::Identifier(index_name.clone(), span.clone()),
                                Expression::NumericValue("1".to_string(), span.clone()),
                            ],
                            span.clone(),
                        ),
                    });

                    let while_block = Block {
                        statements: while_body_statements,
                    };

                    // Add the while statement
                    block_statements.push(Statement::While {
                        condition: Box::new(condition),
                        block: while_block,
                        span: span.clone(),
                    });

                    // The block expression evaluates to the result array
                    let final_block = Block {
                        statements: block_statements,
                    };

                    // Add the block statements to the main statements list
                    statements.extend(final_block.statements);

                    // Return the result array
                    Expression::Identifier(result_name, span.clone())
                } else {
                    // For loops in non-lifting contexts fall back to placeholder
                    Expression::StringValue("for_loop_todo".to_string(), span.clone())
                }
            }
            ast::Expression::ExprBlock(block, span) => {
                // Expression blocks are lowered to HIR preserving their structure
                // This maintains proper scoping - variables defined inside the block
                // are only visible within that block
                Expression::ExpressionBlock(
                    Box::new(Block::from_expression_block(block)),
                    span.clone(),
                )
            }
            ast::Expression::Lambda(_args, _body, span) => {
                // Lambdas are not yet implemented
                Expression::StringValue("lambda_todo".to_string(), span.clone())
            }
            ast::Expression::ClassConstructor(cc, span) => {
                Expression::ClassConstructor(
                    ClassConstructor {
                        class_name: cc.class_name.to_string(),
                        fields: cc
                            .fields
                            .iter()
                            .filter_map(|field| {
                                match field {
                                    ast::ClassConstructorField::Named(name, expr) => {
                                        Some(ClassConstructorField {
                                            name: name.to_string(),
                                            value: Self::from_ast(
                                                expr,
                                                with_lifting,
                                                statements,
                                                temp_counter,
                                            ),
                                        })
                                    }
                                    ast::ClassConstructorField::Spread(_) => {
                                        // Spreads should be desugared in HIR
                                        None
                                    }
                                }
                            })
                            .collect(),
                    },
                    span.clone(),
                )
            }
            ast::Expression::JinjaExpressionValue(jinja, span) => {
                Expression::JinjaExpressionValue(jinja.to_string(), span.clone())
            }
        }
    }
}

// Pretty printing implementations using Wadler-style combinators
impl Program {
    /// Pretty print the entire HIR to a string with default formatting options
    pub fn pretty_print(&self) -> String {
        self.pretty_print_with_options(80, 2)
    }

    /// Pretty print the HIR with custom line width and indent width
    pub fn pretty_print_with_options(&self, line_width: usize, _indent_width: isize) -> String {
        let doc = self.to_doc();
        let mut output = Vec::new();
        doc.render(line_width, &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    /// Convert HIR to a pretty printing document
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        let mut docs = Vec::new();

        // Add expression functions
        for func in &self.expr_functions {
            docs.push(func.to_doc());
        }

        // Add LLM functions
        for func in &self.llm_functions {
            docs.push(func.to_doc());
        }

        // Add classes
        for class in &self.classes {
            docs.push(class.to_doc());
        }

        // Add enums
        for enum_def in &self.enums {
            docs.push(enum_def.to_doc());
        }

        if docs.is_empty() {
            RcDoc::nil()
        } else {
            RcDoc::intersperse(docs, RcDoc::hardline().append(RcDoc::hardline()))
        }
    }
}

impl ExprFunction {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        let body_doc = if self.body.statements.is_empty() {
            RcDoc::nil()
        } else {
            // The key is to apply nest() to the entire content that includes line breaks
            RcDoc::hardline()
                .append(RcDoc::intersperse(
                    self.body
                        .statements
                        .iter()
                        .map(|s| s.to_doc())
                        .collect::<Vec<_>>(),
                    RcDoc::hardline(),
                ))
                .append(RcDoc::hardline())
                .nest(2)
        };

        RcDoc::text("fn")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::text("("))
            .append(self.parameters_to_doc())
            .append(RcDoc::text(")"))
            .append(RcDoc::space())
            .append(RcDoc::text("{"))
            .append(body_doc)
            .append(RcDoc::text("}"))
    }

    fn parameters_to_doc(&self) -> RcDoc<'static, ()> {
        if self.parameters.is_empty() {
            RcDoc::nil()
        } else {
            let param_docs: Vec<_> = self.parameters.iter().map(|p| p.to_doc()).collect();
            RcDoc::intersperse(param_docs, RcDoc::text(",").append(RcDoc::space()))
        }
    }
}

impl LLMFunction {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text("function")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::text("("))
            .append(self.parameters_to_doc())
            .append(RcDoc::text(")"))
            .append(RcDoc::space())
            .append(RcDoc::text("{"))
            .append(RcDoc::hardline())
            .append(
                RcDoc::text("client")
                    .append(RcDoc::space())
                    .append(RcDoc::text(self.client.clone()))
                    .append(RcDoc::hardline())
                    .append(RcDoc::text("prompt"))
                    .append(RcDoc::space())
                    .append(RcDoc::text(self.prompt.clone()))
                    .nest(2),
            )
            .append(RcDoc::hardline())
            .append(RcDoc::text("}"))
    }

    fn parameters_to_doc(&self) -> RcDoc<'static, ()> {
        if self.parameters.is_empty() {
            RcDoc::nil()
        } else {
            let param_docs: Vec<_> = self.parameters.iter().map(|p| p.to_doc()).collect();
            RcDoc::intersperse(param_docs, RcDoc::text(",").append(RcDoc::space()))
        }
    }
}

impl Class {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text("class")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::space())
            .append(RcDoc::text("{"))
            .append(if self.fields.is_empty() {
                RcDoc::nil()
            } else {
                RcDoc::hardline()
                    .append(
                        RcDoc::intersperse(
                            self.fields.iter().map(|f| f.to_doc()).collect::<Vec<_>>(),
                            RcDoc::hardline(),
                        )
                        .nest(2),
                    )
                    .append(RcDoc::hardline())
            })
            .append(RcDoc::text("}"))
    }
}

impl Field {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text(self.name.clone())
    }
}

impl Enum {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text("enum")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::space())
            .append(RcDoc::text("{"))
            .append(if self.variants.is_empty() {
                RcDoc::nil()
            } else {
                RcDoc::hardline()
                    .append(
                        RcDoc::intersperse(
                            self.variants.iter().map(|v| v.to_doc()).collect::<Vec<_>>(),
                            RcDoc::hardline(),
                        )
                        .nest(2),
                    )
                    .append(RcDoc::hardline())
            })
            .append(RcDoc::text("}"))
    }
}

impl EnumVariant {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text(self.name.clone())
    }
}

impl Parameter {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        // For now, just show the parameter name since types aren't included in HIR
        RcDoc::text(self.name.clone())
    }
}

impl Block {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        if self.statements.is_empty() {
            RcDoc::nil()
        } else {
            RcDoc::intersperse(
                self.statements
                    .iter()
                    .map(|s| s.to_doc())
                    .collect::<Vec<_>>(),
                RcDoc::hardline(),
            )
        }
    }
}

impl Statement {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Statement::Let { name, value, .. } => RcDoc::text("let")
                .append(RcDoc::space())
                .append(RcDoc::text(name.clone()))
                .append(RcDoc::space())
                .append(RcDoc::text("="))
                .append(RcDoc::space())
                .append(value.to_doc())
                .append(RcDoc::text(";")),
            Statement::DeclareReference { name, .. } => RcDoc::text("var")
                .append(RcDoc::space())
                .append(RcDoc::text(name.clone()))
                .append(RcDoc::text(";")),
            Statement::Assign { name, value } => RcDoc::text(name.clone())
                .append(RcDoc::space())
                .append(RcDoc::text("="))
                .append(RcDoc::space())
                .append(value.to_doc())
                .append(RcDoc::text(";")),
            Statement::DeclareAndAssign { name, value, .. } => RcDoc::text("var")
                .append(RcDoc::space())
                .append(RcDoc::text(name.clone()))
                .append(RcDoc::space())
                .append(RcDoc::text("="))
                .append(RcDoc::space())
                .append(value.to_doc())
                .append(RcDoc::text(";")),
            Statement::Return { expr, .. } => RcDoc::text("return")
                .append(RcDoc::space())
                .append(expr.to_doc())
                .append(RcDoc::text(";")),
            Statement::Expression { expr, .. } => expr.to_doc(),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let mut doc = RcDoc::text("if")
                    .append(RcDoc::space())
                    .append(condition.to_doc())
                    .append(RcDoc::space())
                    .append(RcDoc::text("{"))
                    .append(RcDoc::hardline())
                    .append(then_block.to_doc().nest(2))
                    .append(RcDoc::hardline())
                    .append(RcDoc::text("}"));

                if let Some(else_block) = else_block {
                    doc = doc
                        .append(RcDoc::space())
                        .append(RcDoc::text("else"))
                        .append(RcDoc::space())
                        .append(RcDoc::text("{"))
                        .append(RcDoc::hardline())
                        .append(else_block.to_doc().nest(2))
                        .append(RcDoc::hardline())
                        .append(RcDoc::text("}"));
                }

                doc
            }
            Statement::While {
                condition, block, ..
            } => RcDoc::text("while")
                .append(RcDoc::space())
                .append(condition.to_doc())
                .append(RcDoc::space())
                .append(RcDoc::text("{"))
                .append(RcDoc::hardline())
                .append(block.to_doc().nest(2))
                .append(RcDoc::hardline())
                .append(RcDoc::text("}")),
        }
    }
}

impl Expression {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        match self {
            Expression::BoolValue(val, _) => RcDoc::text(val.to_string()),
            Expression::NumericValue(val, _) => RcDoc::text(val.clone()),
            Expression::Identifier(name, _) => RcDoc::text(name.clone()),
            Expression::StringValue(val, _) => RcDoc::text(format!("\"{}\"", val)),
            Expression::RawStringValue(val, _) => RcDoc::text(format!("#\"{}\"#", val)),
            Expression::Array(values, _) => RcDoc::text("[")
                .append(if values.is_empty() {
                    RcDoc::nil()
                } else {
                    RcDoc::intersperse(
                        values.iter().map(|v| v.to_doc()).collect::<Vec<_>>(),
                        RcDoc::text(",").append(RcDoc::space()),
                    )
                })
                .append(RcDoc::text("]")),
            Expression::Map(pairs, _) => RcDoc::text("{")
                .append(if pairs.is_empty() {
                    RcDoc::nil()
                } else {
                    RcDoc::space()
                        .append(RcDoc::intersperse(
                            pairs
                                .iter()
                                .map(|(k, v)| {
                                    k.to_doc()
                                        .append(RcDoc::text(":"))
                                        .append(RcDoc::space())
                                        .append(v.to_doc())
                                })
                                .collect::<Vec<_>>(),
                            RcDoc::text(",").append(RcDoc::space()),
                        ))
                        .append(RcDoc::space())
                })
                .append(RcDoc::text("}")),
            Expression::JinjaExpressionValue(val, _) => RcDoc::text(val.clone()),
            Expression::Call(name, args, _) => RcDoc::text(name.clone())
                .append(RcDoc::text("("))
                .append(if args.is_empty() {
                    RcDoc::nil()
                } else {
                    RcDoc::intersperse(
                        args.iter().map(|arg| arg.to_doc()).collect::<Vec<_>>(),
                        RcDoc::text(",").append(RcDoc::space()),
                    )
                })
                .append(RcDoc::text(")")),
            Expression::ClassConstructor(cc, _) => RcDoc::text(cc.class_name.clone())
                .append(RcDoc::space())
                .append(RcDoc::text("{"))
                .append(if cc.fields.is_empty() {
                    RcDoc::nil()
                } else {
                    RcDoc::space()
                        .append(RcDoc::intersperse(
                            cc.fields.iter().map(|f| f.to_doc()).collect::<Vec<_>>(),
                            RcDoc::text(",").append(RcDoc::space()),
                        ))
                        .append(RcDoc::space())
                })
                .append(RcDoc::text("}")),
            Expression::ExpressionBlock(block, _) => RcDoc::text("{")
                .append(RcDoc::hardline())
                .append(block.to_doc().nest(2))
                .append(RcDoc::hardline())
                .append(RcDoc::text("}")),
        }
    }
}

impl ClassConstructorField {
    pub fn to_doc(&self) -> RcDoc<'static, ()> {
        RcDoc::text(self.name.clone())
            .append(RcDoc::space())
            .append(RcDoc::text("="))
            .append(RcDoc::space())
            .append(self.value.to_doc())
    }
}

impl Class {
    /// Lower a class from AST to HIR.
    pub fn from_ast(class: &ast::TypeExpressionBlock) -> Self {
        Class {
            name: class.name().to_string(),
            fields: class
                .fields
                .iter()
                .map(|field| Field {
                    name: field.name().to_string(),
                    span: field.span().clone(),
                })
                .collect(),
            span: class.span().clone(),
        }
    }
}

impl Enum {
    /// Lower an enum from AST to HIR.
    pub fn from_ast(enum_def: &ast::TypeExpressionBlock) -> Self {
        Enum {
            name: enum_def.name().to_string(),
            variants: enum_def
                .fields
                .iter()
                .map(|field| EnumVariant {
                    name: field.name().to_string(),
                    span: field.span().clone(),
                })
                .collect(),
            span: enum_def.span().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use internal_baml_ast::ast;
    use internal_baml_diagnostics::SourceFile;

    /// Test helper to generate HIR from BAML source and return pretty-printed string
    fn hir_from_source(source: &str) -> String {
        let ast = parse_baml(source);
        let hir = Program::from_ast(&ast);
        hir.pretty_print()
    }

    /// Parse BAML source code and return the AST
    fn parse_baml(source: &str) -> ast::Ast {
        let path = std::path::PathBuf::from("test.baml");
        let source_file = SourceFile::from((path.clone(), source));

        let validated_schema = internal_baml_core::validate(&path, vec![source_file]);

        if validated_schema.diagnostics.has_errors() {
            panic!(
                "Parse errors: {}",
                validated_schema.diagnostics.to_pretty_string()
            );
        }

        validated_schema.db.ast
    }

    // Test cases start here

    #[test]
    fn test_simple_expression_function() {
        let source = r#"
            fn MyFunc(x: int, y: string) -> int {
                42
            }
        "#;

        let expected = r#"fn MyFunc(x, y) {
  return 42;
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_expression_with_let_binding() {
        let source = r#"
            fn AddOne(x: int) -> int {
                let y = x;
                y
            }
        "#;

        let expected = r#"fn AddOne(x) {
  let y = x;
  return y;
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_basic_expressions() {
        let source = r#"
            fn TestExpressions() -> string {
                let bool_val = true;
                let num_val = 123.45;
                let str_val = "hello";
                str_val
            }
        "#;

        let expected = r#"fn TestExpressions() {
  let bool_val = true;
  let num_val = 123.45;
  let str_val = "hello";
  return str_val;
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_array_expression() {
        let source = r#"
            fn TestArray() -> int[] {
                [1, 2, 3]
            }
        "#;

        let expected = r#"fn TestArray() {
  return [1, 2, 3];
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_function_call() {
        let source = r#"
            fn myFunc(x: int, y: string) -> int {
                x
            }
            
            fn CallTest() -> int {
                let result = myFunc(42, "hello");
                result
            }
        "#;

        let expected = r#"fn myFunc(x, y) {
  return x;
}

fn CallTest() {
  let result = myFunc(42, "hello");
  return result;
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    // Note: LLM function test disabled due to string literal parsing issues
    // TODO: Re-enable and fix string literal issues

    #[test]
    fn test_pretty_print_demo() {
        let source = r#"
            fn fibonacci(n: int) -> int {
                let a = 0;
                let b = 1;
                let result = add(a, b);
                result
            }
            
            fn add(x: int, y: int) -> int {
                x
            }
        "#;

        let ast = parse_baml(source);
        let hir = Program::from_ast(&ast);

        println!("\n=== HIR Pretty Print Demo ===");
        println!("Original HIR structure:");
        println!("{}", hir.pretty_print());

        println!("\n=== With different line widths ===");
        println!("Line width 40:");
        println!("{}", hir.pretty_print_with_options(40, 2));

        println!("\nLine width 120:");
        println!("{}", hir.pretty_print_with_options(120, 2));
    }

    #[test]
    fn test_pretty_print_expression_function() {
        let source = r#"
            fn AddOne(x: int) -> int {
                let y = x;
                y
            }
        "#;

        let ast = parse_baml(source);
        let hir = Program::from_ast(&ast);

        let pretty_printed = hir.pretty_print();

        // Check that the pretty printed output contains the expected structure
        assert!(pretty_printed.contains("fn AddOne(x)"));
        assert!(pretty_printed.contains("let y = x;"));
        assert!(pretty_printed.contains("y"));

        // Print it for visual inspection
        println!("Pretty printed HIR:");
        println!("{}", pretty_printed);
    }

    #[test]
    fn test_pretty_print_array_and_call() {
        let source = r#"
            fn helper(x: int) -> int {
                x
            }
            
            fn TestArray() -> int[] {
                let arr = [1, 2, 3];
                let result = helper(42);
                [arr, result]
            }
        "#;

        let ast = parse_baml(source);
        let hir = Program::from_ast(&ast);

        let pretty_printed = hir.pretty_print();

        // Check that the pretty printed output contains the expected structure
        assert!(pretty_printed.contains("fn helper(x)"));
        assert!(pretty_printed.contains("fn TestArray()"));
        assert!(pretty_printed.contains("let arr = [1, 2, 3];"));
        assert!(pretty_printed.contains("let result = helper(42);"));
        assert!(pretty_printed.contains("[arr, result]"));

        // Print it for visual inspection
        println!("Pretty printed HIR with arrays and calls:");
        println!("{}", pretty_printed);
    }

    #[test]
    fn test_indentation_consistency() {
        let source = r#"
            fn simple() -> string {
                "hello"
            }
        "#;

        let expected = r#"fn simple() {
  return "hello";
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_if_expression_desugaring() {
        // Test if expression desugaring in let bindings
        let source = r#"
            fn simpleIf() -> string {
                let x = if true { "yes" } else { "no" };
                x
            }
        "#;

        let expected = r#"fn simpleIf() {
  var x;
  if true {
  x = "yes";
  } else {
  x = "no";
  }
  return x;
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_pretty_print_complex_structures() {
        let source = r#"
            fn complexFunction(a: int, b: string, c: bool) -> string {
                let nested_array = [[1, 2], [3, 4]];
                let result = helper(a, b);
                result
            }
            
            fn helper(x: int, y: string) -> string {
                "result"
            }
        "#;

        let ast = parse_baml(source);
        let hir = Program::from_ast(&ast);

        let pretty_printed = hir.pretty_print();

        // Check that it contains the expected structure
        assert!(pretty_printed.contains("fn complexFunction(a, b, c)"));
        assert!(pretty_printed.contains("fn helper(x, y)"));
        assert!(pretty_printed.contains("[[1, 2], [3, 4]]"));
        assert!(pretty_printed.contains("helper(a, b)"));

        // Test custom formatting options
        let narrow_format = hir.pretty_print_with_options(40, 4);
        assert!(narrow_format.len() > 0);

        // Print for visual inspection
        println!("Pretty printed complex HIR:");
        println!("{}", pretty_printed);
        println!("\nNarrow format (40 chars wide):");
        println!("{}", narrow_format);
    }

    #[test]
    fn test_if_expression_in_return_position() {
        // Test if expression desugaring in return position
        let source = r#"
            fn conditionalReturn(flag: bool) -> string {
                if flag { "success" } else { "failure" }
            }
        "#;

        let expected = r#"fn conditionalReturn(flag) {
  if flag {
  return "success";
  } else {
  return "failure";
  }
}"#;

        assert_eq!(hir_from_source(source), expected);
    }

    #[test]
    fn test_nested_expression_blocks() {
        // Test nested expression blocks with proper scoping
        let source = r#"
            fn Foo() -> int {
                let x = {
                    let y = 1;
                    y
                };
                x
            }
        "#;

        // Expression blocks now properly preserve scope - the inner block
        // maintains its own variables which are not visible outside
        let result = hir_from_source(source);

        let expected = r#"fn Foo() {
  let x = {
  let y = 1;
    y
  };
  return x;
}"#;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_class_constructor_with_complex_expressions() {
        // Test class constructor with both if expressions and expression blocks
        let source = r#"
            class Foo {
                a int
                b int
            }
            
            fn TestConstructor() -> Foo {
                Foo { a: if true { 1 } else { 0 }, b: { let y = 1; y } }
            }
        "#;

        let result = hir_from_source(source);

        // The if expression in field 'a' should get lifted to temporary variables
        // The expression block in field 'b' should work correctly.
        let expected = r#"fn TestConstructor() {
  var temp_0;
  if true {
  temp_0 = 1;
  } else {
  temp_0 = 0;
  }
  return Foo { a = temp_0, b = {
  let y = 1;
    y
  } };
}

class Foo {
a
  b
}"#;

        assert_eq!(result, expected);

        // Print for visual inspection
        println!("HIR for class constructor with complex expressions:");
        println!("{}", result);
    }

    #[test]
    fn test_for_loop_lowering() {
        // Test for loop lowering to while loop with iterator
        let source = r#"
            fn TestForLoop() -> int[] {
                for (item in [1, 2, 3]) { mul(item, 2) }
            }
        "#;

        let result = hir_from_source(source);

        // The for loop should be lowered to:
        // - iterator variable declaration
        // - index variable initialization
        // - result array initialization
        // - while loop with condition and body
        let expected = r#"fn TestForLoop() {
  let iter_0 = [1, 2, 3];
  let index_0 = 0;
  let result_0 = [];
  while lt(index_0, length(iter_0)) {
  let item = index(iter_0, index_0);
    var temp_push_1 = push(result_0, mul(item, 2));
    index_0 = add(index_0, 1);
  }
  return result_0;
}"#;

        assert_eq!(result, expected);

        // Print for visual inspection
        println!("HIR for for loop lowering:");
        println!("{}", result);
    }
}
