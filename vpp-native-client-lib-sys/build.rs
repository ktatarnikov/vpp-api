use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is expected"));
    let workspace_dir_path = std::env::var("CARGO_WORKSPACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_workspace_root());
    let workspace_dir = workspace_dir_path
        .to_str()
        .expect("CARGO_WORKSPACE_DIR is invalid");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("architecture is not defined with CARGO_CFG_TARGET_ARCH variable");

    // Pick the right library for the platform
    match target_os.as_str() {
        "linux" => (),
        _ => panic!("unsupported target OS"),
    };

    let cargo_toml_path = PathBuf::from(workspace_dir.to_owned()).join("Cargo.toml");
    let cargo_toml = std::fs::read_to_string(cargo_toml_path).expect("Cannot read Cargo.toml");
    let value: toml::Value = toml::from_str(&cargo_toml).expect("invalid Cargo.toml format");

    // Navigate to `[workspace.metadata.build-config]`
    let build_cfg = value
        .get("workspace")
        .and_then(|ws| ws.get("metadata"))
        .and_then(|m| m.get("build-config"))
        .expect("workspace.metadata.build-config not found");

    let Some(vpp_version) = build_cfg.get("vpp-version").and_then(|v| v.as_str()) else {
        panic!("vpp-version is not defined in the workspace Cargo.toml")
    };

    let lib_path = format!("{}/lib/{}", vpp_version, arch);
    let library_filename = format!("libvppapiclient.so.{}", vpp_version);
    let dst_library_filename = "libvppapiclient.so";
    println!("cargo:info=defined lib_path '{}'", lib_path);
    println!("cargo:info=defined library_filename '{}'", library_filename);
    
    let src = manifest_dir.join(lib_path).join(library_filename);
    
    // Copy the library into OUT_DIR so Cargo treats it as a build artifact
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is expected"));
    let dst = out_dir.join(dst_library_filename);
    fs::copy(&src, &dst).expect("failed to copy bundled library");
    // Tell rustc where to find it
    println!("cargo:rustc-link-search=native={}", out_dir.display());

    // Tell rustc which library to link
    println!("cargo:rustc-link-lib=vppapiclient");

    // Ensure rebuilds when library changes
    println!("cargo:rerun-if-changed={}", src.display());
}

fn find_workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    loop {
        let manifest = dir.join("Cargo.toml");

        if manifest.exists() {
            let text = fs::read_to_string(&manifest).expect("failed reading Cargo.toml");

            // Treat this Cargo.toml as the workspace root if it declares a workspace
            if text.contains("[workspace]") {
                return dir;
            }
        }

        if !dir.pop() {
            panic!("Could not find workspace root");
        }
    }
}
