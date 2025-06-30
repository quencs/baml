//! VM integ tests.
//!
//! These tests need the compiler crate to go from source to bytecode, that's
//! why they're not placed in the source vm module.

use baml_vm::{Frame, Value, Vm};
use internal_baml_parser_database::ParserDatabase;

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
fn function_call_without_parameters() -> anyhow::Result<()> {
    let ast = ast("
        fn two() -> int {
            let v = 2;
            v
        }

        fn main() -> int {
            let v = two();
            v
        }
    ")?;

    let (objects, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(1)],
        objects,
        globals,
    };

    vm.frames.push(Frame {
        function: 1,
        instruction_ptr: 0,
        locals_offset: 0,
    });

    let expected = Value::Int(2);
    let result = vm.exec().unwrap();

    assert!(
        matches!(&result, expected),
        "Expected {expected:?}, got {result:?}"
    );

    Ok(())
}

#[test]
fn function_call_with_parameters() -> anyhow::Result<()> {
    let ast = ast("
        fn one_of(a: int, b: int) -> int {
            a
        }

        fn main() -> int {
            let v = one_of(1, 2);
            v
        }
    ")?;

    let (objects, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(1)],
        objects,
        globals,
    };

    vm.frames.push(Frame {
        function: 1,
        instruction_ptr: 0,
        locals_offset: 0,
    });

    let result = vm.exec().unwrap();

    assert!(
        matches!(&result, Value::Int(1)),
        "Expected {expected:?}, got {result:?}",
        expected = Value::Int(1),
    );

    Ok(())
}

#[test]
fn exec_if_branch() -> anyhow::Result<()> {
    let ast = ast("
        fn run_if(b: bool) -> int {
            if b { 1 } else { 2 }
        }

        fn main() -> int {
            let a = run_if(true);
            a
        }
    ")?;

    let (objects, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(1)],
        objects,
        globals,
    };

    vm.frames.push(Frame {
        function: 1,
        instruction_ptr: 0,
        locals_offset: 0,
    });

    let result = vm.exec().unwrap();

    assert!(
        matches!(&result, Value::Int(1)),
        "Expected {expected:?}, got {result:?}",
        expected = Value::Int(1),
    );

    Ok(())
}

#[test]
fn exec_else_branch() -> anyhow::Result<()> {
    let ast = ast("
        fn run_if(b: bool) -> int {
            if b { 1 } else { 2 }
        }

        fn main() -> int {
            let a = run_if(false);
            a
        }
    ")?;

    let (objects, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(1)],
        objects,
        globals,
    };

    vm.frames.push(Frame {
        function: 1,
        instruction_ptr: 0,
        locals_offset: 0,
    });

    let result = vm.exec().unwrap();

    assert!(
        matches!(&result, Value::Int(2)),
        "Expected {expected:?}, got {result:?}",
        expected = Value::Int(2),
    );

    Ok(())
}
