#!/usr/bin/env bash

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

# TODO: Do I need `find` all the `*.rs` files to invoke
fmtoutput="$(rustfmt --write-mode=diff "${ROOT}/src/main.rs")"

if [[ ! -z "${fmtoutput}" ]]; then
    echo "Your code base is not properly formatted." >/dev/stderr
    echo "->${fmtoutput}<-" >/dev/stderr
fi
