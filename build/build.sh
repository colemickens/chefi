#!/usr/bin/env bash

# <project>/build/build.sh
# Uses rustup + musl-gcc to produce a fully static binary of `chefi`.

set -e
set -u
set -o pipefail

SOURCE="${BASH_SOURCE[0]}"
while [ -h "$SOURCE" ]; do # resolve $SOURCE until the file is no longer a symlink
  DIR="$( cd -P "$( dirname "$SOURCE" )" && pwd )"
  SOURCE="$(readlink "$SOURCE")"
  [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE" # if $SOURCE was a relative symlink, we need to resolve it relative to the path where the symlink file was located
done
DIR="$( cd -P "$( dirname "$SOURCE" )" && pwd )"

################################################################################

RUST_CHANNEL="nightly"
TARGET="x86_64-unknown-linux-musl"

# ensure prerequisites
which rustup || (echo "You must have 'rustup' available" && exit -1)
which musl-gcc || (echo "You must have 'musl' available" && exit -1)

# ensure nightly toolchain
rustup install "${RUST_CHANNEL}"

# ensure musl toolchain
rustup target add \
	--toolchain "${RUST_CHANNEL}" \
	"${TARGET}"

alias cargo="rustup run ${RUST_CHANNEL} cargo"

# ensure rustfmt is here
# TODO: cargo install --upgrade rustfmt (https://github.com/rust-lang/cargo/issues/3496)
cargo install --list | grep 'cargo-fmt' \
	|| cargo install rustfmt

# ensure clippy is here
# TODO: cargo install --upgrade clippy (https://github.com/rust-lang/cargo/issues/3496)
cargo install --list | grep 'cargo-clippy' \
	|| cargo install clippy

# ensure source is formatted
cargo fmt -- --write-mode=diff \
	|| (echo "Run 'cargo fmt'" && exit -1)

# lint source
cargo clippy

# build binary
cargo build \
	--target="${TARGET}" \
	--release
