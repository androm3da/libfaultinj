#![feature(dynamic_lib,path_relative_from,hashmap_hasher)]

extern crate libc;
extern crate errno;


use std::collections::hash_state::DefaultState;
use std::hash::{Hasher, SipHasher};

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref DELAY_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
    static ref ERR_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
}


macro_rules! get_libc_func(
    ($destination_t:ty, $funcname:expr) =>
        (
            {
                use std::dynamic_lib::DynamicLibrary;
                use std::mem::transmute;
                use std::path::Path;

                let system_c_library: &str = "libc.so.6";

                unsafe {
                    let libc_dl = match DynamicLibrary::open(Some(Path::new(system_c_library))) {
                        Ok(libc) => libc,
                        Err(error) => panic!("Couldn't open libc: '{}'", error),
                    };

                    match libc_dl.symbol::<c_void>($funcname) {
                        Ok(open_func) => transmute::<* mut c_void, $destination_t>(open_func),
                        Err(error) => panic!("Couldn't '{}'", error),
                }
            }
        })
);


use errno::{Errno, set_errno};
use std::sync::RwLock;

use std::collections::hash_set::HashSet;

use libc::{c_char, c_int, c_void};



type OpenFunc = fn(* const c_char, c_int, libc::mode_t) -> c_int;
use libc::types::os::arch::posix88::ssize_t;
type ReadFunc = fn(fd: c_int, buf: * mut c_void, nbytes: c_int) -> ssize_t;
type WriteFunc = ReadFunc;
type CloseFunc = fn(fd: c_int) -> c_int;

use std::env;

fn get_delay_amount_ms() -> u32 {
    let default_delay_amount_ms = 200;

    match env::var("LIBFAULTINJ_DELAY_MS") {
        Ok(p) => match p.parse::<u32>() {
            Ok(i) => i,
            Err(_) => default_delay_amount_ms,
        },
        Err(_) => default_delay_amount_ms,
    }
}

use std::path::Path;

macro_rules! matchesPath(
        ($filename: expr, $env_name: expr) =>
    {
        match env::var($env_name) {
            Ok(p) => {
                let filename_path = Path::new(&$filename);
                let delay_path_match = (filename_path.relative_from(Path::new(&p))) != None;
                let filename_match = filename_path == Path::new(&p);

                delay_path_match || filename_match
            }
            Err(_) => false
        }
    });

fn initialize_sets() {
    let fd: c_int = 0;
    DELAY_FDS.read().unwrap().contains(&fd);
    ERR_FDS.read().unwrap().contains(&fd);
}

#[no_mangle]
pub extern "C" fn open(filename_: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    let filename: String = unsafe {
        std::ffi::CStr::from_ptr(filename_).to_string_lossy().into_owned()
    };
    let open_func = get_libc_func!(OpenFunc, "open");
    let fd: c_int = open_func(filename_, flags, mode);

    initialize_sets();

    if matchesPath!(filename, "LIBFAULTINJ_DELAY_PATH") {
        DELAY_FDS.write().unwrap().insert(fd);
    }

    fd
}

#[no_mangle]
pub extern "C" fn read(fd: c_int, buf: *mut c_void, nbytes: c_int) -> ssize_t {
    use std::thread;
    let read_func = get_libc_func!(ReadFunc, "read");

    let delay_match = DELAY_FDS.read().unwrap().contains(&fd);
    let err_match = ERR_FDS.read().unwrap().contains(&fd);

    let ret: ssize_t = read_func(fd, buf, nbytes);

    if delay_match {
        thread::sleep_ms(get_delay_amount_ms());
    }
    if err_match {
        use libc::consts::os::posix88::EIO;
        set_errno(Errno(EIO));

        return -1
    }

    ret
}

#[no_mangle]
pub extern "C" fn write(fd: c_int, buf: *mut c_void, nbytes: c_int) -> ssize_t {
    let write_func = get_libc_func!(WriteFunc, "write");

    let matches = DELAY_FDS.read().unwrap().contains(&fd);

    let ret: ssize_t = write_func(fd, buf, nbytes);

    if matches {
        use libc::consts::os::posix88::EIO;
        set_errno(Errno(EIO));

        return -1
    }

    ret
}

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    let close_func = get_libc_func!(CloseFunc, "close");

    let ret: c_int = close_func(fd);

    let mut err_fds = ERR_FDS.write().unwrap();
    if err_fds.contains(&fd) {
        err_fds.remove(&fd);
    }

    let mut delay_fds = DELAY_FDS.write().unwrap();
    if delay_fds.contains(&fd) {
        delay_fds.remove(&fd);
    }

    ret
}
