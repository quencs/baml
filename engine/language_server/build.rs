/// This build script exists as a workaround to guarantee
/// that the web-panel dist directory is built before the
/// language server is built. As the web-panel is a embedded
/// in the language server.
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn run_command(cmd: &mut Command, name: &str) -> std::io::Result<()> {
    println!("cargo:warning=Running {}...", name);
    let output = cmd.output()?;

    // Print stdout and stderr
    if !output.stdout.is_empty() {
        println!("cargo:warning={} stdout:", name);
        println!("cargo:warning={}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        println!("cargo:warning={} stderr:", name);
        println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        panic!("{} failed with status: {}", name, output.status);
    }
    Ok(())
}

fn main() {
    // Get the manifest directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = PathBuf::from(&manifest_dir).join("../..");
    let typescript_dir = workspace_root.join("typescript");
    let web_panel_dir = typescript_dir.join("vscode-ext/packages/web-panel");

    println!(
        "cargo:warning=Typescript directory: {}",
        typescript_dir.display()
    );

    // pnpm install
    run_command(
        Command::new("pnpm")
            .current_dir(&typescript_dir)
            .arg("install"),
        "pnpm install",
    )
    .expect("Failed to execute pnpm install command");

    // Run tsc and vite build in web-panel directory to get the dist directory
    // for embedding in the language server
    run_command(
        Command::new("npx")
            .current_dir(&web_panel_dir)
            .args(["tsc", "--noEmit"]),
        "tsc type check",
    )
    .expect("Failed to execute tsc type check");

    run_command(
        Command::new("npx")
            .current_dir(&web_panel_dir)
            .args(["vite", "build"]),
        "vite build",
    )
    .expect("Failed to execute vite build");

    // Try to find the dist directory
    let dist_path = web_panel_dir.join("dist");

    // Check if the directory exists
    if !dist_path.exists() {
        panic!(
            "Web panel dist directory not found at {}. Please ensure the path is correct or set BAML_WEB_PANEL_DIST environment variable.",
            dist_path.display()
        );
    }

    // Set the environment variable for the build
    println!(
        "cargo:rustc-env=BAML_WEB_PANEL_DIST={}",
        dist_path.display()
    );
}
