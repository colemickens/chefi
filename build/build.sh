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

set -x

RUST_CHANNEL="stable"
TARGET="x86_64-unknown-linux-musl"

# ensure prerequisites
which rustup || (echo "You must have 'rustup' available" && exit -1)
which musl-gcc || (echo "You must have 'musl' available" && exit -1)

# ensure nightly toolchain
rustup install stable
rustup install nightly # for clippy

# ensure musl toolchain
rustup target add \
	--toolchain "${RUST_CHANNEL}" \
	"${TARGET}"

# ensure rustfmt is here
# TODO: cargo install --upgrade rustfmt (https://github.com/rust-lang/cargo/issues/3496)
rustup run "${RUST_CHANNEL}" \
	cargo install --list | grep 'cargo-fmt' \
		|| cargo install rustfmt

# ensure clippy is here
# TODO: cargo install --upgrade clippy (https://github.com/rust-lang/cargo/issues/3496)
rustup run "${RUST_CHANNEL}" \
	cargo install --list | grep 'cargo-clippy' \
		|| cargo install clippy

# ensure source is formatted
rustup run "${RUST_CHANNEL}" \
	cargo fmt -- --write-mode=diff \
		|| (echo "Run 'cargo fmt'" && exit -1)

# lint source
rustup run nightly \
	cargo clippy

# build binary
rustup run "${RUST_CHANNEL}" \
	cargo build \
		--target="${TARGET}" \
		--release
