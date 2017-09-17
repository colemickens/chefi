extern crate chefi;

#[macro_use]
extern crate error_chain;
extern crate sloggers;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::path::Path;

use errors::*;
use sloggers::Build;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use structopt::StructOpt;

mod errors {
    error_chain!{}
}

#[derive(StructOpt, Debug)]
#[structopt(name = "chefi", description = "tcp/http pastebin (clone of fiche)")]
struct ChefiArgs {
    #[structopt(long = "listen", short = "l", default_value = "0.0.0.0")]
    listen: String,

    #[structopt(long = "tcp-port", default_value = "9999")]
    tcp_port: u16,

    #[structopt(long = "http-port", default_value = "8080")]
    http_port: u16,

    #[structopt(long = "buffer-size", default_value = "10000000")]
    buffer_size: usize,

    #[structopt(long = "domain", default_value = "localhost")]
    domain: String,

    #[structopt(long = "slug-len", default_value = "5")]
    slug_len: usize,

    #[structopt(long = "storage-dir", default_value = "/tmp/chefi/data")]
    storage_dir: String,

    #[structopt(long = "log-level", default_value = "info")]
    log_level: sloggers::types::Severity,
}

quick_main!(start);

pub fn start() -> Result<()> {
    let args = ChefiArgs::from_args();

    let mut builder = TerminalLoggerBuilder::new();
    builder.level(args.log_level);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();

    let storage_dir = Path::new(&args.storage_dir);
    std::fs::create_dir_all(&storage_dir).chain_err(|| "failed to create storage dir")?;

    let duration = std::time::Duration::from_secs(5);

    // run_http spawns a new thread
    chefi::run_http(
        args.http_port,
        args.slug_len,
        args.buffer_size,
        &args.domain,
        &storage_dir,
        &logger,
    );

    // ? returns or blocks?
    chefi::run_tcp(
        args.http_port,
        args.tcp_port,
        args.slug_len,
        args.buffer_size,
        &args.domain,
        duration,
        &storage_dir,
        &logger,
    ).chain_err(|| "oops")
}
