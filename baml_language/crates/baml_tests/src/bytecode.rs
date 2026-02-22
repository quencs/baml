//! Shared test utilities for compiling BAML source into bytecode programs.

use std::path::Path;

pub use baml_project::ProjectDatabase;
use bex_vm_types::Program as VmProgram;

/// Set up a test database from BAML source code.
///
/// Creates a `ProjectDatabase`, sets a project root, and adds the source as
/// `test.baml`. Builtins are loaded automatically via `set_project_root()`.
pub fn setup_test_db(source: &str) -> ProjectDatabase {
    let mut db = ProjectDatabase::new();
    db.set_project_root(Path::new("."));
    db.add_file("test.baml", source);
    db
}

/// Assert that a `ProjectDatabase` has no diagnostic errors.
///
/// Panics with a descriptive message if any error-level diagnostics are found.
/// Warnings and info-level diagnostics are ignored.
#[track_caller]
pub fn assert_no_diagnostic_errors(db: &ProjectDatabase) {
    use baml_compiler_diagnostics::Severity;

    let project = db.get_project().expect("project must be set");
    let all_files = db.get_source_files();
    let diagnostics = baml_project::collect_diagnostics(db, project, &all_files);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    if !errors.is_empty() {
        let mut msg = String::from("Compilation produced diagnostic errors:\n");
        for (i, err) in errors.iter().enumerate() {
            msg.push_str(&format!("  {}. [{}] {}\n", i + 1, err.code(), err.message));
        }
        panic!("{msg}");
    }
}

/// Compile BAML source code into a VM program.
///
/// Also checks for diagnostic errors and panics if any are found.
pub fn compile_source(source: &str) -> VmProgram {
    let db = setup_test_db(source);
    assert_no_diagnostic_errors(&db);

    let project = db.get_project().expect("project should be set");
    let all_files = project.files(&db).clone();
    baml_compiler_emit::compile_files(&db, &all_files, baml_compiler_emit::OptLevel::One)
        .expect("compile_files should succeed for valid test source")
}
