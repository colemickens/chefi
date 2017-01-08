# chefi

## Overview

Clone of [fiche](https://github.com/solusipse/fiche) in [Rust](https://rust-lang.org).

Tested and working in Linux and Windows (likely macOS as well).

`chefi` is an application that starts a TCP and HTTP server.
A user can post a "paste" to the service with only `netcat` and
then receives a short URL which can then be used to retrieve the paste
contents on another machine.

```shell
$ cargo run
Jan 04 23:09:02.303 INFO listening for pastes, port: 9999
Jan 04 23:09:02.303 INFO serving pastes, port: 9090, dir: /tmp/chefi/data

# (you'll need to open another terminal)

$ echo 'This is a test!' | nc localhost 9999
http://localhost:9090/OJhK5

$ curl http://localhost:9090/OJhK5
This is a test!
```

This is partially meant as a learning exercise and as a way to demostrate some features/libaries in Rust:
* async IO (`future-rs` and `tokio`)
* human-parseable error-backtraces (`error_chain`)
* structured logging (`slog`)
* painless CLI argument parsing (`clap.rs`)
* http static file serving (`iron.rs`)

## Status

Mostly working (albeit with some hacks).

Notes:

1. Please grep through the source for TODOs. There are a few places where some hacks
have been used. (For example, the directory for persisting pastes is derived from the
command line flag properly, but the http serving directory is hard coded).
2. This builds on stable rust, though the `./build/build.sh` script uses clippy and
ensures that the nightly toolchain is updated and available as well.

## Thanks

Huge thanks to [24 days of Rust](https://siciarz.net/24-days-rust-conclusion-2016/).

I can't emphasize enough what a fantastic resource this is. Real patterns for
building real apps in Rust with great examples and links for further reading and
more elaborate examples. Thanks **@zsiciar** ([twitter](https://twitter.com/zsiciar)/[github](https://github.com/zsiciarz))!

Another big thank you to @Ralith!

Ralith suggestions:
 - "make a helper function to log an error_chain error in all it's proper glory"
