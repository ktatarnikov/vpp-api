use vpp_api_gen::api_gen::opts::OptParseType;
use vpp_api_gen::api_gen::opts::Opts;
use std::{env, fs, path::PathBuf};

fn main() {
    let workspace_dir_path = std::env::var("CARGO_WORKSPACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_workspace_root());
    let workspace_dir = workspace_dir_path
        .to_str()
        .expect("CARGO_WORKSPACE_DIR is invalid");

    let vpp_version = read_version_from_workspace(workspace_dir);

    let api_dir = format!("{}/vpp-native-client-lib-sys/{}/api", workspace_dir, vpp_version);

    let opts = Opts {
        in_file: api_dir,
        out_file: "".into(),
        parse_type: OptParseType::Tree,
        package_name: vpp_version.into(),
        vppapi_opts: "".into(),
        package_path: "./gen".into(),
        print_message_names: true,
        create_binding: true,
        create_package: true,
        generate_code: true,
        verbose: 2,
    };
    std::fs::create_dir_all(&format!("{}/{}/src", &opts.package_path, opts.package_name))
        .expect("Error creating package dir");

    vpp_api_gen::parse_type_tree(&opts);
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

fn read_version_from_workspace(workspace_dir: &str) -> String {
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
    vpp_version.into()
}