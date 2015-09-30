#!/bin/bash

#set -euo pipefail

error_handler()
{
    LINE=${1}
    echo "Error at line #$LINE"
    exit 1
}

trap 'error_handler $LINENO' ERR

export LD_PRELOAD=target/debug/libfaultinj.so 

# Success cases:

LIBFAULTINJ_ERROR_PATH=Cargo.toml \
    LIBFAULTINJ_ERROR_OPEN=35 \
    cat src/fault.rs > /dev/null

trap - ERR
set +e

# Failure cases:
LIBFAULTINJ_ERROR_PATH=Cargo.toml \
    LIBFAULTINJ_ERROR_OPEN=35 cat Cargo.toml > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO

exit 0
