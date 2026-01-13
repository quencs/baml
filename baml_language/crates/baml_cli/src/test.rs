//! Test command - run BAML tests from the CLI.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use baml_compiler_hir::{ItemId, Test, file_items};
use baml_db::{Setter, baml_workspace::Project};
use baml_executor::{
    BamlExecutor, BamlMap, BamlValue,
    context::{DynamicBamlContext, PerCallContext, SharedCallContext},
};
use baml_project::ProjectDatabase;
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct TestArgs {
    /// Path to baml_src directory or a specific .baml file
    #[arg(long, default_value = ".")]
    pub from: PathBuf,

    /// Filter tests by function name (e.g., "MyFunction" or "MyFunction::TestName")
    #[arg(short = 'i', long = "include")]
    pub include: Vec<String>,

    /// Exclude tests matching pattern
    #[arg(short = 'x', long = "exclude")]
    pub exclude: Vec<String>,
}

/// A discovered test case.
struct TestCase {
    function_name: String,
    test_name: String,
    args: BamlMap,
}

impl TestArgs {
    pub fn run(&self) -> Result<crate::ExitCode> {
        // Create a tokio runtime for async HTTP execution
        let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

        runtime.block_on(self.run_async())
    }

    async fn run_async(&self) -> Result<crate::ExitCode> {
        // Resolve the baml_src path
        let baml_src = self.resolve_baml_src()?;
        println!("{} {}", "Loading BAML from:".blue(), baml_src.display());

        // Discover and load BAML files
        let baml_files = discover_baml_files(&baml_src)?;
        if baml_files.is_empty() {
            println!("{}", "No .baml files found".yellow());
            return Ok(crate::ExitCode::NoTestsRun);
        }
        println!("{} {} files", "Found".blue(), baml_files.len());

        // Create database and load files
        let (db, project, source_files) = load_baml_project(&baml_files)?;

        // Create executor
        let executor = BamlExecutor::with_project(db.clone(), project);

        // Discover tests
        let tests = discover_tests(&db, project, &source_files, &self.include, &self.exclude)?;
        if tests.is_empty() {
            println!("{}", "No tests found matching filters".yellow());
            return Ok(crate::ExitCode::NoTestsRun);
        }
        println!("{} {} tests", "Running".blue(), tests.len());
        println!();

        // Run tests
        let mut passed = 0;
        let mut failed = 0;
        let total_start = Instant::now();

        // Get env vars for execution
        let env_vars: HashMap<String, String> = std::env::vars().collect();

        for test in &tests {
            let test_start = Instant::now();
            print!(
                "  {} {}::{} ... ",
                "test".dimmed(),
                test.function_name,
                test.test_name
            );

            let result = run_single_test(&executor, test, &env_vars).await;
            let duration = test_start.elapsed();

            match result {
                Ok(output) => {
                    passed += 1;
                    println!("{} ({:.2?})", "ok".green(), duration);
                    // Optionally show output for debugging
                    if std::env::var("BAML_TEST_VERBOSE").is_ok() {
                        println!("    output: {:?}", output);
                    }
                }
                Err(e) => {
                    failed += 1;
                    println!("{} ({:.2?})", "FAILED".red(), duration);
                    // Show the full error chain for debugging
                    println!("    {}: {}", "error".red(), e);
                    for cause in e.chain().skip(1) {
                        println!("    {}: {}", "caused by".red(), cause);
                    }
                }
            }
        }

        let total_duration = total_start.elapsed();
        println!();
        println!(
            "test result: {}. {} passed; {} failed; finished in {:.2?}",
            if failed == 0 {
                "ok".green()
            } else {
                "FAILED".red()
            },
            passed,
            failed,
            total_duration
        );

        if failed > 0 {
            Ok(crate::ExitCode::TestFailure)
        } else {
            Ok(crate::ExitCode::Success)
        }
    }

    fn resolve_baml_src(&self) -> Result<PathBuf> {
        let path = self
            .from
            .canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", self.from.display()))?;

        // If it's a file, use it directly
        if path.is_file() {
            return Ok(path);
        }

        // If it's already baml_src, use it
        if path.file_name().map(|n| n == "baml_src").unwrap_or(false) {
            return Ok(path);
        }

        // Check if there's a baml_src subdirectory
        let baml_src = path.join("baml_src");
        if baml_src.is_dir() {
            return Ok(baml_src);
        }

        // Otherwise use the path as-is
        Ok(path)
    }
}

fn discover_baml_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.clone()]);
    }

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "baml").unwrap_or(false) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn load_baml_project(
    files: &[PathBuf],
) -> Result<(ProjectDatabase, Project, Vec<baml_db::SourceFile>)> {
    let mut db = ProjectDatabase::new();

    // Determine project root (parent of first file or the file's directory)
    let project_root = files
        .first()
        .and_then(|f| f.parent())
        .unwrap_or(std::path::Path::new("."));

    let project = db.set_project_root(project_root);

    // Load all files
    let mut source_files = Vec::new();
    for file_path in files {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let source_file = db.add_file(file_path.to_string_lossy().as_ref(), &content);
        source_files.push(source_file);
    }

    // Wire up project files
    project.set_files(&mut db).to(source_files.clone());

    Ok((db, project, source_files))
}

fn discover_tests(
    db: &ProjectDatabase,
    _project: Project,
    source_files: &[baml_db::SourceFile],
    include: &[String],
    exclude: &[String],
) -> Result<Vec<TestCase>> {
    let mut tests = Vec::new();

    // Iterate over all source files
    for &source_file in source_files {
        let items = file_items(db, source_file);

        for item_id in items.items(db) {
            if let ItemId::Test(test_loc) = item_id {
                // Get the test item tree
                let item_tree = baml_compiler_hir::file_item_tree(db, source_file);
                let test: &Test = &item_tree[test_loc.id(db)];

                let test_name = test.name.to_string();

                // Each test can reference multiple functions
                for func_ref in &test.function_refs {
                    let function_name = func_ref.to_string();
                    let full_name = format!("{}::{}", function_name, test_name);

                    // Apply filters
                    if exclude.iter().any(|pattern| {
                        matches_pattern(pattern, &full_name, &function_name, &test_name)
                    }) {
                        continue;
                    }

                    if !include.is_empty()
                        && !include.iter().any(|pattern| {
                            matches_pattern(pattern, &full_name, &function_name, &test_name)
                        })
                    {
                        continue;
                    }

                    // For MVP, use empty args - test args parsing will need more work
                    // TODO: Parse test args from syntax tree
                    let args = BamlMap::new();

                    tests.push(TestCase {
                        function_name,
                        test_name: test_name.clone(),
                        args,
                    });
                }
            }
        }
    }

    Ok(tests)
}

fn matches_pattern(pattern: &str, full_name: &str, func_name: &str, test_name: &str) -> bool {
    // Support patterns like "FuncName", "FuncName::TestName", "::TestName", "*::*"
    if pattern.contains("::") {
        let parts: Vec<&str> = pattern.splitn(2, "::").collect();
        let func_pattern = parts[0];
        let test_pattern = parts.get(1).unwrap_or(&"*");

        let func_matches =
            func_pattern.is_empty() || func_pattern == "*" || func_name.contains(func_pattern);
        let test_matches =
            test_pattern.is_empty() || *test_pattern == "*" || test_name.contains(test_pattern);

        func_matches && test_matches
    } else {
        // Just a function name pattern
        func_name.contains(pattern) || full_name.contains(pattern)
    }
}

async fn run_single_test(
    executor: &BamlExecutor,
    test: &TestCase,
    env_vars: &HashMap<String, String>,
) -> Result<BamlValue> {
    // Prepare the function
    let prepared = executor
        .prepare_function(&test.function_name, test.args.clone())
        .with_context(|| format!("Failed to prepare function {}", test.function_name))?;

    // Create contexts
    let shared_ctx = SharedCallContext::default();
    let dynamic_ctx = DynamicBamlContext::default();
    let mut per_call_ctx = PerCallContext::new();

    // Add env vars to context
    for (k, v) in env_vars {
        per_call_ctx.env_vars.insert(k.clone(), v.clone());
    }

    // Call the function
    let result = executor
        .call_function(&prepared, &shared_ctx, &dynamic_ctx, &per_call_ctx)
        .await
        .with_context(|| format!("Failed to execute function {}", test.function_name))?;

    Ok(result.value)
}
