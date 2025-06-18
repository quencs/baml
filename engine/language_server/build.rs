/// This build script exists as a fix to guarantee
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

    // Install dependencies
    run_command(
        Command::new("pnpm")
            .current_dir(&typescript_dir)
            .arg("install"),
        "pnpm install",
    )
    .expect("Failed to execute pnpm install command");

    run_command(
        Command::new("pnpm")
            .current_dir(&web_panel_dir)
            .arg("build"),
        "pnpm build web-panel",
    )
    .expect("Failed to execute pnpm build command");

    // Double check we correctly built the frontend
    let dist_path = env::var("BAML_WEB_PANEL_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| web_panel_dir.join("dist"));

    if !dist_path.exists() {
        panic!(
            "Web panel dist directory not found at {}. Please ensure the path is correct or set BAML_WEB_PANEL_DIST environment variable.",
            dist_path.display()
        );
    }
}
