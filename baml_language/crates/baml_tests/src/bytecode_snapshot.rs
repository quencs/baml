//! Suite-level bytecode + engine snapshot runner.
//!
//! Snapshot files are top-level YAML maps:
//!
//! ```text
//! test_name:
//!   baml: |-
//!     function main() -> int { 42 }
//!   entry: main      # optional, defaults to "main"
//!   opt: one         # optional, one|zero, defaults to "one"
//!   bytecode: ...    # auto-generated
//!   result: ...      # auto-generated
//! ```

use std::{fmt::Write, sync::Arc};

use anyhow::Context;
use bex_engine::{BexEngine, FunctionCallContextBuilder};
use indexmap::IndexMap;
use insta::{assert_snapshot, with_settings};
use serde::{Deserialize, Serialize};
use sys_native::SysOpsExt;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum SnapshotOptLevel {
    Zero,
    #[default]
    One,
}

impl SnapshotOptLevel {
    fn as_emit_opt_level(self) -> baml_compiler_emit::OptLevel {
        match self {
            SnapshotOptLevel::Zero => baml_compiler_emit::OptLevel::Zero,
            SnapshotOptLevel::One => baml_compiler_emit::OptLevel::One,
        }
    }

    fn as_yaml_str(self) -> &'static str {
        match self {
            SnapshotOptLevel::Zero => "zero",
            SnapshotOptLevel::One => "one",
        }
    }
}

fn default_entry() -> String {
    "main".to_string()
}

fn is_default_entry(entry: &str) -> bool {
    entry == "main"
}

fn is_default_opt(opt: &SnapshotOptLevel) -> bool {
    matches!(opt, SnapshotOptLevel::One)
}

#[derive(Debug, Deserialize)]
struct InputCase {
    baml: String,
    #[serde(default = "default_entry")]
    entry: String,
    #[serde(default)]
    opt: SnapshotOptLevel,
    #[allow(dead_code)]
    #[serde(default)]
    bytecode: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    result: Option<String>,
}

#[derive(Debug, Serialize)]
struct OutputCase {
    baml: String,
    #[serde(default = "default_entry", skip_serializing_if = "is_default_entry")]
    entry: String,
    #[serde(default, skip_serializing_if = "is_default_opt")]
    opt: SnapshotOptLevel,
    bytecode: String,
    result: String,
}

fn strip_insta_frontmatter(contents: &str) -> &str {
    if let Some(rest) = contents.strip_prefix("---\n")
        && let Some(idx) = rest.find("\n---\n")
    {
        // Skip `<metadata>\n---\n`.
        return &rest[idx + "\n---\n".len()..];
    }
    contents
}

fn parse_cases(snapshot_contents: &str) -> anyhow::Result<IndexMap<String, InputCase>> {
    let body = strip_insta_frontmatter(snapshot_contents).trim();
    serde_yaml::from_str(body).context("failed to parse suite YAML from snapshot body")
}

fn compile_with_opt(source: &str, opt: SnapshotOptLevel) -> anyhow::Result<bex_vm_types::Program> {
    let db = crate::bytecode::setup_test_db(source);
    crate::bytecode::assert_no_diagnostic_errors(&db);

    let project = db.get_project().context("project should be set")?;
    let all_files = project.files(&db).clone();
    baml_compiler_emit::compile_files(&db, &all_files, opt.as_emit_opt_level())
        .context("compile_files failed")
}

fn disassemble_program(program: &bex_vm_types::Program) -> anyhow::Result<String> {
    let mut names: Vec<_> = program
        .function_indices
        .keys()
        .filter(|name| !name.starts_with("baml."))
        .collect();
    names.sort();

    let functions: Vec<(String, &bex_vm_types::types::Function)> = names
        .iter()
        .map(|name| {
            let idx = *program.function_indices.get(*name).ok_or_else(|| {
                anyhow::anyhow!("function '{name}' missing from function_indices")
            })?;
            let object = program
                .objects
                .get(idx)
                .ok_or_else(|| anyhow::anyhow!("object index {idx} not found for '{name}'"))?;
            let bex_vm_types::Object::Function(function) = object else {
                anyhow::bail!("object index {idx} for '{name}' is not a Function");
            };
            Ok(((*name).clone(), function.as_ref()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(bex_vm::debug::display_program(
        &functions,
        bex_vm::debug::BytecodeFormat::Textual,
    ))
}

fn run_with_engine(
    runtime: &tokio::runtime::Runtime,
    program: bex_vm_types::Program,
    entry: &str,
) -> anyhow::Result<String> {
    let engine = BexEngine::new(program, Arc::new(sys_types::SysOps::native()))
        .context("failed to construct BexEngine")?;

    let context = FunctionCallContextBuilder::new(sys_types::CallId::next()).build();
    let result = runtime
        .block_on(engine.call_function(entry, Vec::new(), context))
        .with_context(|| format!("engine call_function failed for '{entry}'"))?;

    Ok(format!("BexExternalValue::{result:?}"))
}

fn render_inline_yaml_string(value: &str) -> anyhow::Result<String> {
    let mut rendered = serde_yaml::to_string(value).context("failed to serialize scalar")?;
    if let Some(stripped) = rendered.strip_prefix("---\n") {
        rendered = stripped.to_string();
    }
    Ok(rendered.trim_end().to_string())
}

fn render_block(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "    {key}: |");
    for line in value.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            let _ = writeln!(out, "        {line}");
        }
    }
}

fn render_cases(cases: &IndexMap<String, OutputCase>) -> anyhow::Result<String> {
    let mut out = String::new();

    for (idx, (test_name, case)) in cases.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }

        writeln!(out, "{test_name}:").expect("write to string should not fail");
        render_block(&mut out, "baml", &case.baml);

        if !is_default_entry(&case.entry) {
            out.push('\n');
            let entry = render_inline_yaml_string(&case.entry)?;
            writeln!(out, "    entry: {entry}").expect("write to string should not fail");
        }

        if !is_default_opt(&case.opt) {
            out.push('\n');
            writeln!(out, "    opt: {}", case.opt.as_yaml_str())
                .expect("write to string should not fail");
        }

        out.push('\n');
        render_block(&mut out, "bytecode", &case.bytecode);
        out.push('\n');
        let result = render_inline_yaml_string(&case.result)?;
        writeln!(out, "    result: {result}").expect("write to string should not fail");
    }

    Ok(out)
}

/// Recompute all test cases from a suite snapshot, then snapshot the canonical YAML.
pub fn assert_suite_snapshot(
    suite_name: &str,
    snapshot_path: &str,
    snapshot_contents: &str,
) -> anyhow::Result<()> {
    let input_cases = parse_cases(snapshot_contents)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    let mut output_cases: IndexMap<String, OutputCase> = IndexMap::new();

    for (test_name, case) in input_cases {
        let program = compile_with_opt(&case.baml, case.opt)
            .with_context(|| format!("compile failed for case '{test_name}'"))?;
        let bytecode = disassemble_program(&program)
            .with_context(|| format!("disassembly failed for case '{test_name}'"))?;
        let result = run_with_engine(&runtime, program, &case.entry)
            .with_context(|| format!("execution failed for case '{test_name}'"))?;

        output_cases.insert(
            test_name,
            OutputCase {
                baml: case.baml.trim_end_matches('\n').to_string(),
                entry: case.entry,
                opt: case.opt,
                bytecode,
                result,
            },
        );
    }

    let rendered = render_cases(&output_cases)?;

    with_settings!({snapshot_path => snapshot_path, omit_expression => true, prepend_module_to_snapshot => false}, {
        assert_snapshot!(suite_name, rendered);
    });

    Ok(())
}
