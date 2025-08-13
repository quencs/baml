use internal_baml_diagnostics::{DatamodelError, Diagnostics};

use super::{
    helpers::{parsing_catch_all, Pair},
    parse_identifier::parse_identifier,
    Rule,
};
use crate::{
    assert_correct_parser,
    ast::{
        self, expr::ExprFn, App, ArgumentsList, AssignOp, AssignOpStmt, AssignStmt, ExprStmt,
        Expression, ExpressionBlock, ForLoopStmt, LetStmt, Stmt, TopLevelAssignment, *,
    },
    parser::{
        parse_arguments::parse_arguments_list, parse_expression::parse_expression,
        parse_field::parse_field_type_chain, parse_identifier,
        parse_named_args_list::parse_named_argument_list, parse_types::parse_field_type,
    },
    unreachable_rule,
};

pub fn parse_expr_fn(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<expr::ExprFn> {
    assert_correct_parser!(token, Rule::expr_fn);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();
    let name = parse_identifier(tokens.next()?, diagnostics);
    let args = parse_named_argument_list(tokens.next()?, diagnostics);
    let arrow_or_body = tokens.next()?;

    // We may or may not have an arrow and a return type.
    // If the args list is immediately followed by an arrow, we have an arrow and a return type.
    // Otherwise, we have just a body.
    let (maybe_return_type, maybe_body) = if matches!(arrow_or_body.as_rule(), Rule::ARROW) {
        let return_type = parse_field_type_chain(tokens.next()?, diagnostics);
        let function_body = parse_function_body(tokens.next()?, diagnostics);
        (Some(return_type), function_body)
    } else {
        diagnostics.push_error(DatamodelError::new_static(
            "function must have a return type: e.g. function Foo() -> int",
            span.clone(),
        ));
        let function_body = parse_function_body(arrow_or_body, diagnostics);
        (None, function_body)
    };
    match (maybe_return_type, maybe_body) {
        (Some(return_type), Some(body)) => Some(ExprFn {
            name,
            args,
            return_type,
            body,
            span,
            annotations: vec![],
        }),
        _ => None,
    }
}

pub fn parse_top_level_assignment(
    token: Pair<'_>,
    diagnostics: &mut Diagnostics,
) -> Option<expr::TopLevelAssignment> {
    assert_correct_parser!(token, Rule::top_level_assignment);
    let mut tokens = token.into_inner();

    match parse_statement(tokens.next()?, diagnostics)? {
        Stmt::Let(stmt) => Some(TopLevelAssignment { stmt }),
        Stmt::Assign(stmt) => {
            // NOTE: (Jesus) top-level is generally regarded as order-independent,
            // and assignments need an order of execution.

            diagnostics.push_error(DatamodelError::new_static(
                "assignments are not allowed at top level, only let statements are allowed",
                stmt.span.clone(),
            ));

            None
        }
        Stmt::AssignOp(stmt) => {
            diagnostics.push_error(DatamodelError::new_static(
                "assign operations are not allowed at top level, only let statements are allowed",
                stmt.span.clone(),
            ));

            None
        }

        Stmt::ForLoop(stmt) => {
            diagnostics.push_error(DatamodelError::new_static(
                "for loops are not allowed at top level, only let statements are allowed",
                stmt.span.clone(),
            ));

            None
        }

        Stmt::Expression(es) => {
            diagnostics.push_error(DatamodelError::new_static(
                "expressions are not allowed at top level, only let statements are allowed",
                es.span.clone(),
            ));

            None
        }
    }
}

pub fn parse_for_loop(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Stmt> {
    assert_correct_parser!(token, Rule::for_loop);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();
    let identifier = parse_identifier(tokens.next()?, diagnostics);
    let iterator = parse_expression(tokens.next()?, diagnostics)?;
    let body = parse_expr_block(tokens.next()?, diagnostics)?;
    Some(Stmt::ForLoop(ForLoopStmt {
        identifier,
        iterator,
        body,
        span,
        annotations: vec![],
    }))
}

pub fn parse_statement(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Stmt> {
    assert_correct_parser!(token, Rule::stmt);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();

    let stmt_token = tokens.next()?;
    let stmt = match stmt_token.as_rule() {
        Rule::assign_stmt => {
            let mut assignment_tokens = stmt_token.into_inner();

            let identifier = parse_identifier(assignment_tokens.next()?, diagnostics);

            let rhs = assignment_tokens.next()?;
            let rhs_span = diagnostics.span(rhs.as_span());
            let maybe_body = parse_assignment_expr(diagnostics, rhs, rhs_span);
            maybe_body.map(|body| {
                Stmt::Assign(AssignStmt {
                    identifier,
                    expr: body,
                    span: span.clone(),
                })
            })
        }
        Rule::assign_op_stmt => {
            let mut assignment_tokens = stmt_token.into_inner();

            let identifier = parse_identifier(assignment_tokens.next()?, diagnostics);

            let op_token = assignment_tokens.next()?;

            let assign_op = match op_token.as_rule() {
                Rule::ADD_ASSIGN => AssignOp::AddAssign,
                Rule::SUB_ASSIGN => AssignOp::SubAssign,
                Rule::MUL_ASSIGN => AssignOp::MulAssign,
                Rule::DIV_ASSIGN => AssignOp::DivAssign,
                Rule::MOD_ASSIGN => AssignOp::ModAssign,
                Rule::BIT_AND_ASSIGN => AssignOp::BitAndAssign,
                Rule::BIT_OR_ASSIGN => AssignOp::BitOrAssign,
                Rule::BIT_XOR_ASSIGN => AssignOp::BitXorAssign,
                Rule::BIT_SHL_ASSIGN => AssignOp::ShlAssign,
                Rule::BIT_SHR_ASSIGN => AssignOp::ShrAssign,
                other => unreachable_rule!(op_token, other),
            };

            let rhs = assignment_tokens.next()?;
            let rhs_span = diagnostics.span(rhs.as_span());

            let maybe_body = parse_assignment_expr(diagnostics, rhs, rhs_span);

            maybe_body.map(|body| {
                Stmt::AssignOp(AssignOpStmt {
                    identifier,
                    assign_op,
                    expr: body,
                    span: span.clone(),
                })
            })
        }
        Rule::let_expr => {
            let mut let_binding_tokens = stmt_token.into_inner();

            let is_mutable = if let Rule::MUT_KEYWORD = let_binding_tokens.peek()?.as_rule() {
                let_binding_tokens.next()?;
                true
            } else {
                false
            };

            let identifier = parse_identifier(let_binding_tokens.next()?, diagnostics);

            let rhs = let_binding_tokens.next()?;
            let rhs_span = diagnostics.span(rhs.as_span());
            let maybe_body = parse_assignment_expr(diagnostics, rhs, rhs_span);
            maybe_body.map(|body| {
                Stmt::Let(LetStmt {
                    identifier,
                    is_mutable,
                    expr: body,
                    span: span.clone(),
                    annotations: vec![],
                })
            })
        }
        Rule::for_loop => parse_for_loop(stmt_token, diagnostics),
        Rule::if_expression => parse_if_expression(stmt_token, diagnostics).map(|expr| {
            Stmt::Expression(ExprStmt {
                expr,
                annotations: vec![],
                span: span.clone(),
            })
        }),
        Rule::fn_app => parse_fn_app(stmt_token, diagnostics).map(|expr| {
            Stmt::Expression(ExprStmt {
                expr,
                annotations: vec![],
                span: span.clone(),
            })
        }),
        Rule::generic_fn_app => parse_generic_fn_app(stmt_token, diagnostics).map(|expr| {
            Stmt::Expression(ExprStmt {
                expr,
                annotations: vec![],
                span: span.clone(),
            })
        }),
        _ => {
            diagnostics.push_error(DatamodelError::new_static(
                "Expected let expression or for loop",
                span.clone(),
            ));
            None
        }
    };

    let maybe_semicolon = tokens.next();
    match maybe_semicolon {
        Some(p) if p.as_str() == ";" => {}
        _ => {
            if matches!(stmt, Some(Stmt::Let(_))) {
                diagnostics.push_error(DatamodelError::new_static(
                    "Statement must end with a semicolon.",
                    span.clone(),
                ));
            }
        }
    }

    stmt
}

fn parse_assignment_expr(
    diagnostics: &mut Diagnostics,
    rhs: Pair<'_>,
    rhs_span: Span,
) -> Option<Expression> {
    match rhs.as_rule() {
        Rule::expr_block => {
            let block_span = diagnostics.span(rhs.as_span());
            let maybe_expr_block = parse_expr_block(rhs, diagnostics);
            maybe_expr_block.map(|expr_block| Expression::ExprBlock(expr_block, block_span))
        }
        Rule::expression => parse_expression(rhs, diagnostics),
        _ => {
            diagnostics.push_error(DatamodelError::new_static(
                "Parser only allows expr_block and expr here",
                rhs_span,
            ));
            None
        }
    }
}

pub fn parse_expr_block(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<ExpressionBlock> {
    assert_correct_parser!(token, Rule::expr_block);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();
    let mut stmts = Vec::new();
    let mut expr = None;
    let _open_bracket = tokens.next()?;

    // Collect all items first to process headers together
    let mut items: Vec<Pair<'_>> = Vec::new();
    for item in tokens {
        items.push(item);
    }

    // Track headers with their hierarchy
    let mut all_headers_in_block: Vec<std::sync::Arc<Header>> = Vec::new();

    // First pass: collect all headers
    for item in &items {
        match item.as_rule() {
            Rule::mdx_header => {
                let header = parse_header(item.clone(), diagnostics);
                if let Some(header) = header {
                    let header_arc = std::sync::Arc::new(header);
                    all_headers_in_block.push(header_arc.clone());
                }
            }
            _ => {}
        }
    }

    // Normalize all headers in the block together
    normalize_headers(&mut all_headers_in_block);

    // Debug: Print normalized headers (disabled)
    // println!("PARSER: Normalized headers in block:");
    // for (i, header) in all_headers_in_block.iter().enumerate() {
    //     println!("  [{}] '{}' (Level: {})", i, header.title, header.level);
    // }

    // Second pass: process statements and expressions with normalized headers
    let mut current_headers: Vec<std::sync::Arc<Header>> = Vec::new();
    let mut headers_since_last_stmt: Vec<std::sync::Arc<Header>> = Vec::new();

    for item in items {
        match item.as_rule() {
            Rule::stmt => {
                let maybe_stmt = parse_statement(item, diagnostics);
                if let Some(mut stmt) = maybe_stmt {
                    // Bind only the headers that were declared since the last statement
                    bind_headers_to_statement(&mut stmt, &headers_since_last_stmt);
                    stmts.push(stmt);

                    // Clear headers since last statement
                    headers_since_last_stmt.clear();
                }
            }
            Rule::expression => {
                let maybe_expr = parse_expression(item, diagnostics);
                if let Some(parsed_expr) = maybe_expr {
                    expr = Some(parsed_expr);
                    continue;
                }
            }
            Rule::mdx_header => {
                // Headers are already processed, just update current headers
                let header = parse_header(item, diagnostics);
                if let Some(header) = header {
                    let header_arc = std::sync::Arc::new(header);

                    // Find the corresponding normalized header
                    if let Some(normalized_header) = all_headers_in_block
                        .iter()
                        .find(|h| h.title == header_arc.title)
                    {
                        // Implement header hierarchy logic
                        filter_headers_by_hierarchy(&mut current_headers, normalized_header);

                        // Add to current headers and headers since last statement
                        current_headers.push(normalized_header.clone());
                        headers_since_last_stmt.push(normalized_header.clone());
                    }
                }
            }
            Rule::BLOCK_CLOSE => {
                // Commentend out because we can't have blocks without return
                // expressions otherwise. Plus we need functions with no return
                // types as well.

                // if expr.is_none() {
                //     diagnostics.push_error(DatamodelError::new_static(
                //         "Function must end in an expression.",
                //         span.clone(),
                //     ));
                // }
                break;
            }
            Rule::NEWLINE => {
                continue;
            }
            Rule::comment_block => {
                // Skip comments in function bodies
                continue;
            }
            Rule::empty_lines => {
                // Skip empty lines in function bodies
                continue;
            }
            _ => {
                diagnostics.push_error(DatamodelError::new_static(
                    "Internal Error: Parser only allows statements and expressions in function body.",
                    span.clone()
                ));
            }
        }
    }

    let mut return_expr = expr.map(Box::new);

    // Special case for returning if expressions.
    // TODO: Likely there's no need to separate statements and final expression
    // since a statement can now be an expression. We just need to allow any
    // random expression as a statement as mentioned in the grammar file.
    if return_expr.is_none()
        && matches!(
            stmts.last(),
            Some(Stmt::Expression(ExprStmt {
                expr: Expression::If(..),
                ..
            }))
        )
    {
        let Some(Stmt::Expression(ExprStmt { expr: e, .. })) = stmts.pop() else {
            unreachable!();
        };

        return_expr = Some(Box::new(e));
    }

    Some(ExpressionBlock {
        stmts,
        expr: return_expr,
        expr_headers: headers_since_last_stmt,
    })
}

/// Parse a single header from an MDX header token
pub fn parse_header(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Header> {
    let full_text = token.as_str();
    let header_span = diagnostics.span(token.as_span());

    // Find the start of the hash sequence
    let hash_start = full_text.find('#')?;
    let after_whitespace = full_text[hash_start..].trim_start();

    // Count consecutive hash characters
    let hash_count = after_whitespace.chars().take_while(|&c| c == '#').count();

    // Extract the title after the hash sequence and whitespace
    let after_hashes = &after_whitespace[hash_count..];
    let title_text = after_hashes.trim().to_string();

    // Remove trailing newline if present
    let title_text = title_text
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string();

    let level = hash_count as u8;

    // Print debug information about the header (disabled)
    // let indent = " ".repeat(level as usize);
    // println!(
    //     "{}└ HEADER Level {}: '{}' (hash count: {})",
    //     indent, level, title_text, level
    // );

    Some(Header {
        level,
        title: title_text,
        span: header_span,
    })
}

/// Filter headers based on hierarchy rules (markdown-style nesting)
fn filter_headers_by_hierarchy(
    pending_headers: &mut Vec<std::sync::Arc<Header>>,
    new_header: &std::sync::Arc<Header>,
) {
    // Remove headers that are at the same level or deeper than the new header
    // This implements the markdown hierarchy where:
    // - A new header at level N closes all headers at level N or higher
    // - Headers at level N+1, N+2, etc. nest under the header at level N
    pending_headers.retain(|header| header.level < new_header.level);
}

/// Normalize headers within a single block according to the normalization rules:
/// - All headers within a block scope should start from level 1
/// - Maintain relative hierarchy between headers
fn normalize_headers(headers: &mut Vec<std::sync::Arc<Header>>) {
    if headers.is_empty() {
        return;
    }

    // Find the minimum level to normalize from
    let min_level = headers.iter().map(|h| h.level).min().unwrap();

    // Only normalize if headers don't already start from level 1
    if min_level > 1 {
        // Create new normalized headers
        let mut normalized_headers = Vec::new();

        for header in headers.iter() {
            // Normalize by adjusting all levels to start from 1
            let new_level = header.level - min_level + 1;

            // Create new header with normalized level
            let normalized_header = std::sync::Arc::new(Header {
                level: new_level,
                title: header.title.clone(),
                span: header.span.clone(),
            });

            normalized_headers.push(normalized_header);
        }

        // Replace the original headers with normalized ones
        *headers = normalized_headers;
    }
}

/// Bind pending headers to a statement based on scope rules
fn bind_headers_to_statement(stmt: &mut Stmt, pending_headers: &Vec<std::sync::Arc<Header>>) {
    match stmt {
        Stmt::Let(let_stmt) => {
            let_stmt.annotations.extend(pending_headers.clone());
        }
        Stmt::ForLoop(for_stmt) => {
            for_stmt.annotations.extend(pending_headers.clone());
        }
        Stmt::Expression(es) => {
            es.annotations.extend(pending_headers.clone());
        }
        Stmt::Assign(_) => {
            // Assignments do not carry annotations
        }
        Stmt::AssignOp(_) => {
            // Assignment operations do not carry annotations
        }
    }
}

fn parse_fn_args(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Vec<Expression> {
    assert_correct_parser!(token, Rule::fn_args);

    token
        .into_inner()
        .filter_map(|item| parse_expression(item, diagnostics))
        .collect()
}

pub fn parse_fn_app(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Expression> {
    assert_correct_parser!(token, Rule::fn_app);

    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();

    let fn_name = parse_identifier(tokens.next()?, diagnostics);

    let args = parse_fn_args(tokens.next()?, diagnostics);

    Some(Expression::App(App {
        name: fn_name,
        type_args: vec![],
        args,
        span,
    }))
}

/// Parse function application with generic type arguments.
///
/// Grammar rules for this one are a little bit more complicated than for
/// normal functions so can't reuse parse_fn_app easily.
pub fn parse_generic_fn_app(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Expression> {
    assert_correct_parser!(token, Rule::generic_fn_app);

    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();

    // Grab name from generic_fn_app_identifier rule.
    let fn_name = parse_identifier(tokens.next()?.into_inner().next()?, diagnostics);

    // Move into generic_fn_app_args rule.
    tokens = tokens.next()?.into_inner();

    // Parse type argument. Only one for now.
    let type_arg = parse_field_type_chain(tokens.next()?, diagnostics)?;

    // Parse arguments.
    let args = parse_fn_args(tokens.next()?, diagnostics);

    Some(Expression::App(App {
        name: fn_name,
        type_args: vec![type_arg],
        args,
        span,
    }))
}

pub fn parse_lambda(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Expression> {
    assert_correct_parser!(token, Rule::lambda);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();
    let mut args = ArgumentsList {
        arguments: Vec::new(),
    };
    parse_arguments_list(tokens.next()?, &mut args, &None, diagnostics);
    let maybe_body = parse_function_body(tokens.next()?, diagnostics);
    maybe_body.map(|body| Expression::Lambda(args, Box::new(body), span))
}

pub fn parse_function_body(
    token: Pair<'_>,
    diagnostics: &mut Diagnostics,
) -> Option<ExpressionBlock> {
    parse_expr_block(token, diagnostics)
}

pub fn parse_if_expression(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Option<Expression> {
    assert_correct_parser!(token, Rule::if_expression);
    let span = diagnostics.span(token.as_span());
    let mut tokens = token.into_inner();
    let condition = parse_expression(tokens.next()?, diagnostics)?;

    // TODO: Some weird parsing going on here, figure out rules and spans.
    let then_branch_expr_block = tokens.next()?;
    let then_branch_span = diagnostics.span(then_branch_expr_block.as_span());
    let then_branch = parse_expr_block(then_branch_expr_block, diagnostics)?;

    let else_branch_expr = tokens.next()?;
    let else_branch_span = diagnostics.span(else_branch_expr.as_span());

    let else_branch = match else_branch_expr.as_rule() {
        Rule::expr_block => parse_expr_block(else_branch_expr, diagnostics)
            .map(|e| Box::new(Expression::ExprBlock(e, else_branch_span))),

        Rule::if_expression => parse_if_expression(else_branch_expr, diagnostics).map(Box::new),

        _ => unreachable_rule!(else_branch_expr, Rule::if_expression),
    };

    Some(Expression::If(
        Box::new(condition),
        Box::new(Expression::ExprBlock(then_branch, then_branch_span)),
        else_branch,
        span,
    ))
}
