use std::collections::HashMap;

use baml_vm::{Bytecode, Function, FunctionKind, Instruction, Object, Value};
use internal_baml_core::ast::WithName;
use internal_baml_parser_database::{ast::Expression, ParserDatabase};

/// Generate bytecode for an expression.
fn compile_expression(
    expression: &Expression,
    locals: &HashMap<String, usize>,
    resolved_globals: &HashMap<String, usize>,
    bytecode: &mut Bytecode,
) {
    match expression {
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
        Expression::Identifier(identifier) => {
            bytecode
                .instructions
                .push(Instruction::LoadVar(locals[identifier.name()]));
        }
        Expression::Array(expressions, span) => todo!(),
        Expression::Map(items, span) => todo!(),
        Expression::JinjaExpressionValue(jinja_expression, span) => todo!(),
        Expression::Lambda(arguments_list, expression_block, span) => todo!(),
        Expression::App(app) => {
            eprintln!("{app:#?}");
            bytecode
                .instructions
                .push(Instruction::LoadGlobal(resolved_globals[app.name.name()]));

            for arg in &app.args {
                compile_expression(arg, locals, resolved_globals, bytecode);
            }

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
}

/// Generate bytecode.
pub fn compile(ast: ParserDatabase) -> anyhow::Result<(Vec<Object>, Vec<Value>)> {
    // eprintln!("{:#?}", ast.ast);

    let mut resolved_globals = HashMap::new();

    // TODO: Probably we can use Top::Id to map these to VM globals.
    for (i, function) in ast.walk_expr_fns().enumerate() {
        resolved_globals.insert(function.name().to_string(), i);
    }

    eprintln!("{:#?}", resolved_globals);

    let mut globals = Vec::with_capacity(resolved_globals.len());
    let mut objects = Vec::with_capacity(resolved_globals.len());

    for function in ast.walk_expr_fns() {
        let mut bytecode = Bytecode::new();
        let mut locals = HashMap::new();

        for param in &function.args().args {
            locals.insert(param.0.to_string(), locals.len() + 1);
        }

        for statement in function.expr_fn().body.stmts.iter() {
            compile_expression(&statement.body, &locals, &resolved_globals, &mut bytecode);

            let local_index = locals.len() + 1;

            // We don't need to emit this because when the expression is
            // executed and leaves the value on top of the stack, that index in
            // the stack will be the index of the local variable. It's already
            // "stored".

            // bytecode
            //     .instructions
            //     .push(Instruction::StoreVar(local_index));

            locals.insert(statement.identifier.to_string(), local_index);
        }

        eprintln!("{:#?}", locals);

        compile_expression(
            &function.expr_fn().body.expr,
            &locals,
            &resolved_globals,
            &mut bytecode,
        );
        bytecode.instructions.push(Instruction::Return);

        eprintln!("==== {} ====", function.name());
        eprintln!("{}", bytecode);

        let function = Function {
            name: function.name().to_string(),
            arity: function.args().args.len(),
            bytecode,
            kind: FunctionKind::Exec,
        };

        globals.push(Value::Object(objects.len()));
        objects.push(Object::Function(function));
    }

    Ok((objects, globals))
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
            fn two() -> int {
                2
            }

            fn main() -> int {
                let a = two();
                a
            }
        ")?;

        let (objects, globals) = compile(ast)?;

        let Object::Function(main) = &objects[0] else {
            return Err(anyhow::anyhow!("Main function not found"));
        };

        let Object::Function(two) = &objects[1] else {
            return Err(anyhow::anyhow!("Two function not found"));
        };

        assert_eq!(
            main.bytecode.instructions,
            vec![Instruction::LoadConst(0), Instruction::Return]
        );

        assert_eq!(
            two.bytecode.instructions,
            vec![
                Instruction::LoadGlobal(0),
                Instruction::Call(0),
                Instruction::StoreVar(1),
                Instruction::LoadVar(1),
                Instruction::Return,
            ]
        );

        Ok(())
    }
}
