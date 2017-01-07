#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate futures;
extern crate iron;
extern crate mount;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_stream;
extern crate slog_term;
extern crate staticfile;
extern crate tokio_core;

use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::thread;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use errors::*;
use futures::Future;
use futures::stream::Stream;
use iron::Iron;
use mount::Mount;
use rand::Rng;
use slog::DrainExt;
use staticfile::Static;
use tokio_core::io::{Io, read, write_all};
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;

// TODO: Fix use of `error_chain`

mod errors {
    error_chain!{}
}

quick_main!(start);

pub fn start() -> Result<()> {
    let matches = App::new("chefi")
        .version(crate_version!())
        .author("Cole Mickens")
        .about("Clone of `solusipse/fiche` (aka termbin.com) in Rust")
        .arg(Arg::with_name("listen")
            .long("listen")
            .default_value("0.0.0.0")
            .help("Address to listen on."))
        .arg(Arg::with_name("tcp-port")
            .long("port")
            .default_value("9999")
            .help("Port to listen on (TCP)"))
        .arg(Arg::with_name("buffer")
            .long("buffer-size")
            .default_value("65536")
            .help("Size of the buffer to read into"))
        .arg(Arg::with_name("domain")
            .long("domain")
            .default_value("localhost")
            .help("Domain name pastes are served on"))
        .arg(Arg::with_name("http-port")
            .long("http-port")
            .default_value("9090")
            .help("Port to listen on (HTTP)"))
        .arg(Arg::with_name("slug-len")
            .long("slug-len")
            .default_value("5")
            .help("Length of the slug for the pastes"))
        .arg(Arg::with_name("storage-dir")
            .long("storage-dir")
            .default_value("/tmp/chefi/data")
            .help("Storage location for pastes"))
        .arg(Arg::with_name("log-file")
            .long("log-file")
            .default_value("/tmp/chefi/log.json")
            .help("Location for log file"))
        .get_matches();

    run(matches)
}

pub fn run(matches: ArgMatches) -> Result<()> {
    // parse application arguments
    let tcp_port = value_t!(matches.value_of("tcp-port"), u16).unwrap_or_else(|e| e.exit());
    let buffer_size = value_t!(matches.value_of("buffer"), usize).unwrap_or_else(|e| e.exit());
    let domain = value_t!(matches.value_of("domain"), String).unwrap_or_else(|e| e.exit());
    let http_port = value_t!(matches.value_of("http-port"), u16).unwrap_or_else(|e| e.exit());
    let slug_len = value_t!(matches.value_of("slug-len"), usize).unwrap_or_else(|e| e.exit());
    let storage_dir_s: String = value_t!(matches.value_of("storage-dir"), String)
        .unwrap_or_else(|e| e.exit());
    let storage_dir = Path::new(storage_dir_s.as_str());
    let log_file_s = value_t!(matches.value_of("log-file"), String).unwrap_or_else(|e| e.exit());
    let log_file = Path::new(log_file_s.as_str());

    fs::create_dir_all(storage_dir).chain_err(|| "failed to create storage dir")?;

    // configure logging
    let console_drain = slog_term::streamer().build();
    let file = File::create(&log_file).chain_err(|| "Couldn't open log file")?;
    let file_drain = slog_stream::stream(file, slog_json::default());
    let logger = slog::Logger::root(slog::duplicate(console_drain, file_drain).fuse(), o!());

    // serve existing pastes
    {
        let logger = logger.clone();
        let storage_dir_s = storage_dir_s.clone();
        thread::spawn(move || {
            let mut mount = Mount::new();
            let path = Path::new(storage_dir_s.as_str());
            mount.mount("/", Static::new(path));
            info!(logger, "serving pastes"; "port" => http_port, "dir" => path.to_str());
            Iron::new(mount)
                .http(format!("0.0.0.0:{}", http_port).as_str())
                .chain_err(|| "failed to listen on http")
        });
    }

    // configure async loop
    let mut lp = Core::new().expect("failed to create async Core");
    let handle = lp.handle();

    // handle tcp
    let addr = format!("0.0.0.0:{}", tcp_port).parse().chain_err(|| "failed to parse http socket")?;
    let listener = TcpListener::bind(&addr, &handle).chain_err(|| "failed to listen on tcp")?;
    let client_logger = logger.clone();
    let srv = listener.incoming().for_each(move |(tcpconn, addr)| {
        let client_logger = client_logger.new(o!("client" => addr.ip().to_string()));
        info!(client_logger, "accepted connection");

        let slug: String = rand::thread_rng().gen_ascii_chars().take(slug_len).collect();
        let filepath = storage_dir.join(&slug);
        let filepath = filepath.to_str().expect("storage path for paste was invalid").to_string();

        // TODO: how to make this work?
        // I can't tell if this is a problem stemming from tokio and for_each....
        // Or if this is an issue with error_chain...
        // chain_err seemed to work fine above for the log output file........
        //
        //
        //
        // vvvv
        //let mut paste_file = File::create(&filepath).chain_err(|| "failed to create paste file")?;
        // ^^^^
        // TODO: does chain_err give me much over expect(...) in this case anyway?
        let mut paste_file = File::create(&filepath).expect("failed to create paste file");

        let mut host = domain.clone();
        if http_port != 80 {
            host = format!("{}:{}", host, http_port);
        }
        let url = format!("http://{}/{}", host, slug);

        let (reader, writer) = tcpconn.split();
        let process = read(reader, vec!(0; buffer_size)).then(move |res| {
            let result = match res {
                Ok((_, buf, n)) => {
                    // TODO: should I match here and error!() if it fails to write? should I let it
                    // bubble up? likewise with the other write_all
                    paste_file.write(&buf[0..n])
                        .expect("failed to write paste to file");
                    info!(client_logger, "persisted"; "filepath" => filepath);

                    write_all(writer, format!("{}\n", url).as_bytes())
                        .wait()
                        .expect("failed to reply to tcp client");
                    info!(client_logger, "replied"; "message" => url);

                    info!(client_logger, "finished connection");
                    Ok(())
                }
                Err(e) => {
                    error!(client_logger, "failed to read from client");
                    Err(e)
                }
            };
            // If I return `result` here then I get this complaint:
            //
            // error[E0271]: type mismatch resolving `<futures::Then<tokio_core::io::Read<tokio_core::io::ReadHalf<tokio_core::net::TcpStream>, std::vec::Vec<u8>>, std::result::Result<(), std::io::Error>, [closure@src/main.rs:159:63: 193:10 paste_file:_, client_logger:_, filepath:_, writer:_, url:_]> as futures::Future>::Error == ()`
            //    --> src/main.rs:195:16
            //     |
            // 195 |         handle.spawn(process);
            //     |                ^^^^^ expected struct `std::io::Error`, found ()
            //     |
            //     = note: expected type `std::io::Error`
            //     = note:    found type `()`
            //
            result
        });

        handle.spawn(process);
        Ok(())
    });

    info!(logger, "listening for pastes"; "port" => tcp_port);
    lp.run(srv).chain_err(|| "failed to serve")
}
