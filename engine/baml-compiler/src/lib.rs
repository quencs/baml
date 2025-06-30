//! Baml bytecode compiler.
//!
//! This crate is concerned with generating VM bytecode from a Baml AST. For now
//! it is pretty straightforward to go from AST to bytecode, but in the future
//! we might need more tree transformations to generate our bytecode.
//! Specifically, read about how Rust handles [HIR] (High Level IR) and [MIR]
//! (Mid Level IR):
//!
//! [HIR]: https://rustc-dev-guide.rust-lang.org/hir.html
//! [MIR]: https://rustc-dev-guide.rust-lang.org/mir/index.html

use std::collections::HashMap;

use baml_vm::{Bytecode, Function, FunctionKind, Instruction, Object, Value};
use internal_baml_core::ast::WithName;
use internal_baml_parser_database::{ast::Expression, ParserDatabase};

/// Generate bytecode for an expression.
///
/// # Dev notes
///
/// Be cautious with "abstractions" here. It's better to be explicit so that
/// we can see exactly what instructions are being emitted in each scenario. We
/// should not create `emit_some_crazy_stuff` functions unless they can be
/// reused many times.
fn compile_expression(
    expression: &Expression,
    resolved_locals: &HashMap<String, usize>,
    resolved_globals: &HashMap<String, usize>,
    bytecode: &mut Bytecode,
) {
    match expression {
        // Constants.
        Expression::BoolValue(bool, _span) => {
            bytecode.constants.push(Value::Bool(*bool));
            bytecode
                .instructions
                .push(Instruction::LoadConst(bytecode.constants.len() - 1));
        }
        Expression::NumericValue(num, _span) => {
            bytecode
                .constants
                .push(Value::Int(num.parse::<i64>().unwrap()));
            bytecode
                .instructions
                .push(Instruction::LoadConst(bytecode.constants.len() - 1));
        }
        Expression::StringValue(string, _span) => todo!(),
        Expression::RawStringValue(raw_string) => todo!(),

        // Variables.
        Expression::Identifier(identifier) => {
            bytecode
                .instructions
                .push(Instruction::LoadVar(resolved_locals[identifier.name()]));
        }

        // Compound objects.
        Expression::Array(expressions, span) => todo!(),
        Expression::Map(items, span) => todo!(),
        Expression::ClassConstructor(class_constructor, span) => todo!(),

        // Functions.
        Expression::Lambda(arguments_list, expression_block, span) => todo!(),
        Expression::App(app) => {
            bytecode
                .instructions
                .push(Instruction::LoadGlobal(resolved_globals[app.name.name()]));

            for arg in &app.args {
                compile_expression(arg, resolved_locals, resolved_globals, bytecode);
            }

            // Call the function.
            bytecode
                .instructions
                .push(Instruction::Call(app.args.len()));
        }
        Expression::JinjaExpressionValue(jinja_expression, span) => todo!(),
        Expression::ExprBlock(expression_block, span) => todo!(),
        Expression::If(condition, r#if, r#else, _span) => {
            // First, compile the condition. This will leave the end result of
            // the condition on top of the stack.
            compile_expression(condition, resolved_locals, resolved_globals, bytecode);

            // Skip the `if { ... }` branch when condition is false. We'll patch
            // this offset later when we know how many instructions to jump
            // over, so we'll store a reference to this instruction.
            bytecode.instructions.push(Instruction::JumpIfFalse(0));
            let skip_if = bytecode.instructions.len() - 1;

            // In case we execute the `if { ... }` branch, prepend a POP to
            // discard the condition value, we don't need it anymore.
            bytecode.instructions.push(Instruction::Pop);

            // Compile the `if { ... }` branch.
            compile_expression(r#if, resolved_locals, resolved_globals, bytecode);

            // Now skip the potential `else { ... }` branch. We'll patch the
            // jump later.
            bytecode.instructions.push(Instruction::Jump(0));
            let skip_else = bytecode.instructions.len() - 1;

            // We now know where the `if { ... }` branch ends so we can patch
            // the JUMP_IF_FALSE instruction above.
            let offset = (bytecode.instructions.len() - skip_if) as isize;
            bytecode.instructions[skip_if] = Instruction::JumpIfFalse(offset);

            // This is either the start of the `else { ... }` branch or the
            // start of whatever code we have after an `if { ... }` branch
            // without an `else` statement. Either way, we still have to discard
            // the condition value.
            bytecode.instructions.push(Instruction::Pop);

            // Compile the `else { ... }` branch if it exists.
            if let Some(r#else) = r#else {
                compile_expression(r#else, resolved_locals, resolved_globals, bytecode);
            }

            // Patch the skip else jump.
            let offset = (bytecode.instructions.len() - skip_else) as isize;
            bytecode.instructions[skip_else] = Instruction::Jump(offset);
        }
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

    let mut globals = Vec::with_capacity(resolved_globals.len());
    let mut objects = Vec::with_capacity(resolved_globals.len());

    for function in ast.walk_expr_fns() {
        let mut bytecode = Bytecode::new();
        let mut resolved_locals = HashMap::new();

        // Resolve parameters.
        for param in &function.args().args {
            resolved_locals.insert(param.0.to_string(), resolved_locals.len() + 1);
        }

        // Resolve rest of locals.
        for statement in function.expr_fn().body.stmts.iter() {
            compile_expression(
                &statement.body,
                &resolved_locals,
                &resolved_globals,
                &mut bytecode,
            );

            let local_index = resolved_locals.len() + 1;

            // We don't need to emit this because when the expression is
            // executed and leaves the value on top of the stack, that index in
            // the stack will be the index of the local variable. It's already
            // "stored".

            // bytecode
            //     .instructions
            //     .push(Instruction::StoreVar(local_index));

            resolved_locals.insert(statement.identifier.to_string(), local_index);
        }

        // Compile the return expression.
        compile_expression(
            &function.expr_fn().body.expr,
            &resolved_locals,
            &resolved_globals,
            &mut bytecode,
        );

        // Pop off the stack.
        bytecode.instructions.push(Instruction::Return);

        let function = Function {
            name: function.name().to_string(),
            arity: function.args().args.len(),
            bytecode,
            kind: FunctionKind::Exec,
        };

        // Add the function to the globals and objects pools.
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
    fn call_function() -> anyhow::Result<()> {
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
                Instruction::LoadVar(1),
                Instruction::Return,
            ]
        );

        Ok(())
    }

    #[test]
    fn if_else_statement() -> anyhow::Result<()> {
        let ast = ast("
            fn main(b: bool) -> int {
                if b { 1 } else { 2 }
            }
        ")?;

        let (objects, globals) = compile(ast)?;

        let Object::Function(main) = &objects[0] else {
            return Err(anyhow::anyhow!("Main function not found"));
        };

        assert_eq!(
            main.bytecode.instructions,
            vec![
                Instruction::LoadVar(1),
                Instruction::JumpIfFalse(4),
                Instruction::Pop,
                Instruction::LoadConst(0),
                Instruction::Jump(3),
                Instruction::Pop,
                Instruction::LoadConst(1),
                Instruction::Return,
            ]
        );

        Ok(())
    }
}
