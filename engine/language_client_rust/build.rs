use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = match env::var("OUT_DIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };

    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.clone());

    let library_filename = match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "libbaml_cffi.dylib",
        Ok("linux") => "libbaml_cffi.so",
        _ => return,
    };

    let default_library_path = profile_dir.join(library_filename);

    println!(
        "cargo:rustc-env=BAML_CFFI_DEFAULT_LIBRARY_PATH={}",
        default_library_path.display()
    );
    println!(
        "cargo:rustc-env=BAML_CFFI_PROFILE_DIR={}",
        profile_dir.display()
    );

    if let Ok(target_triple) = env::var("TARGET") {
        println!("cargo:rustc-env=BAML_CFFI_TARGET_TRIPLE={target_triple}");
    }
    if let Ok(profile_name) = env::var("PROFILE") {
        println!("cargo:rustc-env=BAML_CFFI_PROFILE_NAME={profile_name}");
    }

    println!("cargo:rerun-if-env-changed=BAML_LIBRARY_PATH");
    println!("cargo:rerun-if-env-changed=BAML_CACHE_DIR");
    println!("cargo:rerun-if-env-changed=BAML_LIBRARY_DISABLE_DOWNLOAD");
}
