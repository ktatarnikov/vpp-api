use vpp_api_gen::api_gen::opts::OptParseType;
use vpp_api_gen::api_gen::opts::Opts;

fn main() {
    let opts = Opts {
        in_file: "../vpp-native-client-lib-sys/api".into(),
        out_file: "".into(),
        parse_type: OptParseType::Tree,
        package_name: "25_10".into(),
        vppapi_opts: "".into(),
        package_path: "./gen".into(),
        print_message_names: true,
        create_binding: true,
        create_package: false,
        generate_code: true,
        verbose: 2,
    };
    std::fs::create_dir_all(&format!("{}/{}/src", &opts.package_path, opts.package_name))
        .expect("Error creating package dir");

    vpp_api_gen::parse_type_tree(&opts);
}
