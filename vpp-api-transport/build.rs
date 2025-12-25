extern crate bindgen;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let workspace_dir_path = std::env::var("CARGO_WORKSPACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_workspace_root());
    let workspace_dir = workspace_dir_path
        .to_str()
        .expect("CARGO_WORKSPACE_DIR is invalid");

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("architecture is not defined with CARGO_CFG_TARGET_ARCH variable");

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

    let vpp_lib_dir = format!("{}/vpp-release/{}/lib/{}", workspace_dir, vpp_version, arch);
    let library_filename = format!("libvppapiclient.so.{}", vpp_version);
    println!("cargo:info=defined vpp_lib_dir '{}'", vpp_lib_dir);
    println!("cargo:info=defined library_filename '{}'", library_filename);

    if !std::path::Path::new(&format!("{}/{}", &vpp_lib_dir, &library_filename)).exists() {
        panic!(
            "Can not find libvppapiclient.so.<version> at {}",
            vpp_lib_dir
        )
    };

    let flags = format!(
        "cargo:rustc-flags=-L\"{}\",-l:{}",
        &vpp_lib_dir, &library_filename
    );
    // Tell cargo to tell rustc to link the VPP client library
    println!("{}", flags);

    println!("cargo:rustc-env=GIT_VERSION=version {}", &git_version());

    let bindings = bindgen::Builder::default()
        .header("src/shmem_wrapper.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file_name = out_path.join("bindings.rs");
    bindings
        .write_to_file(out_file_name.clone())
        .expect("Couldn't write bindings!");
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

fn git_version() -> String {
    use std::process::Command;

    let describe_output = Command::new("git")
        .arg("describe")
        .arg("--all")
        .arg("--long")
        .output()
        .unwrap();

    let mut describe = String::from_utf8_lossy(&describe_output.stdout).to_string();
    describe.pop();
    describe
}
