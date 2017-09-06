#![feature(proc_macro, conservative_impl_trait, generators)]

#[macro_use]
extern crate error_chain;
extern crate futures_await as futures;
extern crate iron;
extern crate mount;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate staticfile;
extern crate tokio_core;
extern crate tokio_io;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use std::fs::File;
use std::thread;
use std::path::Path;

use errors::*;
use futures::prelude::*;
use futures::stream::Stream;
use iron::Iron;
use mount::Mount;
use rand::Rng;
use staticfile::Static;
use tokio_io::AsyncRead;
use tokio_core::reactor::Core;
use tokio_core::net::{TcpListener, TcpStream};

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
        }); //should we return the thread?
    }

    let storage_dir = Path::new(storage_dir);

    // configure async loop
    let mut core = Core::new().expect("failed to create async Core");
    let handle = core.handle();

    // handle tcp
    let addr = format!("0.0.0.0:{}", tcp_paste_port)
        .parse()
        .chain_err(|| "failed to parse http socket")?;
    let listener = TcpListener::bind(&addr, &handle).chain_err(|| "failed to listen on tcp")?;
    let client_logger = logger.clone();

    let server = async_block! {
        #[async]
        for (client, _) in listener.incoming() {
            handle.spawn(handle_client(client).then(|result| {
                match result {
                    Ok(n) => println!("wrote {} bytes", n),
                    Err(e) => println!("IO error {:?}", e),
                }
                Ok(())
            }));
        }

        Ok::<(), std::io::Error>(())
    };

    core.run(server).chain_err(|| "failed to serve")
}

#[async]
fn handle_client(stream: TcpStream) -> std::io::Result<u64> {
    let (reader, mut writer) = stream.split();
    let input = BufReader::new(reader);

    let mut total = 0;

    #[async]
    for line in tokio_io::io::lines(input) {
        println!("got client line: {}", line);
        total += line.len() as u64;
        writer = await!(tokio_io::io::write_all(writer, line))?.0;
    }

    Ok(total)
}
