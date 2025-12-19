use std::{path::PathBuf, sync::Arc};

pub mod version_check;

pub use dir_writer::GeneratorArgs;
use dir_writer::{IntermediateRepr, LanguageFeatures};
use indexmap::IndexMap;
use internal_baml_core::configuration::GeneratorOutputType;

pub struct GenerateOutput {
    pub client_type: GeneratorOutputType,
    /// Relative path to the output directory (output_dir in the generator)
    pub output_dir_shorthand: PathBuf,
    /// The absolute path that the generated baml client was written to
    pub output_dir_full: PathBuf,
    pub files: IndexMap<PathBuf, String>,
}

pub fn generate_sdk(
    ir: Arc<IntermediateRepr>,
    gen: &GeneratorArgs,
) -> Result<IndexMap<PathBuf, String>, anyhow::Error> {
    let res = match gen.client_type {
        GeneratorOutputType::Go => {
            use generators_go::GoLanguageFeatures;
            let features = GoLanguageFeatures;
            features.generate_sdk(ir, gen)?
        }
        GeneratorOutputType::PythonPydantic | GeneratorOutputType::PythonPydanticV1 => {
            use generators_python::PyLanguageFeatures;
            let features = PyLanguageFeatures;
            features.generate_sdk(ir, gen)?
        }
        GeneratorOutputType::OpenApi => {
            use generators_openapi::OpenApiLanguageFeatures;
            let features = OpenApiLanguageFeatures;
            features.generate_sdk(ir, gen)?
        }
        GeneratorOutputType::Typescript | GeneratorOutputType::TypescriptReact => {
            use generators_typescript::TsLanguageFeatures;
            let features = TsLanguageFeatures;
            features.generate_sdk(ir, gen)?
        }
        GeneratorOutputType::RubySorbet => {
            use generators_ruby::RbLanguageFeatures;
            let features = RbLanguageFeatures::default();
            features.generate_sdk(ir, gen)?
        }
    };

    // Run on_generate commands
    #[cfg(not(target_arch = "wasm32"))]
    {
        if matches!(gen.client_type, GeneratorOutputType::OpenApi) && gen.on_generate.is_empty() {
            // TODO: we should auto-suggest a command for the user to run here
            baml_log::warn!("No on_generate commands were provided for OpenAPI generator - skipping OpenAPI client generation");
        }
        for cmd in gen.on_generate.iter() {
            baml_log::info!("Running {:?} in {}", cmd, gen.output_dir().display());

            let output = run_shell_command(cmd, &gen.output_dir())?;

            // log::info!("on_generate command finished");
            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let error_msg = format!(
                    "on_generate command finished with {}: {}\nStdout:\n{}\nStderr:\n{}",
                    match output.status.code() {
                        Some(code) => format!("exit code {code}"),
                        None => "no exit code".to_string(),
                    },
                    cmd,
                    stdout,
                    stderr
                );
                return Err(anyhow::anyhow!("{}", error_msg));
            }
        }

        Ok(res)
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(res)
    }
}

/// Runs a shell command in a cross-platform manner.
///
/// On Unix/macOS: Uses `sh -c` directly.
/// On Windows: Tries `sh -c` first (for Git Bash/WSL users), then falls back to `cmd.exe /C`.
#[cfg(not(target_arch = "wasm32"))]
fn run_shell_command(
    cmd: &str,
    working_dir: &std::path::Path,
) -> Result<std::process::Output, anyhow::Error> {
    use anyhow::Context;

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(working_dir)
            .output()
            .context(format!("Failed to run on_generate command: {cmd}"))
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, try `sh` first (available via Git Bash, WSL, MSYS2, etc.)
        let sh_result = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(working_dir)
            .output();

        match sh_result {
            Ok(output) => Ok(output),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // `sh` not found, try `cmd.exe /C` as fallback
                baml_log::info!(
                    "'sh' not found on Windows, falling back to 'cmd.exe /C' for on_generate command"
                );

                let cmd_result = std::process::Command::new("cmd.exe")
                    .arg("/C")
                    .arg(cmd)
                    .current_dir(working_dir)
                    .output();

                match cmd_result {
                    Ok(output) => Ok(output),
                    Err(cmd_err) => {
                        Err(anyhow::anyhow!(
                            "Failed to run on_generate command: {cmd}\n\n\
                            Could not find 'sh' (tried sh -c): {e}\n\
                            Fallback to 'cmd.exe /C' also failed: {cmd_err}\n\n\
                            To fix this on Windows, you can:\n\
                            1. Install Git for Windows (includes Git Bash with sh): https://git-scm.com/downloads\n\
                            2. Install WSL (Windows Subsystem for Linux): https://learn.microsoft.com/en-us/windows/wsl/install\n\
                            3. Rewrite your on_generate command to use Windows-native syntax (cmd.exe compatible)"
                        ))
                    }
                }
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to run on_generate command with sh: {cmd}\nError: {e}"
            )),
        }
    }
}
