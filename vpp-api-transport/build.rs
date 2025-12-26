extern crate bindgen;
use std::env;
use std::path::PathBuf;

fn main() {
    let library_filename = "vppapiclient";
    println!("cargo:info=defined library_filename '{}'", library_filename);

    let flags = format!(
        "cargo:rustc-flags=-l{}", &library_filename
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
