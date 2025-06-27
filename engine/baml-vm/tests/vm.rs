use baml_vm::{Frame, Object, Value, Vm};
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
fn test_vm() -> anyhow::Result<()> {
    let ast = ast("
        fn main() -> int {
            let a = two();
            a
        }

        fn two() -> int {
            2
        }
    ")?;

    let (mut functions, globals) = baml_compiler::compile(ast)?;

    let mut vm = Vm {
        frames: vec![],
        stack: vec![Value::Object(Object::Function(functions[0].clone()))],
        objects: functions
            .iter()
            .map(|f| Object::Function(f.clone()))
            .collect(),
        globals,
    };

    vm.frames.push(Frame {
        function: functions.swap_remove(0),
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
