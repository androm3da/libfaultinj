
# Fault Injection

`libfaultinj` is a fault-injection library.  In the context in which your
software executes, there's some physical device that ultimately carries
out the tasks (ICs, disks, network cables).  These things are typically reliable
enough that finding one to malfunction can be difficult.  Getting one to
malfunction on demand for a regression test is often impossible.  So how can
you test whether your code is robust in the face of a stalled network
connection or disk media error?  

`libfaultinj` provides a facility that  intercepts the functions that drive
much of an application's activity.  Regardless of whether you're using Java,
Python, ruby, C, C++ -- nearly all calls to the system will go through the
system's C library.  Many of them will involve functions like `open()`,
`read()`, `write()` and `close()`.  Misbehavior here is all that it takes to
make a good device seem bad.

By leveraging `libfaultinj` in your test suite, you can define what the
acceptable results are for these oddities.  If the file write takes longer
than you expect, your application should probably return a `5xx` HTTP status
code, or throw an exception, or whatever's appropriate.  But it probably
shouldn't flip out and `SIGBUS` or queue thousands of retries.

[![Build Status](https://travis-ci.org/androm3da/libfaultinj.svg?branch=master)](https://travis-ci.org/androm3da/libfaultinj)


## Concept

Use `LD_PRELOAD` to load `libfaultinj` in front of your platform's C library.  It will
conditionally execute a delay before executing the operation or return an error instead
of executing the operation.

This library is supported on linux, at least `x86_64` and ARM.  Note that
building it requires nightly, and there's a good chance that may stay that
way for some time.  It critically depends on `dynamic_lib`, and less so
some others.

TODO: testing w/`DYLD_INSERT_LIBRARIES` on OS X or other similar platforms.

## Usage

You can download a binary release from github ([latest release](//github.com/androm3da/libfaultinj/releases/latest)), or build one
yourself from the source:

### Building

Get [rust-nightly](https://www.rust-lang.org/downloads.html) or just

    $ curl -sSf https://static.rust-lang.org/rustup.sh | sh -s -- --channel=nightly

Build the package via `cargo`:

    $ cd libfaultinj
    $ cargo build --release

Promote the target to a system location:

    $ mkdir -p /opt/libfaultinj/lib
    $ cp target/release/libfaultinj.so /opt/libfaultinj/lib

Now you can use it via `LD_PRELOAD` per the examples like so:

    $ LD_PRELOAD=/opt/libfaultinj/lib/libfaultinj.so ...

### Overview
There's two primary use cases that `libfaultinj` works well in:

* Unit tests, injecting specific narrow faults and asserting
specific expected results.
* System/integration tests, injecting fuzzy faults and asserting high-level
generic results ("didn't crash", e.g.)

## High level examples
* [Python](#python-example)
* [Ruby](#ruby-example)

### Supported intercept functions
An initial set of calls are below.  Others may be considered, but these cover
quite a bit of functionality.

* `open`
* `read`
* `ioctl`
* `lseek`
* `write`
* `dup3`
* `connect`
* `bind`

### Inject Errors
First, set `LIBFAULTINJ_ERROR_PATH` to the directory or filename to have errors injected upon.  Then set
`LIBFAULT_ERROR_{READ,WRITE,LSEEK}_ERRNO` to your target's errno to be set on each time the corresponding
operation is executed with a file descriptor that came from `LIBFAULTINJ_ERROR_PATH`.

The path described by `LIBFAULTINJ_ERROR_PATH` is effectively recursive into its subdirectories.

    $ LD_PRELOAD=libfaultinj.so \
      LIBFAULTINJ_ERROR_PATH=./testing_dir/ \
      LIBFAULTINJ_ERROR_READ_ERRNO=12 \
      cat ./testing_dir/foo.txt
    cat: ./testing_dir/foo.txt: Cannot allocate memory

    $ LD_PRELOAD=libfaultinj.so \
      LIBFAULTINJ_ERROR_PATH=./testing_dir/ \
      LIBFAULTINJ_ERROR_READ_ERRNO=12 \
      cat ./testing_dir/a/b/c/foo.txt
    cat: ./testing_dir/a/b/c/foo.txt: Cannot allocate memory

#### Fuzzy
Occasional errors can be simulated by using the `LIBFAULTINJ_ERROR_LIKELIHOOD_PCT` environment
variable.  If not set or set to a number that cannot be parsed, the "likelihood" is "certain."

If you want to simulate infrequent failures in some operations, you can set the likelihood to
a small value.

TODO: a better example showing a ratio

    $ LD_PRELOAD=libfaultinj.so \
      LIBFAULTINJ_ERROR_LIKELIHOOD_PCT=0.3 \
      LIBFAULTINJ_ERROR_PATH=./testing_dir/ \
      LIBFAULTINJ_ERROR_READ_ERRNO=12 \
      cat ./testing_dir/foo.txt

### Inject Delays
First, set `LIBFAULTINJ_DELAY_PATH` to the directory or filename to be delayed.  Then set
`LIBFAULT_DELAY_{READ,WRITE,LSEEK}_MS` to the decimal representation of the number of
milliseconds to delay that operation.

For example, without any `LD_PRELOAD`:

    $ LIBFAULTINJ_DELAY_PATH=./testing_dir/ \
      LIBFAULTINJ_DELAY_READ_MS=10000 \
      \time cat ./testing_dir/foo.txt
    0.00user 0.00system 0:00.00elapsed ?%CPU (0avgtext+0avgdata 1748maxresident)k
    0inputs+0outputs (0major+79minor)pagefaults 0swaps

The `cat` command completes faster than the smallest precision shown by `time` -- 0.00:00 elapsed.  But, with `libfaultinj.so` enabled in `LD_PRELOAD`:

    $ LD_PRELOAD=libfaultinj.so \
      LIBFAULTINJ_DELAY_PATH=./testing_dir/ \
      LIBFAULTINJ_DELAY_READ_MS=10000 \
      \time cat ./testing_dir/foo.txt
    0.00user 0.00system 0:10.00elapsed 0%CPU (0avgtext+0avgdata 3452maxresident)k
    0inputs+0outputs (0major+141minor)pagefaults 0swaps

...it shows 0:10.00elapsed.


### Python example

Using the `unittest` module, you can make a simple example of fault injection with Python like the example below:

    filename = './thisfile.txt'
    os.environ['LIBFAULTINJ_ERROR_PATH'] = filename
    os.environ['LIBFAULTINJ_ERROR_OPEN_ERRNO'] = str(errno.ENOMEM)

    with self.assertRaises(EnvironmentError):
        with open(filename, 'rt') as f:
            f.read()

Note that this test will only pass if it's executed like "`LD_PRELOAD=libfaultinj.so python ...`"

### Ruby example

Here's the corresponding example using ruby:

    FileUtils.touch(@@file_name)
    ENV['LIBFAULTINJ_ERROR_PATH'] = @@file_name
    ENV['LIBFAULTINJ_ERROR_OPEN_ERRNO'] = '2'

    assert_raise( Errno::ENOENT ) { x = File.read(@@file_name) }
