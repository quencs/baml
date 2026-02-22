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

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::Arc,
};

use anyhow::Context;
use bex_engine::{BexEngine, FunctionCallContextBuilder};
use bex_vm::{BexVm, VmExecState};
use indexmap::IndexMap;
use insta::{assert_snapshot, with_settings};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
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

#[derive(Debug)]
struct OutputCase {
    baml: String,
    entry: String,
    opt: SnapshotOptLevel,
    bytecode: String,
    notifications: Option<Vec<String>>,
    result: String,
}

fn strip_insta_frontmatter(contents: &str) -> &str {
    if let Some(rest) = contents.strip_prefix("---\n") {
        if let Some(idx) = rest.find("\n---\n") {
            // Skip `<metadata>\n---\n`.
            return &rest[idx + "\n---\n".len()..];
        }
    } else if let Some(rest) = contents.strip_prefix("---\r\n")
        && let Some(idx) = rest.find("\r\n---\r\n")
    {
        // Skip `<metadata>\r\n---\r\n`.
        return &rest[idx + "\r\n---\r\n".len()..];
    }
    contents
}

fn parse_cases(snapshot_contents: &str) -> anyhow::Result<IndexMap<String, InputCase>> {
    let body = strip_insta_frontmatter(snapshot_contents).trim();
    serde_yaml::from_str(body).context("failed to parse suite YAML from snapshot body")
}

fn compile_with_opt(source: &str, opt: SnapshotOptLevel) -> anyhow::Result<bex_vm_types::Program> {
    let db = crate::bytecode::setup_test_db(source);
    {
        use baml_compiler_diagnostics::Severity;

        let project = db.get_project().context("project should be set")?;
        let all_files = db.get_source_files();
        let diagnostics = baml_project::collect_diagnostics(&db, project, &all_files);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .collect();
        if !errors.is_empty() {
            let mut msg = String::from("Compilation produced diagnostic errors:\n");
            for (i, err) in errors.iter().enumerate() {
                msg.push_str(&format!("  {}. [{}] {}\n", i + 1, err.code(), err.message));
            }
            anyhow::bail!(msg);
        }
    }

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
    let engine = BexEngine::new(program, Arc::new(sys_types::SysOps::all_unsupported()))
        .context("failed to construct BexEngine")?;

    let context = FunctionCallContextBuilder::new(sys_types::CallId::next()).build();
    let result = runtime
        .block_on(engine.call_function(entry, Vec::new(), context))
        .with_context(|| format!("engine call_function failed for '{entry}'"))?;

    Ok(format!("BexExternalValue::{result:?}"))
}

// NOTE: We intentionally render VM values directly here instead of reusing
// `BexExternalValue` conversion for now:
// - watch-mode snapshots need the raw `BexVm` loop to capture `VmExecState::Notify`.
// - existing deep-copy conversion used by the engine is not cycle-safe for arbitrary graphs.
// - watch snapshots need deterministic object shape output (with cycle refs), not lossy fallback.
#[derive(Default)]
struct VmValueRenderState {
    object_ids: HashMap<bex_vm_types::HeapPtr, usize>,
    visiting: HashSet<bex_vm_types::HeapPtr>,
}

impl VmValueRenderState {
    fn object_id(&mut self, ptr: bex_vm_types::HeapPtr) -> usize {
        let next = self.object_ids.len();
        *self.object_ids.entry(ptr).or_insert(next)
    }
}

fn render_vm_value(
    vm: &BexVm,
    value: &bex_vm_types::Value,
    state: &mut VmValueRenderState,
) -> String {
    match value {
        bex_vm_types::Value::Null => "Null".to_string(),
        bex_vm_types::Value::Int(v) => format!("Int({v})"),
        bex_vm_types::Value::Float(v) => format!("Float({v})"),
        bex_vm_types::Value::Bool(v) => format!("Bool({v})"),
        bex_vm_types::Value::Object(ptr) => render_vm_object(vm, *ptr, state),
    }
}

fn render_vm_object(
    vm: &BexVm,
    ptr: bex_vm_types::HeapPtr,
    state: &mut VmValueRenderState,
) -> String {
    let object_id = state.object_id(ptr);
    if !state.visiting.insert(ptr) {
        return format!("Ref(#{object_id})");
    }

    let rendered = match vm.get_object(ptr) {
        bex_vm_types::Object::Function(function) => format!("Function({})", function.name),
        bex_vm_types::Object::Class(class) => format!("Class({})", class.name),
        bex_vm_types::Object::Enum(enm) => format!("Enum({})", enm.name),
        bex_vm_types::Object::String(value) => format!("String({value:?})"),
        bex_vm_types::Object::Array(array) => {
            let items = array
                .iter()
                .map(|value| render_vm_value(vm, value, state))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Array([{items}])")
        }
        bex_vm_types::Object::Map(map) => {
            let entries = map
                .iter()
                .map(|(key, value)| format!("{key:?}: {}", render_vm_value(vm, value, state)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Map({{{entries}}})")
        }
        bex_vm_types::Object::Instance(instance) => {
            let (class_name, field_names) = match vm.get_object(instance.class) {
                bex_vm_types::Object::Class(class) => (
                    class.name.clone(),
                    class
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>(),
                ),
                _ => ("<invalid-class>".to_string(), Vec::new()),
            };

            let fields = instance
                .fields
                .iter()
                .enumerate()
                .map(|(idx, value)| {
                    let name = field_names
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| format!("field_{idx}"));
                    format!("{name}: {}", render_vm_value(vm, value, state))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Instance({class_name} {{ {fields} }})")
        }
        bex_vm_types::Object::Variant(variant) => {
            let (enum_name, variant_name) = match vm.get_object(variant.enm) {
                bex_vm_types::Object::Enum(enm) => (
                    enm.name.clone(),
                    enm.variants
                        .get(variant.index)
                        .map(|value| value.name.clone())
                        .unwrap_or_else(|| format!("variant_{}", variant.index)),
                ),
                _ => (
                    "<invalid-enum>".to_string(),
                    format!("variant_{}", variant.index),
                ),
            };
            format!("Variant({enum_name}::{variant_name})")
        }
        bex_vm_types::Object::Future(future) => match future {
            bex_vm_types::types::Future::Pending(pending) => {
                let args = pending
                    .args
                    .iter()
                    .map(|value| render_vm_value(vm, value, state))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Future(Pending(op: {}, args: [{args}]))", pending.operation)
            }
            bex_vm_types::types::Future::Ready(value) => {
                format!("Future(Ready({}))", render_vm_value(vm, value, state))
            }
        },
        bex_vm_types::Object::Resource(resource) => format!("Resource({resource})"),
        bex_vm_types::Object::Media(media) => format!("Media({media:?})"),
        bex_vm_types::Object::PromptAst(prompt_ast) => format!("PromptAst({prompt_ast:?})"),
        bex_vm_types::Object::Collector(_) => "Collector".to_string(),
        bex_vm_types::Object::Type(ty) => format!("Type({ty})"),
        #[cfg(feature = "heap_debug")]
        bex_vm_types::Object::Sentinel(kind) => format!("Sentinel({kind:?})"),
    };

    state.visiting.remove(&ptr);
    rendered
}

struct WatchRunOutput {
    notifications: Vec<String>,
    result: String,
}

fn run_watch_mode(program: bex_vm_types::Program, entry: &str) -> anyhow::Result<WatchRunOutput> {
    let function_index = program
        .function_index(entry)
        .ok_or_else(|| anyhow::anyhow!("function '{entry}' not found"))?;

    let mut vm = BexVm::from_program(program).context("failed to create BexVm")?;
    let function_ptr = vm.heap.compile_time_ptr(function_index);
    vm.set_entry_point(function_ptr, &[]);

    let mut notifications: Vec<String> = Vec::new();
    let mut render_state = VmValueRenderState::default();

    let final_state = loop {
        let result = vm.exec().context("vm.exec failed")?;
        if matches!(result, VmExecState::SpanNotify(_)) {
            continue;
        }
        match result {
            VmExecState::Notify(notification) => {
                notifications.push(format!("{notification:?}"));
            }
            VmExecState::Complete(value) => {
                break format!(
                    "Complete({})",
                    render_vm_value(&vm, &value, &mut render_state)
                );
            }
            VmExecState::Await(handle) => {
                let _ = handle;
                break "Await".to_string();
            }
            VmExecState::ScheduleFuture(handle) => {
                let _ = handle;
                break "ScheduleFuture".to_string();
            }
            VmExecState::SpanNotify(_) => {
                // handled above
            }
        }
    };

    Ok(WatchRunOutput {
        notifications,
        result: final_state,
    })
}

fn format_error_with_debug(err: &anyhow::Error) -> String {
    format!("ERROR: {err:#}\nRUST_ERR_DEBUG: Err({err:?})")
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn format_panic_with_debug(payload: Box<dyn Any + Send>) -> String {
    let message = panic_payload_to_string(payload.as_ref());
    format!("ERROR: panic during execution: {message}\nRUST_ERR_DEBUG: Panic({message:?})")
}

struct CaseExecutionOutput {
    notifications: Option<Vec<String>>,
    result: String,
}

impl CaseExecutionOutput {
    fn plain(result: String) -> Self {
        Self {
            notifications: None,
            result,
        }
    }
}

fn run_plain_case_with_panic_capture(
    f: impl FnOnce() -> anyhow::Result<String>,
) -> CaseExecutionOutput {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => CaseExecutionOutput::plain(value),
        Ok(Err(err)) => CaseExecutionOutput::plain(format_error_with_debug(&err)),
        Err(payload) => CaseExecutionOutput::plain(format_panic_with_debug(payload)),
    }
}

fn run_watch_case_with_panic_capture(
    f: impl FnOnce() -> anyhow::Result<WatchRunOutput>,
) -> CaseExecutionOutput {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => CaseExecutionOutput {
            notifications: Some(value.notifications),
            result: value.result,
        },
        Ok(Err(err)) => CaseExecutionOutput::plain(format_error_with_debug(&err)),
        Err(payload) => CaseExecutionOutput::plain(format_panic_with_debug(payload)),
    }
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

fn render_string_flow_list(out: &mut String, key: &str, values: &[String]) -> anyhow::Result<()> {
    if values.is_empty() {
        writeln!(out, "    {key}: []").expect("write to string should not fail");
        return Ok(());
    }

    writeln!(out, "    {key}: [").expect("write to string should not fail");
    for value in values {
        let rendered = format!("{value:?}");
        writeln!(out, "        {rendered},").expect("write to string should not fail");
    }
    writeln!(out, "    ]").expect("write to string should not fail");
    Ok(())
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
        if let Some(notifications) = &case.notifications {
            render_string_flow_list(&mut out, "notifications", notifications)?;
            out.push('\n');
        }
        if case.result.contains('\n') {
            render_block(&mut out, "result", &case.result);
        } else {
            let result = render_inline_yaml_string(&case.result)?;
            writeln!(out, "    result: {result}").expect("write to string should not fail");
        }
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
        let baml = case.baml;
        let entry = case.entry;

        let (bytecode, execution) = match compile_with_opt(&baml, case.opt)
            .with_context(|| format!("compile failed for case '{test_name}'"))
        {
            Ok(program) => {
                let bytecode = match disassemble_program(&program)
                    .with_context(|| format!("disassembly failed for case '{test_name}'"))
                {
                    Ok(text) => text,
                    Err(err) => format_error_with_debug(&err),
                };

                let watch_execution = run_watch_case_with_panic_capture(|| {
                    run_watch_mode(program.clone(), &entry)
                        .with_context(|| format!("watch execution failed for case '{test_name}'"))
                });

                let execution = if watch_execution
                    .notifications
                    .as_ref()
                    .is_some_and(|notifications| !notifications.is_empty())
                {
                    watch_execution
                } else {
                    run_plain_case_with_panic_capture(|| {
                        run_with_engine(&runtime, program, &entry)
                            .with_context(|| format!("execution failed for case '{test_name}'"))
                    })
                };

                (bytecode, execution)
            }
            Err(err) => {
                let formatted = format_error_with_debug(&err);
                (
                    formatted.clone(),
                    CaseExecutionOutput::plain(format!(
                        "ERROR: compile step failed; no runtime result\n{formatted}"
                    )),
                )
            }
        };

        output_cases.insert(
            test_name,
            OutputCase {
                baml: baml.trim_end_matches('\n').to_string(),
                entry,
                opt: case.opt,
                bytecode,
                notifications: execution.notifications,
                result: execution.result,
            },
        );
    }

    let rendered = render_cases(&output_cases)?;

    with_settings!({snapshot_path => snapshot_path, omit_expression => true, prepend_module_to_snapshot => false}, {
        assert_snapshot!(suite_name, rendered);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_cases;

    #[test]
    fn parse_cases_strips_lf_frontmatter() {
        let snapshot = r#"---
source: crates/baml_tests/src/bytecode_snapshot.rs
---
case:
    baml: |
        function main() -> int {
            1
        }
"#;

        let cases = parse_cases(snapshot).expect("frontmatter should be stripped");
        assert!(cases.contains_key("case"));
    }

    #[test]
    fn parse_cases_strips_crlf_frontmatter() {
        let snapshot = "---\r\nsource: crates/baml_tests/src/bytecode_snapshot.rs\r\n---\r\ncase:\r\n    baml: |\r\n        function main() -> int {\r\n            1\r\n        }\r\n";

        let cases = parse_cases(snapshot).expect("frontmatter should be stripped");
        assert!(cases.contains_key("case"));
    }
}
