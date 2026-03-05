//! Integration tests: compile BAML → convert TIR types → simplify → assert Display.
//!
//! Test cases are defined in `type_simplification_tests.md`.

use std::collections::HashMap;

use baml_compiler_tir::{class_field_types, find_recursive_aliases, type_aliases};
use baml_project::ProjectDatabase;
use baml_type::{convert_tir_ty, simplify, Name};

/// A single test case parsed from the markdown file.
struct TestCase {
    name: String,
    baml_source: String,
    target_class: String,
    target_field: String,
    expected_materialized: String,
}

/// Parse the markdown test file into test cases.
fn parse_test_cases(markdown: &str) -> Vec<TestCase> {
    let mut cases = Vec::new();
    let mut lines = markdown.lines().peekable();

    while let Some(line) = lines.next() {
        // Look for ## test_name headers (but not # section headers or ### sub-headers)
        let trimmed = line.trim();
        if !trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            continue;
        }

        let name = trimmed.strip_prefix("## ").unwrap().trim().to_string();

        // Find ```baml block
        let mut baml_source = String::new();
        let mut in_baml = false;
        while let Some(line) = lines.next() {
            let t = line.trim();
            if t == "```baml" {
                in_baml = true;
                continue;
            }
            if in_baml {
                if t == "```" {
                    break;
                }
                if !baml_source.is_empty() {
                    baml_source.push('\n');
                }
                baml_source.push_str(line);
            }
        }

        // Find ### target: `Class.field`
        let mut target_class = String::new();
        let mut target_field = String::new();
        while let Some(line) = lines.next() {
            let t = line.trim();
            if t.starts_with("### target:") {
                // Extract `Class.field` from the backtick-quoted string
                if let Some(start) = t.find('`') {
                    if let Some(end) = t[start + 1..].find('`') {
                        let target = &t[start + 1..start + 1 + end];
                        if let Some(dot) = target.find('.') {
                            target_class = target[..dot].to_string();
                            target_field = target[dot + 1..].to_string();
                        }
                    }
                }
                break;
            }
        }

        // Find - Materialized: `type`
        let mut expected_materialized = String::new();
        while let Some(line) = lines.next() {
            let t = line.trim();
            if t.starts_with("- Materialized:") {
                if let Some(start) = t.find('`') {
                    if let Some(end) = t[start + 1..].find('`') {
                        expected_materialized = t[start + 1..start + 1 + end].to_string();
                    }
                }
                break;
            }
            // Stop if we hit the next section divider
            if t == "---" {
                break;
            }
        }

        if !target_class.is_empty() && !expected_materialized.is_empty() {
            cases.push(TestCase {
                name,
                baml_source,
                target_class,
                target_field,
                expected_materialized,
            });
        }
    }

    cases
}

/// Compile BAML source, look up a class field's type, convert and simplify it.
fn compile_and_simplify(
    baml_source: &str,
    class_name: &str,
    field_name: &str,
) -> Result<String, String> {
    let mut db = ProjectDatabase::new();
    let root = db.set_project_root(std::path::Path::new("."));

    db.add_file("test.baml", baml_source);

    let class_fields_map = class_field_types(&db, root);
    let classes: &HashMap<Name, HashMap<Name, baml_compiler_tir::Ty>> =
        class_fields_map.classes(&db);

    let type_aliases_map = type_aliases(&db, root).aliases(&db).clone();
    let recursive_aliases = find_recursive_aliases(&type_aliases_map);

    let class_key = Name::new(class_name);
    let field_key = Name::new(field_name);

    let fields = classes
        .get(&class_key)
        .ok_or_else(|| format!("class '{}' not found", class_name))?;

    let tir_ty = fields
        .get(&field_key)
        .ok_or_else(|| format!("field '{}.{}' not found", class_name, field_name))?;

    let converted = convert_tir_ty(tir_ty, &type_aliases_map, &recursive_aliases)?;
    let simplified = simplify(&converted);
    Ok(simplified.to_string())
}

#[test]
fn type_simplification_tests() {
    let markdown = include_str!("type_simplification_tests.md");
    let cases = parse_test_cases(markdown);
    assert!(!cases.is_empty(), "no test cases parsed from markdown");

    let mut failures = Vec::new();
    for case in &cases {
        match compile_and_simplify(&case.baml_source, &case.target_class, &case.target_field) {
            Ok(actual) => {
                if actual != case.expected_materialized {
                    failures.push(format!(
                        "  {}: expected `{}`, got `{}`",
                        case.name, case.expected_materialized, actual,
                    ));
                }
            }
            Err(e) => {
                failures.push(format!("  {}: compile error: {}", case.name, e));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} type simplification test(s) failed:\n{}\n",
            failures.len(),
            failures.join("\n"),
        );
    }
}
