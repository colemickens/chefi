#![feature(conservative_impl_trait)]

extern crate bodyparser;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate iron;
extern crate rand;
extern crate router;
#[macro_use]
extern crate slog;
extern crate staticfile;
extern crate tokio_core;
extern crate tokio_io;

use std::io::prelude::*;
use std::fs;
use std::thread;
use std::path::{Path, PathBuf};

use errors::*;
use futures::stream::Stream;
use iron::prelude::*;
use iron::{Handler,Iron,Request,Response};
use iron::status;
use rand::Rng;
use slog::Logger;
use staticfile::Static;
use router::Router;
use tokio_io::AsyncRead;
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}

fn get_slug(slug_len: usize) -> String {
    rand::thread_rng()
        .gen_ascii_chars()
        .take(slug_len)
        .collect()
}

fn get_url(domain: &str, slug: &str, http_port: u16) -> String {
    let mut host = domain.to_owned();
    if http_port != 80 {
        host = format!("{}:{}", host, &http_port);
    }
    let url = format!("http://{}/{}", host, slug);
    url
}

pub fn run_http(http_port: u16,
            slug_len: usize,
            buffer_size: usize,
            domain: &str,
            storage_dir: &Path,
            logger: &Logger)
{
    let domain: String = domain.to_owned();
    let storage_dir = storage_dir.to_path_buf();
    let logger = logger.clone();

    thread::spawn(move || {
        let mut router = Router::new();
        
        let files = Static::new(&storage_dir);
        router.post("/", handle_http_paste(storage_dir.clone(), slug_len, domain, http_port), "handle_http_paste");
        router.get("/usage", print_usage, "print_usage");
        router.get("/:file", files, "files");

        info!(
            logger,
            "serving pastes"; "port" => http_port, "dir" => storage_dir.to_str());
        let m = Iron::new(router);
        let h = m.http(format!("0.0.0.0:{}", http_port).as_str());
        h.chain_err(|| "failed to listen on http")
    });
}

fn print_usage(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "USAGE GOES HERE")))
}

fn handle_http_paste(storage_dir: PathBuf, slug_len: usize, domain: String, http_port: u16) -> impl Handler {
    move |req: &mut Request| -> IronResult<Response> {
        // TODO: convert to streaming
        let body = req.get::<bodyparser::Raw>();
        match body {
            Ok(Some(body)) => {
                let slug = get_slug(slug_len);
                let url = get_url(&domain, &slug, http_port);
                let filepath = storage_dir.join(&slug);
                // TODO: error handling, etc
                let mut paste_file = fs::File::create(&filepath).expect("failed to create paste file");

                paste_file.write_all(body.as_bytes()).unwrap(); // TODO: error handle
                // TODO: save to file

                Ok(Response::with((status::Ok, url)))
            },
            Ok(None) => {
                println!("No body");
                Ok(Response::with((status::BadRequest, "No Body")))
            },
            Err(err) => {
                println!("Error: {:?}", err);
                Ok(Response::with((status::InternalServerError, "err")))
            },
        }
    }
}

pub fn run_tcp(
    http_port: u16,
    tcp_port: u16,
    slug_len: usize,
    buffer_size: usize,
    domain: &str,
    timeout: std::time::Duration,
    storage_dir: &Path,
    logger: &slog::Logger,
) -> Result<()> {
    let storage_dir = PathBuf::from(storage_dir);

    // configure async loop
    let mut core = Core::new().expect("failed to create async Core");
    let handle = core.handle();

    // handle tcp
    let addr = format!("0.0.0.0:{}", tcp_port)
        .parse()
        .chain_err(|| "failed to parse http socket")?;
    let listener = TcpListener::bind(&addr, &handle).chain_err(|| "failed to listen on tcp")?;
    let client_logger = logger.clone();

    let server = listener.incoming().for_each(move |(tcpconn, addr)| {
        let client_logger = client_logger.new(o!("client" => addr.ip().to_string()));
        info!(client_logger, "accepted connection");

        let slug = get_slug(slug_len);   
        let url = get_url(domain, &slug, http_port);     
        let filepath = storage_dir.join(&slug);

        // TODO: lifetime/cloning or error_chain issue:
        let mut paste_file = fs::File::create(&filepath).expect("failed to create paste file");
        //let mut paste_file = File::create(&filepath).chain_err(|| "failed to create paste file")?;

        let (mut reader, mut writer) = tcpconn.split();
        let process = futures::lazy(move || {
            let mut total_size = 0;
            let mut last_received_timestamp = std::time::Instant::now();
            loop {
                let mut buf = vec![0; buffer_size];
                let read_result = reader.read(&mut buf);

                match read_result {
                    // TODO: remove this arm when Windows isn't being weird
                    Ok(n) if n == 0 => {
                        // this entire thing is put here for windows which seems to just keep reading 0 bytes
                        debug!(client_logger, "ZERO BYTE READ...");

                        // TODO: see note from where I copied this:
                        let duration = std::time::Duration::from_millis(100);
                        std::thread::sleep(duration);
                        if std::time::Instant::now() > last_received_timestamp + timeout {
                            debug!(client_logger, "TIMEOUT: {:?}", read_result);
                            break;
                        }
                    },
                    
                    Ok(n) => {
                        last_received_timestamp = std::time::Instant::now();
                        total_size += n;

                        debug!(client_logger, "received data";
                            "length" => n, "total_length" => total_size);

                        paste_file.write_all(&buf[0..n]).map_err(|_| {
                            error!(client_logger, "failed to append to file");
                            error!(client_logger, "ERR: failed to append to file");
                        })?;

                        debug!(client_logger, "Copied {} bytes to disk", n);
                    },
                    Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                        debug!(client_logger, "WOULDBLOCK...");

                        // TODO: my guess is that I should be implementing Poll somehow
                        // and then letting tokio manage this... rather than sleeping, etc
                        let duration = std::time::Duration::from_millis(100);
                        std::thread::sleep(duration);

                        if std::time::Instant::now() > last_received_timestamp + timeout {
                            debug!(client_logger, "TIMEOUT: {:?}", read_result);
                            break;
                        }
                    }
                    Err(e) => {
                        error!(client_logger, "ERR: {:?}", e);
                        break;
                    }
                };
            }

            info!(client_logger, "read_complete"; "total_size" => total_size);
            info!(client_logger, "replying"; "message" => &url);
            writer.write_all(format!("{}\n", &url).as_bytes()).map_err(|e| {
                error!(client_logger, "failed to reply to tcp client"; "err" => format!("{}", e));
            })?;

            // TODO: can we actually hangout the TcpStream? Maybe it happens on drop?
            info!(client_logger, "hangup");

            Ok(())
        });

        handle.spawn(process);
        Ok(())
    });

    info!(logger, "running server";);

    core.run(server).chain_err(|| "failed to serve")
}
