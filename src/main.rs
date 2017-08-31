#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
//#[macro_use]

extern crate futures;
extern crate iron;
extern crate mount;
//extern crate nix;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate sloggers;
extern crate staticfile;
extern crate tokio_core;
extern crate tokio_io;

use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::thread;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use errors::*;
use futures::stream::Stream;
use iron::Iron;
use mount::Mount;
use rand::Rng;
use sloggers::Build;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use staticfile::Static;
use tokio_io::AsyncRead;
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;

mod errors {
    error_chain!{}
}

quick_main!(start);

pub fn start() -> Result<()> {
    let default_buffer_size = format!("{}", 10 * 1000 * 1000);
    let default_slug_len = 5.to_string();

    let app = App::new("chefi")
        .version(crate_version!())
        .author("Cole Mickens")
        .about("Clone of `solusipse/fiche` (aka termbin.com) in Rust")
        .arg(
            Arg::with_name("listen")
                .long("listen")
                .default_value("0.0.0.0")
                .help("Address to listen on."),
        )
        .arg(
            Arg::with_name("tcp-paste-port")
                .long("tcp-paste-port")
                .default_value("9999")
                .help("Port to listen on (TCP)"),
        )
        .arg(
            Arg::with_name("http-paste-port")
                .long("http-paste-port")
                .default_value("9998")
                .help("Port to listen on (HTTP)"),
        )
        .arg(
            Arg::with_name("buffer-size")
                .long("buffer-size")
                .default_value(&default_buffer_size)
                .help("Size of the buffer to read into"),
        )
        .arg(
            Arg::with_name("domain")
                .long("domain")
                .default_value("localhost")
                .help("Domain name pastes are served on"),
        )
        .arg(
            Arg::with_name("http-serve-port")
                .long("http-serve-port")
                .default_value("9090")
                .help("Port to listen on (HTTP)"),
        )
        .arg(
            Arg::with_name("slug-len")
                .long("slug-len")
                .default_value(&default_slug_len)
                .help("Length of the slug for the pastes"),
        )
        .arg(
            Arg::with_name("storage-dir")
                .long("storage-dir")
                .default_value("/tmp/chefi/data")
                .help("Storage location for pastes"),
        );

    let matches = app.get_matches();

    run(matches)
}

pub fn run(matches: ArgMatches) -> Result<()> {
    // parse application arguments
    let tcp_paste_port =
        value_t!(matches.value_of("tcp-paste-port"), u16).unwrap_or_else(|e| e.exit());
    let http_paste_port =
        value_t!(matches.value_of("http-paste-port"), u16).unwrap_or_else(|e| e.exit());

    let buffer_size = value_t!(matches.value_of("buffer-size"), usize).unwrap_or_else(|e| e.exit());
    let domain = matches.value_of("domain").unwrap().to_string();
    let slug_len = value_t!(matches.value_of("slug-len"), usize).unwrap_or_else(|e| e.exit());

    let http_serve_port = value_t!(matches.value_of("http-serve-port"), u16)
        .unwrap_or_else(|e| e.exit());

    let storage_dir = matches.value_of("storage-dir").unwrap().to_string();
    

    fs::create_dir_all(&storage_dir).chain_err(|| "failed to create storage dir")?;

    // build logger
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();

    run_server(
        &logger,
        tcp_paste_port,
        http_paste_port,
        buffer_size,
        &domain,
        http_serve_port,
        slug_len,
        &storage_dir,
    )
}

pub fn run_server(
    logger: &slog::Logger,
    tcp_paste_port: u16,
    _http_paste_port: u16,
    buffer_size: usize,
    domain: &str,
    http_serve_port: u16,
    slug_len: usize,
    storage_dir: &str,
) -> Result<()> {
    // serve existing pastes
    let storage_dir1 = Path::new(storage_dir);

    {
        let storage_dir = String::from(storage_dir);
        let logger = logger.clone();
        thread::spawn(move || {
            let storage_dir = Path::new(&storage_dir).clone();
            let mut mount = Mount::new();
            mount.mount("/", Static::new(storage_dir));
            info!(logger, "serving pastes"; "port" => http_serve_port, "dir" => storage_dir.to_str());
            Iron::new(mount)
                .http(format!("0.0.0.0:{}", http_serve_port).as_str())
                .chain_err(|| "failed to listen on http")
        }); //should we return the thread?
    }

    // configure async loop
    let mut lp = Core::new().expect("failed to create async Core");
    let handle = lp.handle();

    // handle tcp
    let addr = format!("0.0.0.0:{}", tcp_paste_port)
        .parse()
        .chain_err(|| "failed to parse http socket")?;
    let listener = TcpListener::bind(&addr, &handle).chain_err(|| "failed to listen on tcp")?;
    let client_logger = logger.clone();
    let srv = listener.incoming().for_each(move |(tcpconn, addr)| {
        let client_logger = client_logger.new(o!("client" => addr.ip().to_string()));
        info!(client_logger, "accepted connection");

        let slug: String = rand::thread_rng()
            .gen_ascii_chars()
            .take(slug_len)
            .collect();
        let filepath = storage_dir1.join(&slug);
        let filepath = filepath
            .to_str()
            .expect("storage path for paste was invalid")
            .to_string();

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
        //let mut paste_file = File::create(&filepath).chain_err(|| "failed to create paste file")?;

        let mut host = domain.to_owned();
        if http_serve_port != 80 {
            host = format!("{}:{}", host, &http_serve_port);
        }
        let url = format!("http://{}/{}", host, slug);

        let (mut reader, mut writer) = tcpconn.split();
        let process = futures::lazy(move || {
            let mut total_size = 0;
            loop {
                let mut buf = vec![0; buffer_size];
                let read_result = reader.read(&mut buf).map_err(|e| {
                    warn!(client_logger, "failed to read from client"; "err" => format!("{}", e));
                });
                if read_result.is_err() {
                    break;
                }

                let n = read_result.unwrap();
                total_size += n;
                info!(client_logger, "read"; "size" => n);

                paste_file.write(&buf[0..n]).map_err(|_| {
                    error!(client_logger, "failed to append to file");
                })?;
                info!(client_logger, "append"; "size" => n, "filepath" => &filepath);
            }

            info!(client_logger, "read_done"; "total_size" => total_size);
            info!(client_logger, "reply"; "message" => &url);
            writer.write(format!("{}\n", &url).as_bytes()).map_err(|e| {
                error!(client_logger, "failed to reply to tcp client"; "err" => format!("{}", e));
            })?;

            info!(client_logger, "hangup");

            Ok(())
        });

        /*
        let process = read(reader, vec!(0; buffer_size)).then(move |res| {
            let (_, buf, n) = res.map_err(|_| {
                error!(client_logger, "failed to read from client");
            })?;

            paste_file.write(&buf[0..n]).map_err(|_| {
                error!(client_logger, "failed to write paste to file");
            })?;
            info!(client_logger, "persisted"; "size" => n, "filepath" => filepath);

            writer.write(format!("{}\n", url).as_bytes()).map_err(|_| {
                error!(client_logger, "failed to reply to tcp client");
            })?;
            info!(client_logger, "replied"; "message" => url);

            info!(client_logger, "finished connection");
            Ok(())
        });*/

        handle.spawn(process);
        Ok(())
    });

    info!(logger, "listening for pastes"; "port" => tcp_paste_port);
    lp.run(srv).chain_err(|| "failed to serve")
}
