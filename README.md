

`libfaultinj` is a fault-injection library.

[![Build Status](https://travis-ci.org/androm3da/libfaultinj.svg?branch=master)](https://travis-ci.org/androm3da/libfaultinj)


## Concept

Use `LD_PRELOAD` to load `libfaultinj` in front of your platform's C library.  It will 
conditionally execute a delay before executing the operation or return an error instead
of executing the operation.

This library is supported on linux, at least `x86_64` and ARM.

TODO: testing w/`DYLD_INSERT_LIBRARIES` on OS X or other similar platforms.

## Usage

### Inject Errors
First, set `LIBFAULTINJ_ERROR_PATH` to the directory or filename to have errors injected upon.  Then set
`LIBFAULT_ERROR_{READ,WRITE,LSEEK}_ERRNO` to your target's errno to be set on error.


### Inject Delays
First, set `LIBFAULTINJ_DELAY_PATH` to the directory or filename to be delayed.  Then set
`LIBFAULT_DELAY_{READ,WRITE,LSEEK}_MS` to the decimal representation of the number of
milliseconds to delay that operation.

For example:

    $ LD_PRELOAD=libfaultinj.so \
      LIBFAULTINJ_DELAY_PATH=./testing_dir/ \
      LIBFAULTINJ_DELAY_READ_MS=10000 \
      \time cat ./testing_dir/foo.txt
    0.00user 0.00system 0:10.00elapsed 0%CPU (0avgtext+0avgdata 3452maxresident)k
    0inputs+0outputs (0major+141minor)pagefaults 0swaps
    $ LIBFAULTINJ_DELAY_PATH=./testing_dir/ \
      LIBFAULTINJ_DELAY_READ_MS=10000 \
      \time cat ./testing_dir/foo.txt
    0.00user 0.00system 0:00.00elapsed ?%CPU (0avgtext+0avgdata 1748maxresident)k
    0inputs+0outputs (0major+79minor)pagefaults 0swaps

