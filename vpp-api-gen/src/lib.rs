extern crate strum;
extern crate strum_macros;

pub mod api_gen;

use api_gen::*;

use linked_hash_map::LinkedHashMap;
use std::string::ToString;

use crate::api_gen::opts::Opts;
use crate::api_gen::util::merge_sort;
use crate::api_gen::util::ImportsFiles;
use crate::file_schema::VppJsApiFile;
use crate::parser_helper::*;
use api_gen::code_gen::{
    copy_file_with_fixup, create_cargo_toml, gen_code, gen_code_file, generate_lib_file,
};
use std::fs;

pub fn parse_type_file(opts: &Opts, data: &String) {
    let desc = VppJsApiFile::from_str(&data).unwrap();
    eprintln!(
        "File: {} version: {} services: {} types: {} messages: {} aliases: {} imports: {} enums: {} unions: {}",
        &opts.in_file,
        &desc.vl_api_version,
        desc.services.len(),
        desc.types.len(),
        desc.messages.len(),
        desc.aliases.len(),
        desc.imports.len(),
        desc.enums.len(),
        desc.unions.len()
    );
    if opts.verbose > 1 {
        println!("Dump File: {:#?}", &desc);
    }
    let _data = serde_json::to_string_pretty(&desc).unwrap();
    // println!("{}", &data);
    let mut api_definition: Vec<(String, String)> = vec![];
    if opts.generate_code {
        gen_code_file(
            &desc,
            &opts.package_path,
            &opts.in_file,
            &mut api_definition,
        );
    }
}

pub fn parse_type_tree(opts: &Opts) {
    // it was a directory tree, descend downwards...
    let mut api_files: LinkedHashMap<String, VppJsApiFile> = LinkedHashMap::new();
    parse_api_tree(&opts, &opts.in_file, &mut api_files);
    println!("// Loaded {} API definition files", api_files.len());
    if opts.print_message_names {
        for (name, f) in &api_files {
            println!("{}", name);
            for m in &f.messages {
                let _crc = &m.info.crc.strip_prefix("0x").unwrap();
                // println!("{}_{}", &m.name, &crc);
            }
        }
    }
    if opts.generate_code {
        let mut api_definition: Vec<(String, String)> = vec![];
        for (name, f) in &api_files {
            gen_code(
                f,
                name.trim_start_matches("testdata/vpp/api"),
                // .trim_end_matches("json"),
                &mut api_definition,
                &opts.package_name,
                &opts.package_path,
            );
        }
    }
    if opts.create_binding {
        let mut import_collection: Vec<ImportsFiles> = vec![];
        // Searching for types
        for (name, f) in api_files.clone() {
            if name.ends_with("_types.api.json") {
                import_collection.push(ImportsFiles {
                    name: name.to_string(),
                    file: Box::new(f),
                })
            }
        }
        let mut api_definition: Vec<(String, String)> = vec![];
        import_collection = merge_sort(import_collection.clone(), 0, import_collection.len());
        for x in import_collection {
            println!("{}-{}", x.name, x.file.imports.len());
            gen_code(
                &x.file,
                &x.name,
                &mut api_definition,
                &opts.package_name,
                &opts.package_path,
            );
        }
        // Searching for non types
        for (name, f) in api_files.clone() {
            if !name.ends_with("_types.api.json") {
                gen_code(
                    &f,
                    &name,
                    &mut api_definition,
                    &opts.package_name,
                    &opts.package_path,
                );
            }
        }
    }
    if opts.create_package {
        // println!("{}", opts.package_name);
        let mut api_definition: Vec<(String, String)> = vec![];
        println!("Do whatever you need to hear with creating package");
        fs::create_dir_all(&format!("{}/{}", &opts.package_path, opts.package_name))
            .expect("Error creating package dir");
        fs::create_dir_all(&format!("{}/{}/src", opts.package_path, opts.package_name))
            .expect("Error creating package/src dir");
        fs::create_dir_all(&format!(
            "{}/{}/tests",
            opts.package_path, opts.package_name
        ))
        .expect("Error creating package/tests dir");
        fs::create_dir_all(&format!(
            "{}/{}/examples",
            opts.package_path, opts.package_name
        ))
        .expect("Error creating package/examples dir");
        generate_lib_file(&opts.package_path, &api_files, &opts.package_name);
        create_cargo_toml(&opts.package_path, &opts.package_name, &opts.vppapi_opts);
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        eprintln!("package path: {}", &crate_dir);
        copy_file_with_fixup(
            &opts.package_path,
            &format!("{}/code-templates/tests/interface-test.rs", crate_dir),
            &opts.package_name,
            "tests/interface_test.rs",
        );
        copy_file_with_fixup(
            &opts.package_path,
            &format!("{}/code-templates/examples/progressive-vpp.rs", crate_dir),
            &opts.package_name,
            "examples/progressive-vpp.rs",
        );

        let mut import_collection: Vec<ImportsFiles> = vec![];
        for (name, f) in api_files.clone() {
            if name.ends_with("_types.api.json") {
                import_collection.push(ImportsFiles {
                    name: name.to_string(),
                    file: Box::new(f),
                })
            }
        }
        import_collection = merge_sort(import_collection.clone(), 0, import_collection.len());
        for x in import_collection {
            gen_code(
                &x.file,
                &x.name,
                &mut api_definition,
                &opts.package_name,
                &opts.package_path,
            );
        }
        for (name, f) in api_files.clone() {
            if !name.ends_with("_types.api.json") {
                gen_code(
                    &f,
                    &name,
                    &mut api_definition,
                    &opts.package_name,
                    &opts.package_path,
                );
            }
        }
    }
}
