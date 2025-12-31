use anyhow::Result;
use clap::Parser as ClapParser;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::runtime::Builder;
use vpp_api_transport::afunix;
use vpp_api_transport::shmem;

/// This program is a minimum test of vpp-api-transport crate
/// To make it somewhat useful, it can also bench the cli_inband API
/// execution time for various commands
#[derive(Debug, Clone, ClapParser, Serialize, Deserialize)]
#[clap(version = env!("GIT_VERSION"), author = "Andrew Yourtchenko <ayourtch@gmail.com>")]
struct Opts {
    /// Run the bench using this CLI, else use "show version"
    #[clap(short, long)]
    command: Option<String>,

    /// Use AF_UNIX socket if this path is mentioned, else use shared memory transport
    #[clap(short, long)]
    socket_path: Option<String>,

    /// Override options from this yaml/json file
    #[clap(short, long)]
    options_override: Option<String>,

    /// set non-blocking mode for the connection
    #[clap(short, long)]
    nonblocking: bool,

    /// repeat count for the command
    #[clap(short, long, default_value = "100000")]
    repeat_count: u32,

    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

async fn bench_non_blocking(opts: &Opts, t: &mut crate::shmem::non_blocking::Client) {
    let now = SystemTime::now();
    let mut last_show = now;

    let count = opts.repeat_count;
    let command = opts.command.clone().unwrap_or("show version".to_string());
    println!("Starting {} requests of '{}'", count, &command);

    for i in 0..count {
        let result = t.run_cli_inband(&command).await;
        if opts.verbose > 2 {
            if let Ok(str) = result {
                println!("Result:\n{}", &str);
            } else {
                println!("Error Result: {:?}", &result);
            }
        }
        if let Ok(ela) = last_show.elapsed() {
            if ela.as_secs_f64() > 5.0 {
                let elapsed = now.elapsed().unwrap();
                println!(
                    "Still running... {} iterations in {:?}: {} per second",
                    i,
                    elapsed,
                    (i as f64) / elapsed.as_secs_f64()
                );
                last_show = SystemTime::now();
            }
        }
    }

    match now.elapsed() {
        Ok(elapsed) => {
            // it prints '2'
            println!(
                "Ran {} operations in {:?} : {} per second",
                count,
                elapsed,
                (count as f64) / elapsed.as_secs_f64()
            );
        }
        Err(e) => {
            // an error occurred!
            println!("Error: {:?}", e);
        }
    }
}

async fn bench(opts: &Opts, client: &mut crate::afunix::client::Client) -> Result<()> {
    let now = SystemTime::now();
    let mut last_show = now;

    let count = opts.repeat_count;
    let command = opts.command.clone().unwrap_or("show version".to_string());
    println!("Starting {} requests of '{}'", count, &command);

    for i in 0..count {
        let s = client.run_cli_inband(&command).await;
        if opts.verbose > 2 {
            if let Ok(str) = s {
                println!("Result:\n{}", &str);
            } else {
                println!("Error Result: {:?}", &s);
            }
        }
        if let Ok(ela) = last_show.elapsed() {
            if ela.as_secs_f64() > 5.0 {
                let elapsed = now.elapsed().unwrap();
                println!(
                    "Still running... {} iterations in {:?}: {} per second",
                    i,
                    elapsed,
                    (i as f64) / elapsed.as_secs_f64()
                );
                last_show = SystemTime::now();
            }
        }
    }

    match now.elapsed() {
        Ok(elapsed) => {
            // it prints '2'
            println!(
                "Ran {} operations in {:?} : {} per second",
                count,
                elapsed,
                (count as f64) / elapsed.as_secs_f64()
            );
        }
        Err(e) => {
            // an error occurred!
            println!("Error: {:?}", e);
        }
    }
    Ok(())
}

fn main() {
    let opts: Opts = Opts::parse();

    // allow to load the options, so far there is no good built-in way
    let opts = if let Some(fname) = &opts.options_override {
        if let Ok(data) = std::fs::read_to_string(&fname) {
            let res = serde_json::from_str(&data);
            if res.is_ok() {
                res.unwrap()
            } else {
                serde_yaml_bw::from_str(&data).unwrap()
            }
        } else {
            opts
        }
    } else {
        opts
    };

    if opts.verbose > 4 {
        let data = serde_json::to_string_pretty(&opts).unwrap();
        println!("{}", data);
        println!("===========");
        let data = serde_yaml_bw::to_string(&opts).unwrap();
        println!("{}", data);
    }
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    if let Some(afunix_path) = &opts.socket_path {
        let result: Result<(), anyhow::Error> = runtime.block_on(async {
            let mut client = afunix::client::Client::connect(&afunix_path, "api-test").await?;
            bench(&opts, &mut client).await?;
            client.disconnect();
            Ok(())
        });
        result.unwrap()
    } else {
        let result: Result<(), anyhow::Error> = runtime.block_on(async {
            let mut client = shmem::non_blocking::Client::connect("api-test", None, 256).await?;
            bench_non_blocking(&opts, &mut client).await;
            Ok(())
        });
        result.unwrap()
    };
}
