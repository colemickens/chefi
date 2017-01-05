# chefi

## Status

Mostly working (albeit with some hacks). Try it out:

```
$ cargo run
 (you'll need to open another terminal)

$ echo "test" | nc localhost 9999
http://localhost:9090/OJhK5
```

The full output from `chefi` should look like this:

```
Jan 04 20:31:48.663 INFO listening for pastes, port: 9999
Jan 04 20:31:48.663 INFO serving, port: 9090, dir: /tmp/chefi/data
Jan 04 20:31:50.069 INFO accepted connection, client: 127.0.0.1
Jan 04 20:31:50.072 INFO persisted, client: 127.0.0.1, filepath: /tmp/chefi/data/OJhK5
Jan 04 20:31:50.073 INFO replied, client: 127.0.0.1, message: http://localhost:9090/OJhK5
Jan 04 20:31:50.073 INFO finished connection, client: 127.0.0.1
```

## Overview

Clone of [fiche](https://github.com/solusipse/fiche) in [Rust](https://rust-lang.org).

This is partially meant as a learning exercise and as a way to demostrate some features/libaries in Rust:
* async IO (`future-rs` and `tokio`)
* human-parseable error-backtraces (`error_chain`)
* structured logging (`slog`)
* painless CLI argument parsing (`clap.rs`)
* http static file serving (`iron.rs`)

## Thanks

Huge thanks to [24 days of Rust](https://siciarz.net/24-days-rust-conclusion-2016/).

I can't emphasize enough what a fantastic resource this is. Real patterns for
building real apps in Rust with great examples and links for further reading and
more elaborate examples. Thanks **@zsiciar** ([twitter](https://twitter.com/zsiciar)/[github](https://github.com/zsiciarz))!

## Bugs

* `Ctrl-c` doesn't stop the process when running in docker
  * could be because of `musl`, or `docker`. leaning toward `musl` right now, should be easy to narrow down
* for that matter, `fiche`/`chefi` doesn't seem to work under docker... period...
* clippy + disallow warnings is not working as expected...

## Todo

* `./build/release.sh` should use a docker container to build (also lowers barrier of entry so ppl can build w/o `rustup`/toolchains/`musl` installed
