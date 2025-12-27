use std::string::ToString;
extern crate strum;
use crate::api_gen::file_schema::*;
use crate::api_gen::opts::Opts;
use crate::api_gen::types::*;

use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

pub fn parse_api_tree(opts: &Opts, root: &str, map: &mut LinkedHashMap<String, VppJsApiFile>) {
    use std::fs;
    if opts.verbose > 2 {
        println!("parse tree: {:?}", root);
    }
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if opts.verbose > 2 {
            println!("Entry: {:?}", &entry);
        }

        let metadata = fs::metadata(&path).unwrap();
        if metadata.is_file() {
            let res = std::fs::read_to_string(&path);
            if let Ok(data) = res {
                let desc = VppJsApiFile::from_str(&data);
                if let Ok(d) = desc {
                    map.insert(path.to_str().unwrap().to_string(), d);
                } else {
                    eprintln!("Error loading {:?}: {:?}", &path, &desc);
                }
            } else {
                eprintln!("Error reading {:?}: {:?}", &path, &res);
            }
        }
        if metadata.is_dir() && entry.file_name() != "." && entry.file_name() != ".." {
            parse_api_tree(opts, path.to_str().unwrap(), map);
        }
    }
}
pub fn get_type(apitype: &str) -> String {
    if apitype.starts_with("vl_api_") {
        // let ctype_trimmed = apitype.trim_start_matches("vl_api_").trim_end_matches("_t");
        // String::from(ctype_trimmed)
        camelize_ident(apitype.trim_start_matches("vl_api_").trim_end_matches("_t"))
        // camelize_ident(ctype_trimmed)
    } else if apitype == "string" {
        "String".to_string()
    } else {
        apitype.to_string()
    }
}
pub fn get_ident(api_ident: &str) -> String {
    if api_ident == "type" {
        return "typ".to_string();
    }
    if api_ident == "match" {
        // println!("Found match");
        "mach".to_string()
    } else {
        api_ident.trim_start_matches("_").to_string()
    }
}

pub fn get_rust_type_from_ctype(enum_containers: &HashMap<String, String>, ctype: &str) -> String {
    use convert_case::{Case, Casing};

    
    {
        let rtype: String = if ctype.starts_with("vl_api_") {
            let ctype_trimmed = ctype.trim_start_matches("vl_api_").trim_end_matches("_t");
            ctype_trimmed.to_case(Case::UpperCamel)
        } else {
            ctype.to_string()
        };
        /* if the candidate Rust type is an enum, we need to create
        a parametrized type such that we knew which size to
        deal with at serialization/deserialization time */

        if let Some(container) = enum_containers.get(&rtype) {
            format!("SizedEnum<{}, {}>", rtype, container)
        } else {
            rtype
        }
    }
}

pub fn get_rust_field_name(name: &str) -> String {
    if name == "type" || name == "match" {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

pub fn get_rust_field_type(
    enum_containers: &HashMap<String, String>,
    fld: &VppJsApiMessageFieldDef,
    is_last: bool,
) -> String {
    use crate::api_gen::types::VppJsApiFieldSize::*;
    let rtype = get_rust_type_from_ctype(enum_containers, &fld.ctype);
    let full_rtype = if let Some(size) = &fld.maybe_size {
        match size {
            Variable(_max_var) => {
                if fld.ctype == "string" {
                    "VariableSizeString".to_string()
                } else {
                    format!("VariableSizeArray<{}>", rtype)
                }
            }
            Fixed(maxsz) => {
                if fld.ctype == "string" {
                    format!("FixedSizeString<typenum::U{}>", maxsz)
                } else {
                    format!("FixedSizeArray<{}, typenum::U{}>", rtype, maxsz)
                }
            }
        }
    } else {
        rtype.to_string()
    };
    if fld.maybe_options.is_none() {
        full_rtype.to_string()
    } else {
        format!("{} /* {:?} {} */", full_rtype, fld, is_last)
    }
}

pub fn camelize_ident(ident: &str) -> String {
    let c = ident.split("_");
    let collection: Vec<&str> = c.collect();
    let mut final_string = String::new();

    for x in collection {
        for (i, c) in x.chars().enumerate() {
            if i == 0 {
                let c_upper: Vec<_> = c.to_uppercase().collect();
                final_string.push(c_upper[0]);
            } else {
                final_string.push(c);
            }
        }
    }
    final_string
}

pub fn camelize(ident: &str) -> String {
    use convert_case::{Case, Casing};
    ident.to_case(Case::UpperCamel)
}
