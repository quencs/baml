use std::collections::HashMap;

use baml_vm::{Bytecode, Function, FunctionKind, Instruction, Object, Value};
use internal_baml_core::ast::WithName;
use internal_baml_parser_database::{ast::Expression, ParserDatabase};

/// Generate bytecode.
pub fn compile(ast: ParserDatabase) -> anyhow::Result<(Vec<Function>, Vec<Value>)> {
    // eprintln!("{:#?}", ast.ast);

    let mut resolved_globals = HashMap::new();

    // TODO: Probably we can use Top::Id to map these to VM globals.
    for (i, function) in ast.walk_expr_fns().enumerate() {
        resolved_globals.insert(function.name().to_string(), i);
    }

    eprintln!("{:#?}", resolved_globals);

    let mut functions = Vec::with_capacity(resolved_globals.len());
    let mut globals = Vec::with_capacity(resolved_globals.len());

    for function in ast.walk_expr_fns() {
        let mut bytecode = Bytecode::new();
        let mut locals = HashMap::new();

        for statement in function.expr_fn().body.stmts.iter() {
            match &statement.body {
                Expression::BoolValue(bool, span) => {
                    bytecode.constants.push(Value::Bool(*bool));
                    bytecode
                        .instructions
                        .push(Instruction::LoadConst(bytecode.constants.len() - 1));
                }
                Expression::NumericValue(num, span) => {
                    bytecode
                        .constants
                        .push(Value::Int(num.parse::<i64>().unwrap()));
                    bytecode
                        .instructions
                        .push(Instruction::LoadConst(bytecode.constants.len() - 1));
                }
                Expression::StringValue(string, span) => todo!(),
                Expression::RawStringValue(raw_string) => todo!(),
                Expression::Identifier(identifier) => todo!(),
                Expression::Array(expressions, span) => todo!(),
                Expression::Map(items, span) => todo!(),
                Expression::JinjaExpressionValue(jinja_expression, span) => todo!(),
                Expression::Lambda(arguments_list, expression_block, span) => todo!(),
                Expression::App(app) => {
                    eprintln!("{app:#?}");
                    bytecode
                        .instructions
                        .push(Instruction::LoadGlobal(resolved_globals[app.name.name()]));
                    bytecode
                        .instructions
                        .push(Instruction::Call(app.args.len()));
                }
                Expression::ClassConstructor(class_constructor, span) => todo!(),
                Expression::ExprBlock(expression_block, span) => todo!(),
                Expression::If(expression, expression1, expression2, span) => todo!(),
                Expression::ForLoop {
                    identifier,
                    iterator,
                    body,
                    span,
                } => todo!(),
            }

            let local_index = locals.len() + 1;
            bytecode
                .instructions
                .push(Instruction::StoreVar(local_index));

            locals.insert(statement.identifier.to_string(), local_index);
        }

        match &*function.expr_fn().body.expr {
            Expression::Identifier(identifier) => {
                bytecode
                    .instructions
                    .push(Instruction::LoadVar(locals[identifier.name()]));
                bytecode.instructions.push(Instruction::Return);
            }
            Expression::BoolValue(bool, span) => {
                bytecode.constants.push(Value::Bool(*bool));
                bytecode
                    .instructions
                    .push(Instruction::LoadConst(bytecode.constants.len() - 1));
                bytecode.instructions.push(Instruction::Return);
            }
            Expression::NumericValue(num, span) => {
                bytecode
                    .constants
                    .push(Value::Int(num.parse::<i64>().unwrap()));
                bytecode
                    .instructions
                    .push(Instruction::LoadConst(bytecode.constants.len() - 1));
                bytecode.instructions.push(Instruction::Return);
            }
            Expression::StringValue(_, span) => todo!(),
            Expression::RawStringValue(raw_string) => todo!(),
            Expression::Array(expressions, span) => todo!(),
            Expression::Map(items, span) => todo!(),
            Expression::JinjaExpressionValue(jinja_expression, span) => todo!(),
            Expression::Lambda(arguments_list, expression_block, span) => todo!(),
            Expression::App(app) => todo!(),
            Expression::ClassConstructor(class_constructor, span) => todo!(),
            Expression::ExprBlock(expression_block, span) => todo!(),
            Expression::If(expression, expression1, expression2, span) => todo!(),
            Expression::ForLoop {
                identifier,
                iterator,
                body,
                span,
            } => todo!(),
        }

        eprintln!("==== {} ====", function.name());
        eprintln!("{}", bytecode);

        functions.push(Function {
            name: function.name().to_string(),
            arity: function.args().args.len(),
            bytecode,
            kind: FunctionKind::Exec,
        });

        globals.push(Value::Object(Object::Function(
            functions.last().unwrap().clone(),
        )));
    }

    Ok((functions, globals))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn ast(source: &str) -> anyhow::Result<ParserDatabase> {
        let path = std::path::PathBuf::from("test.baml");
        let source_file = internal_baml_diagnostics::SourceFile::from((path.clone(), source));

        let validated_schema = internal_baml_core::validate(&path, vec![source_file]);

        if validated_schema.diagnostics.has_errors() {
            return Err(anyhow::anyhow!(
                "{}",
                validated_schema.diagnostics.to_pretty_string()
            ));
        }

        Ok(validated_schema.db)
    }

    #[test]
    fn test_compile() -> anyhow::Result<()> {
        let ast = ast("
            fn main() -> int {
                let a = two();
                a
            }

            fn two() -> int {
                2
            }
        ")?;

        let (functions, globals) = compile(ast)?;

        Ok(())
    }
}
