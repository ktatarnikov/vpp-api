use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is expected"));

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("architecture is not defined with CARGO_CFG_TARGET_ARCH variable");

    // Pick the right library for the platform
    match target_os.as_str() {
        "linux" => (),
        _ => panic!("unsupported target OS"),
    };

    let vpp_version =
        read_version_from_feature().expect("version is expected to be set via feature");

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

fn read_version_from_feature() -> Option<String> {
    let mut versions = Vec::new();
    for (key, _) in env::vars() {
        // Cargo uppercases and replaces '-' with '_'
        if let Some(rest) = key.strip_prefix("CARGO_FEATURE_") {
            // Match only numeric version-like features: e.g. "25_10"
            let parts: Vec<_> = rest.split('_').collect();
            if parts.len() == 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                versions.push(format!("{}.{}", parts[0], parts[1]));
            }
        }
    }

    match versions.len() {
        0 => None,
        1 => Some(versions.remove(0)),
        _ => panic!("multiple version features enabled: {versions:?}"),
    }
}
