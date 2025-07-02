//! VM integ tests.
//!
//! These tests need the compiler crate to go from source to bytecode, that's
//! why they're not placed in the source vm module.

use baml_compiler::ast;
use baml_vm::{Frame, Object, Value, Vm};

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

#[test]
fn array_constructor() -> anyhow::Result<()> {
    let ast = ast("
        fn main() -> int[] {
            let a = [1, 2, 3];
            a
        }
    ")?;

    let (objects, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(0)],
        objects,
        globals,
    };

    vm.frames.push(Frame {
        function: 0,
        instruction_ptr: 0,
        locals_offset: 0,
    });

    let result = vm.exec().unwrap();

    assert!(
        matches!(&result, Value::Object(1)),
        "Expected {expected:?}, got {result:?}",
        expected = Value::Object(1),
    );

    let Object::Array(array) = &vm.objects[1] else {
        panic!("Expected Array, got {:?}", vm.objects[1]);
    };

    // Assert self.objects[1] is an array with 3 elements.
    assert_eq!(array.len(), 3);
    assert!(
        matches!(array[0], Value::Int(1)),
        "Expected Int(1), got {:?}",
        array[0]
    );
    assert!(
        matches!(array[1], Value::Int(2)),
        "Expected Int(2), got {:?}",
        array[1]
    );
    assert!(
        matches!(array[2], Value::Int(3)),
        "Expected Int(3), got {:?}",
        array[2]
    );

    Ok(())
}
