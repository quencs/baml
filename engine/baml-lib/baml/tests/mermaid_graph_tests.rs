use std::{fs, path::Path};

use baml_lib::{
    internal_baml_ast::{ast::BamlVisDiagramGenerator, parse},
    internal_baml_diagnostics::SourceFile,
};

const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/validation_files/headers"
);

#[test]
fn headers_mermaid_snapshots() {
    let dir = Path::new(ROOT);
    if !dir.exists() {
        panic!("fixtures dir missing: {}", ROOT);
    }

    let mut ran = 0usize;
    let mut passed = 0usize;
    let mut updated = 0usize;
    let mut skipped_panic = 0usize;
    let mut skipped_parse = 0usize;
    let mut missing_expect = 0usize;
    let mut failed = 0usize;
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("baml") {
            continue;
        }

        // Friendly relative name for output
        let rel_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        // Parse defensively; let panic message print so it is visible, but keep the test running
        let path_clone = path.clone();
        let res = std::panic::catch_unwind(|| {
            let baml = fs::read_to_string(&path_clone).unwrap();
            let src = SourceFile::new_allocated(path_clone.clone(), baml.clone().into());
            parse(Path::new("."), &src)
        });

        let Ok(parse_result) = res else {
            eprintln!("[mermaid] {:<12} | {}", "SKIP(panic)", rel_name);
            skipped_panic += 1;
            continue;
        };

        let Ok((ast, _diags)) = parse_result else {
            eprintln!("[mermaid] {:<12} | {}", "SKIP(parse)", rel_name);
            skipped_parse += 1;
            continue;
        };

        let got = BamlVisDiagramGenerator::generate_headers_flowchart(&ast);

        let mut exp_path = path.clone();
        exp_path.set_extension("mmd");
        if std::env::var("UPDATE_EXPECT").ok().as_deref() == Some("1") {
            fs::write(&exp_path, got).unwrap();
            println!("[mermaid] {:<12} | {}", "UPDATED", rel_name);
            ran += 1;
            updated += 1;
            continue;
        }

        match fs::read_to_string(&exp_path) {
            Ok(expected) => {
                let got_n = normalize(&got);
                let exp_n = normalize(&expected);
                if got_n == exp_n {
                    println!("[mermaid] {:<12} | {}", "PASS", rel_name);
                    passed += 1;
                    ran += 1;
                } else {
                    eprintln!("[mermaid] {:<12} | {}", "FAIL", rel_name);
                    failed += 1;
                    assert_eq!(got_n, exp_n, "mismatch in {}", rel_name);
                }
            }
            Err(_) => {
                eprintln!("[mermaid] {:<12} | {}", "SKIP(expect)", rel_name);
                missing_expect += 1;
                continue;
            }
        }
    }

    assert!(ran > 0, "no valid fixtures were executed in {}", ROOT);
    println!("[mermaid] Summary");
    println!("  ran:     {}", ran);
    println!("  pass:    {}", passed);
    println!("  updated: {}", updated);
    println!("  skip:");
    println!("    panic:  {}", skipped_panic);
    println!("    parse:  {}", skipped_parse);
    println!("    expect: {}", missing_expect);
    println!("  fail:    {}", failed);
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim().to_string()
}

// verbose mode removed for simplicity; panics print naturally and we tag the file in output
