#![feature(dynamic_lib,path_relative_from,hashmap_hasher)]

extern crate libc;
extern crate errno;


use std::collections::hash_state::DefaultState;
use std::hash::{Hasher, SipHasher};

//use std::dynamic_lib::DynamicLibrary;
//const SYSTEM_C_LIBRARY: &'static str = "libc.so.6";
//unsafe impl Sync for DynamicLibrary { }
//unsafe impl Send for DynamicLibrary { }

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref DELAY_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
    static ref ERR_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
    //static ref LIBC: RwLock<DynamicLibrary> = RwLock::new(DynamicLibrary::open(Some(Path::new(SYSTEM_C_LIBRARY))).unwrap());
}

macro_rules! get_libc_func(
    ($destination_t:ty, $funcname:expr) =>
        (
            {
                use std::dynamic_lib::DynamicLibrary;
                use std::mem::transmute;
                use std::path::Path;

                const SYSTEM_C_LIBRARY: &'static str = "libc.so.6";

                unsafe {
                    let libc_dl = match DynamicLibrary::open(Some(Path::new(SYSTEM_C_LIBRARY))) {
                        Ok(libc) => libc,
                        Err(error) => panic!("Couldn't open libc: '{}'", error),
                    };

                    match libc_dl.symbol::<c_void>($funcname) {
                        Ok(open_func) => transmute::<* mut c_void, $destination_t>(open_func),
                        Err(error) => panic!("Couldn't find '{}': '{}'", $funcname, error),
                }
            }
        })
);


use errno::{Errno, set_errno};
use std::sync::RwLock;

use std::collections::hash_set::HashSet;

use libc::{c_char, c_int, c_void, off_t, size_t};
use libc::types::os::arch::posix88::ssize_t;


type OpenFunc = fn(* const c_char, c_int, libc::mode_t) -> c_int;
type ReadFunc = fn(fd: c_int, buf: * mut c_void, nbytes: c_int) -> ssize_t;
type WriteFunc = ReadFunc;
type MmapFunc = fn(addr: *mut c_void, length_: size_t, prot: c_int,
                   flags: c_int, fd: c_int, offset: off_t) -> *mut c_void;
type CloseFunc = fn(fd: c_int) -> c_int;
type SeekFunc = fn(c_int, off_t, c_int) -> off_t;

use std::env;

fn get_delay_amount_ms(funcname: &str) -> u32 {
    const DEFAULT_DELAY_AMOUNT_MS: u32 = 200;
    let err_prefix = "LIBFAULTINJ_DELAY_".to_string();
    let env_name = err_prefix + &String::from(funcname).to_uppercase() + "_MS";

    match env::var(env_name) {
        Ok(p) => match p.parse::<u32>() {
            Ok(i) => i,
            Err(_) => DEFAULT_DELAY_AMOUNT_MS,
        },
        Err(_) => DEFAULT_DELAY_AMOUNT_MS,
    }
}

fn get_errno(funcname: &str) -> i32 {
    use std::string::String;

    const DEFAULT_ERRNO: i32 = 1;
    let err_prefix = "LIBFAULTINJ_ERROR_".to_string();
    let env_name = err_prefix + &String::from(funcname).to_uppercase() + "_ERRNO";

    match env::var(env_name) {
        Ok(p) => match p.parse::<i32>() {
            Ok(i) => i,
            Err(_) => DEFAULT_ERRNO,
        },
        Err(_) => DEFAULT_ERRNO,
    }
}

fn insert_delay(fd: c_int, funcname: &str) {
    use std::thread;

    let delay_match = DELAY_FDS.read().unwrap().contains(&fd);

    if delay_match {
        thread::sleep_ms(get_delay_amount_ms(funcname));
    }
}

use std::path::Path;

macro_rules! returnError(
        ($fd: expr, $funcname:expr, $err:expr) =>
    ({
        if ERR_FDS.read().unwrap().contains(&$fd) {
            set_errno(Errno(get_errno($funcname)));

            return $err;
        }
    })
);

macro_rules! injectFaults(
        ($fd: expr, $funcname:expr, $err:expr) =>
        {
            insert_delay($fd, $funcname);

            returnError!($fd, $funcname, $err);
        });

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

macro_rules! do_open(
    ($filename_:expr, $flags:expr, $mode:expr) =>
    ({
        let filename: String = unsafe {
            std::ffi::CStr::from_ptr($filename_).to_string_lossy().into_owned()
        };
        let open_func = get_libc_func!(OpenFunc, "open");
        let fd: c_int = open_func($filename_, $flags, $mode);
        const INVALID_FD: c_int = -1;

        if matchesPath!(filename, "LIBFAULTINJ_ERROR_PATH") {
            ERR_FDS.write().unwrap().insert(fd);
        }

        if matchesPath!(filename, "LIBFAULTINJ_DELAY_PATH") {
            DELAY_FDS.write().unwrap().insert(fd);
        }

        returnError!(fd, "open", INVALID_FD);

        fd
    })
    );

#[no_mangle]
pub extern "C" fn open64(filename_: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    do_open!(filename_, flags, mode)
}

#[no_mangle]
pub extern "C" fn open(filename_: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    do_open!(filename_, flags, mode)
}

#[no_mangle]
pub extern "C" fn creat(filename_: *const c_char, mode: libc::mode_t) -> c_int {
    let flags = 0;
    do_open!(filename_, flags, mode)
}

const SSIZE_ERR: ssize_t = -1i64;

#[no_mangle]
pub extern "C" fn read(fd: c_int, buf: *mut c_void, nbytes: c_int) -> ssize_t {
    let read_func = get_libc_func!(ReadFunc, "read");

    injectFaults!(fd, "read", SSIZE_ERR);

    let ret: ssize_t = read_func(fd, buf, nbytes);

    ret
}

#[no_mangle]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    const OFF_T_ERR: off_t = -1i64;

    let seek_func = get_libc_func!(SeekFunc, "lseek");

    injectFaults!(fd, "lseek", OFF_T_ERR);

    let ret: off_t = seek_func(fd, offset, whence);

    ret
}

#[no_mangle]
pub extern "C" fn write(fd: c_int, buf: *mut c_void, nbytes: c_int) -> ssize_t {
    let write_func = get_libc_func!(WriteFunc, "write");

    injectFaults!(fd, "write", SSIZE_ERR);

    let ret: ssize_t = write_func(fd, buf, nbytes);

    ret
}

// mmap() interception is disabled for now.  deadlocks on
//   malloc_init_hard()->mmap()->DynamicLibrary::open()->malloc_init_hard(), at least
//   on systems w/jemalloc.
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn mmap__DISABLED(addr: *mut c_void, length_: size_t, prot: c_int,
                                 flags: c_int, fd: c_int, offset: off_t) -> *mut c_void {
    use std::mem::transmute;

    let map_failed: * mut c_void = unsafe { transmute::<i64, *mut c_void>(-1) }; // FIXME only works on 64-bit?
    let mmap_func = get_libc_func!(MmapFunc, "mmap");

    injectFaults!(fd, "mmap", map_failed);

    let ret: *mut c_void = mmap_func(addr, length_, prot, flags, fd, offset);

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
