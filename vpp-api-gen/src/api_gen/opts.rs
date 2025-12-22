
use clap::Parser;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Parser, Debug, Clone, Serialize, Deserialize, EnumString, Display)]
pub enum OptParseType {
    File,
    Tree,
    ApiType,
    ApiMessage,
}

/// Ingest the VPP API JSON definition file and output the Rust code
#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[clap(version = "1.0", author = "Andrew Yourtchenko <ayourtch@gmail.com>")]

pub struct Opts {
    /// Input file name
    #[clap(short, long)]
    pub in_file: String,

    /// output file name
    #[clap(short, long, default_value = "dummy.rs")]
    pub out_file: String,

    /// parse type for the operation: Tree, File, ApiMessage or ApiType
    #[clap(short, long, default_value = "File")]
    pub parse_type: OptParseType,

    /// Package name for the generated package
    #[clap(long, default_value = "someVPP")]
    pub package_name: String,

    /// Options to specify within generated Cargo.toml for all crates from vpp-api
    #[clap(
        long,
        default_value = "{ git=\"https://github.com/ayourtch/vpp-api\", branch=\"main\" }"
    )]
    pub vppapi_opts: String,

    /// Package name for the generated package
    #[clap(long, default_value = "../")]
    pub package_path: String,

    /// Print message names
    #[clap(long)]
    pub print_message_names: bool,

    /// Generate the bindings within the directory
    #[clap(long)]
    pub create_binding: bool,

    /// Generate the package for the binding
    #[clap(long)]
    pub create_package: bool,

    /// Generate the code
    #[clap(long)]
    pub generate_code: bool,

    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
}
