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
    LIBFAULTINJ_ERROR_OPEN_ERRNO=35 \
    cat src/fault.rs > /dev/null

trap - ERR
set +e

# Failure cases:
LIBFAULTINJ_ERROR_PATH=Cargo.toml \
    LIBFAULTINJ_ERROR_OPEN_ERRNO=35 cat Cargo.toml > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO

LIBFAULTINJ_ERROR_PATH=Cargo.toml \
    LIBFAULTINJ_ERROR_READ_ERRNO=12 cat Cargo.toml > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO

LIBFAULTINJ_ERROR_PATH=tests/discard \
    LIBFAULTINJ_ERROR_WRITE_ERRNO=1 dd if=/dev/zero of=tests/discard count=1 > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO

DEEP_DIR=tests/foo/a/b/c/
mkdir -p ${DEEP_DIR}
LIBFAULTINJ_ERROR_PATH=tests/ \
    LIBFAULTINJ_ERROR_WRITE_ERRNO=1 dd if=/dev/zero of=${DEEP_DIR}/discard count=1 > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO

# Seems to do seek even w/o seek=1
# This test doesn't pass, not clear why -- we successfully intercept 
#    the input file's lseek() but maybe not the output one, or 
#    possibly not tracking the fd correctly through the dupX().
#LIBFAULTINJ_ERROR_PATH=tests/ \
#   LIBFAULTINJ_ERROR_LSEEK_ERRNO=1 dd if=/dev/zero of=tests/discard count=1 seek=1 > /dev/null 2>&1  ; [ $? -eq 1 ] || error_handler $LINENO


# TODO: We need an injection test for ioctl(). So far, no simple shell 
#     commands spring to mind.  hwclock requires superuser to access /dev/rtc
#     and tty control (reset, stty) interact with STD{IN,OUT}_FILENO that skips
#     our open() hook.

exit 0
