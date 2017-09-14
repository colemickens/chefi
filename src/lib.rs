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
use std::io::BufReader;
use std::fs;
use std::fs::File;
use std::thread;
use std::path::{Path,PathBuf};

use errors::*;
use futures::future::Future;
//use futures::prelude::*;
use futures::stream::Stream;
use iron::Iron;
use mount::Mount;
use rand::Rng;
use staticfile::Static;
use tokio_io::AsyncRead;
use tokio_io::io::{copy,read};
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
        info!(client_logger, "accepted connection--------------------------------------");

        let slug: String = rand::thread_rng()
            .gen_ascii_chars()
            .take(slug_len)
            .collect();
        let filepath = storage_dir.join(&slug);
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

        /*
            loop {
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
            });
        */

        /* 
            let bytes_copied = copy(reader, paste_file);
            let handle_conn = bytes_copied.map(|(n, _, _)| {
                println!("wrote {} bytes", n)
                writer.write_all("blah")
            }).map_err(|err| {
                println!("IO error {:?}", err)
            });
        */

        
            let process = futures::lazy(move || {
                let mut total_size = 0;
                loop {
                    let mut buf = vec![0; buffer_size];
                    let read_result = try_nb!(reader.read(&mut buf));
                    
                    info!(client_logger, "append"; "size" => n, "filepath" => &filepath);


                    read_result.map_err(|e| {
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
                writer.write_all(format!("{}\n", &url).as_bytes()).map_err(|e| {
                    error!(client_logger, "failed to reply to tcp client"; "err" => format!("{}", e));
                })?;

                info!(client_logger, "hangup");

                Ok(())
            });

        
        
        handle.spawn(process);
        Ok(())
    });

    info!(logger, "running server";);
                
    core.run(server).chain_err(|| "failed to serve")
}
