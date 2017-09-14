#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate iron;
extern crate mount;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate staticfile;
extern crate tokio_core;
extern crate tokio_io;

use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::thread;
use std::path::{Path, PathBuf};

use errors::*;
use futures::stream::Stream;
use iron::Iron;
use mount::Mount;
use rand::Rng;
use staticfile::Static;
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

pub fn run_server(
    logger: &slog::Logger,
    tcp_paste_port: u16,
    _http_paste_port: u16,
    buffer_size: usize,
    domain: &str,
    http_serve_port: u16,
    slug_len: usize,
    storage_dir: &str,
    timeout: std::time::Duration,
) -> Result<()> {
    // serve existing pastes
    fs::create_dir_all(&storage_dir).chain_err(|| "failed to create storage dir")?;

    {
        let storage_dir = String::from(storage_dir);
        let logger = logger.clone();

        thread::spawn(move || {
            let storage_dir = Path::new(&storage_dir);
            let mut mount = Mount::new();
            mount.mount("/", Static::new(storage_dir));
            info!(
                logger,
                "serving pastes"; "port" => http_serve_port, "dir" => storage_dir.to_str());
            let m = Iron::new(mount);
            let h = m.http(format!("0.0.0.0:{}", http_serve_port).as_str());
            h.chain_err(|| "failed to listen on http")
        });
    }

    // TODO: start another thread to listen for pastes over HTTP PUT or whatever is convenient with curl
    // TODO: also, spit out usage at curl http://URLBASE/usage

    let storage_dir = PathBuf::from(storage_dir);

    // configure async loop
    let mut core = Core::new().expect("failed to create async Core");
    let handle = core.handle();

    // handle tcp
    let addr = format!("0.0.0.0:{}", tcp_paste_port)
        .parse()
        .chain_err(|| "failed to parse http socket")?;
    let listener = TcpListener::bind(&addr, &handle).chain_err(|| "failed to listen on tcp")?;
    let client_logger = logger.clone();

    let server = listener.incoming().for_each(move |(tcpconn, addr)| {
        let client_logger = client_logger.new(o!("client" => addr.ip().to_string()));
        info!(client_logger, "accepted connection");

        let slug: String = rand::thread_rng()
            .gen_ascii_chars()
            .take(slug_len)
            .collect();
        let filepath = storage_dir.join(&slug);
        let filepath = filepath
            .to_str()
            .expect("storage path for paste was invalid")
            .to_string();

        // TODO: lifetime/cloning or error_chain issue:
        let mut paste_file = File::create(&filepath).expect("failed to create paste file");
        //let mut paste_file = File::create(&filepath).chain_err(|| "failed to create paste file")?;

        let mut host = domain.to_owned();
        if http_serve_port != 80 {
            host = format!("{}:{}", host, &http_serve_port);
        }

        // TODO: lifetime/cloning issue:
        //let url = format!("http://{}/{}", host, slug);

        let (mut reader, mut writer) = tcpconn.split();

        let process = futures::lazy(move || {
            let url = format!("http://{}/{}", host, slug);

            //let timeout = std::time::Duration::from_secs(timeout);
            let mut total_size = 0;
            let mut last_received_timestamp = std::time::Instant::now();
            loop {
                let mut buf = vec![0; buffer_size];
                let read_result = reader.read(&mut buf);

                match read_result {
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
                    }
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
