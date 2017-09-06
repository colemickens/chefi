#!/usr/bin/env bash

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
ROOT="${DIR}/.."

################################################################################

set -x

# TODO: rip all of this out
# require that the official build goes through docker

function cargo() { rustup run nightly cargo "${@}" ; }

TARGET="x86_64-unknown-linux-musl"

# ensure prerequisites
which rustup || (echo "You must have 'rustup' available" && exit -1)
which musl-gcc || (echo "You must have 'musl' available" && exit -1)

# ensure nightly toolchain
rustup install nightly

# ensure all updated
rustup update

# ensure musl toolchain
rustup target add \
	--toolchain nightly \
	"${TARGET}"

# ensure rustfmt is here
cargo install --list | grep 'cargo-fmt' \
	|| cargo install rustfmt

# ensure clippy is here
cargo install --list | grep 'cargo-clippy' \
	|| cargo install clippy

# ensure cargo-update is here
cargo install --list | grep 'cargo-update' \
	|| cargo install cargo-update

# ensure everything is updated
# TODO: remove this if/when cargo gets appropriate functionality:
#       https://github.com/rust-lang/cargo/issues/2082
cargo install-update -a

# ensure source is formatted
cargo fmt -- --write-mode=diff \
	|| (echo "Run 'cargo fmt'" && exit -1)

# lint source
cargo clippy

# build binary
cargo build \
	--target="${TARGET}" \
	--release
