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
use internal_baml_core::ast::{self, Expression, WithName};
use internal_baml_parser_database::ParserDatabase;

/// Baml compiler.
///
/// This struct compiles a single AST function into bytecode.
///
/// **IMPORTANT**: The compiler DOES NOT validate anything, AST must already be
/// validated before calling this otherwise it will issue incorrect bytecode,
/// the VM will break and the universe will collapse.
struct Compiler<'g> {
    /// Resolved global variables.
    ///
    /// Maps the name of the global variable to its index in the globals pool.
    globals: &'g HashMap<String, usize>,

    /// Resolved local variables.
    ///
    /// Maps the name of the variable to its final index in the eval stack.
    locals: HashMap<String, usize>,

    /// Bytecode to generate.
    bytecode: Bytecode,
}

impl<'g> Compiler<'g> {
    /// Create a new compiler.
    pub fn new(globals: &'g HashMap<String, usize>) -> Self {
        Self {
            globals,
            locals: HashMap::new(),
            bytecode: Bytecode::new(),
        }
    }

    /// Compile AST function into VM function.
    pub fn compile(mut self, function: &ast::ExprFn) -> anyhow::Result<Function> {
        // Resolve parameters.
        for param in &function.args.args {
            self.locals
                .insert(param.0.to_string(), self.locals.len() + 1);
        }

        // Resolve rest of locals.
        for statement in function.body.stmts.iter() {
            self.compile_expression(&statement.body);

            let local_index = self.locals.len() + 1;

            // We don't need to emit this because when the expression is
            // executed and leaves the value on top of the stack, that index in
            // the stack will be the index of the local variable. It's already
            // "stored".

            // self.emit(Instruction::StoreVar(local_index));

            self.locals
                .insert(statement.identifier.to_string(), local_index);
        }

        // Compile the return expression.
        self.compile_expression(&function.body.expr);

        // Pop off the stack.
        self.emit(Instruction::Return);

        Ok(Function {
            name: function.name.to_string(),
            arity: function.args.args.len(),
            bytecode: self.bytecode,
            kind: FunctionKind::Exec,

            local_var_names: {
                let mut names = Vec::with_capacity(self.locals.len() + 1);
                names.push(format!("<fn {}>", function.name));
                names.resize_with(names.capacity(), String::new);

                for (name, index) in &self.locals {
                    names[*index] = name.to_string();
                }

                names
            },
        })
    }

    /// Emits a single instruction and returns the index of the instruction.
    ///
    /// The return value is useful when we want to modify an instruction that
    /// we've already ommited. Take a look at how we compile if statements in
    /// the [`Self::compile_expression`] function.
    fn emit(&mut self, instruction: Instruction) -> usize {
        self.bytecode.instructions.push(instruction);
        self.bytecode.instructions.len() - 1
    }

    /// Adds a new constant to the constants pool and returns its index.
    fn add_constant(&mut self, value: Value) -> usize {
        self.bytecode.constants.push(value);
        self.bytecode.constants.len() - 1
    }

    /// Patches a jump instruction to point to the correct destination.
    ///
    /// When we first emit a jump instruction, we do not know what offset to use
    /// because we don't know how many instructions the block we want to jump
    /// over will emit. In order to solve that, we emit the jump instruction
    /// with a placeholder offset (like 0), then we compile the jump target,
    /// and finally we call this function passing the index of the jump
    /// instruction to adjust the offset and make it point to the end of the
    /// target block.
    fn patch_jump(&mut self, instruction_ptr: usize) {
        let destination = self.bytecode.instructions.len();

        match &mut self.bytecode.instructions[instruction_ptr] {
            Instruction::Jump(offset) | Instruction::JumpIfFalse(offset) => {
                *offset = (destination - instruction_ptr) as isize;
            }

            // This should never run. Don't call this function with anything
            // that is not a jump instruction.
            // TODO: Just error out and report some "Internal Error" instead of
            // panicking and breaking the program.
            _ => unreachable!(
                "expected jump instruction at index {instruction_ptr}, but got {:?}",
                self.bytecode.instructions[instruction_ptr]
            ),
        }
    }

    /// Generate bytecode for an expression.
    ///
    /// # Dev notes
    ///
    /// Be cautious with "abstractions" here. It's better to be explicit so that
    /// we can see exactly what instructions are being emitted in each scenario.
    /// We should not create `emit_some_crazy_stuff` functions unless they can
    /// be reused many times.
    fn compile_expression(&mut self, expression: &Expression) {
        match expression {
            // Constants.
            Expression::BoolValue(bool, _span) => {
                let index = self.add_constant(Value::Bool(*bool));
                self.emit(Instruction::LoadConst(index));
            }

            Expression::NumericValue(num, _span) => {
                let index = self.add_constant(Value::Int(num.parse::<i64>().unwrap()));
                self.emit(Instruction::LoadConst(index));
            }

            Expression::StringValue(string, _span) => todo!(),

            Expression::RawStringValue(raw_string) => todo!(),

            // Variables.
            Expression::Identifier(identifier) => {
                self.emit(Instruction::LoadVar(self.locals[identifier.name()]));
            }

            // Compound objects.
            Expression::Array(expressions, span) => todo!(),

            Expression::Map(items, span) => todo!(),

            Expression::ClassConstructor(class_constructor, span) => todo!(),

            // Functions.
            Expression::Lambda(arguments_list, expression_block, span) => todo!(),

            Expression::App(app) => {
                // Push the function onto the stack.
                self.emit(Instruction::LoadGlobal(self.globals[app.name.name()]));

                // Push the arguments onto the stack.
                for arg in &app.args {
                    self.compile_expression(arg);
                }

                // Call the function.
                self.emit(Instruction::Call(app.args.len()));
            }

            Expression::JinjaExpressionValue(jinja_expression, span) => todo!(),

            Expression::ExprBlock(expression_block, span) => todo!(),

            // Branching.
            Expression::If(condition, r#if, r#else, _span) => {
                // First, compile the condition. This will leave the end result
                // of the condition on top of the stack.
                self.compile_expression(condition);

                // Skip the `if { ... }` branch when condition is false. We'll
                // patch this offset later when we know how many instructions to
                // jump over, so we'll store a reference to this instruction.
                let skip_if = self.emit(Instruction::JumpIfFalse(0));

                // In case we execute the `if { ... }` branch, prepend a POP to
                // discard the condition value, we don't need it anymore.
                self.emit(Instruction::Pop);

                // Compile the `if { ... }` branch.
                self.compile_expression(r#if);

                // Now skip the potential `else { ... }` branch. We'll patch the
                // jump later.
                let skip_else = self.emit(Instruction::Jump(0));

                // We now know where the `if { ... }` branch ends so we can
                // patch the JUMP_IF_FALSE instruction above.
                self.patch_jump(skip_if);

                // This is either the start of the `else { ... }` branch or the
                // start of whatever code we have after an `if { ... }` branch
                // without an `else` statement. Either way, we still have to
                // discard the condition value.
                self.emit(Instruction::Pop);

                // Compile the `else { ... }` branch if it exists.
                if let Some(r#else) = r#else {
                    self.compile_expression(r#else);
                }

                // Patch the skip else jump.
                self.patch_jump(skip_else);
            }

            Expression::ForLoop {
                identifier,
                iterator,
                body,
                span,
            } => todo!(),
        }
    }
}

/// Compile a Baml AST into bytecode.
///
/// For now, these creates a couple data structures that the VM needs in order
/// to run. First, it returns the object pool or object arena, which contains
/// all the functions, then it returns the globals pool (functions are basically
/// global "variables").
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
        let function = Compiler::new(&resolved_globals).compile(function.expr_fn())?;

        // Add the function to the globals and objects pools.
        globals.push(Value::Object(objects.len()));
        objects.push(Object::Function(function));
    }

    Ok((objects, globals))
}

/// For tests.
///
/// We reuse this in the VM.
pub fn ast(source: &str) -> anyhow::Result<ParserDatabase> {
    let path = std::path::PathBuf::from("test.baml");
    let source_file = internal_baml_diagnostics::SourceFile::from((path.clone(), source));

    let validated_schema = internal_baml_core::validate(&path, vec![source_file]);

    if validated_schema.diagnostics.has_errors() {
        let errors = validated_schema.diagnostics.to_pretty_string();
        anyhow::bail!("{}", errors);
    }

    Ok(validated_schema.db)
}

#[cfg(test)]
mod tests {
    use super::*;

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
