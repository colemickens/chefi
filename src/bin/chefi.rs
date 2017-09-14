extern crate chefi;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate slog;
extern crate sloggers;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use errors::*;
use sloggers::Build;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use structopt::StructOpt;

mod errors {
    error_chain!{}
}

#[derive(StructOpt, Debug)]
#[structopt(name = "chefi", description = "tcp/http pastebin (clone of fiche)")]
struct ChefiArgs {
    #[structopt(long = "listen", short = "l", default_value = "0.0.0.0")]
    listen: String,

    #[structopt(long = "tcp-paste-port", default_value = "9999")]
    tcp_paste_port: u16,

    #[structopt(long = "http-paste-port", default_value = "9998")]
    http_paste_port: u16,

    #[structopt(long = "buffer-size", default_value = "10000000")]
    buffer_size: usize,

    #[structopt(long = "domain", default_value = "localhost")]
    domain: String,

    #[structopt(long = "http-serve-port", default_value = "9090")]
    http_serve_port: u16,

    #[structopt(long = "slug-len", default_value = "5")]
    slug_len: usize,

    #[structopt(long = "storage-dir", default_value = "/tmp/chefi/data")]
    storage_dir: String,
}

quick_main!(start);

pub fn start() -> Result<()> {
    let args = ChefiArgs::from_args();

    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();

    // TODO: Consider parsing out the dir and create it first
    // then pass it in to run_server

    info!(logger, "running server";
        "port" => &args.tcp_paste_port, "dir" => &args.storage_dir);

    chefi::run_server(
        &logger,
        args.tcp_paste_port,
        args.http_paste_port,
        args.buffer_size,
        &args.domain,
        args.http_serve_port,
        args.slug_len,
        &args.storage_dir,
        std::time::Duration::from_secs(5),
    ).chain_err(|| "oops")
}
