#![feature(dynamic_lib,path_relative_from,hashmap_hasher)]

extern crate libc;
extern crate errno;
extern crate rand;

#[macro_use]
extern crate lazy_static;


pub use libc::{c_char, c_int, c_void, off_t, size_t,mode_t};
pub use libc::types::os::arch::posix88::ssize_t;

#[macro_use]
mod errors;
use errors::{OpenFunc,ReadFunc,WriteFunc,SeekFunc,CloseFunc,MmapFunc,Dup2Func,ERR_FDS,DELAY_FDS,};
use errors::{remove_fd_if_present,add_fd_if_old_present,};

// These functions are designed to conform to their 
//  libc counterparts, but may instead inject errors
//  depending on conditions defined in various environment
//  variables.


#[no_mangle]
pub extern "C" fn open64(filename_: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    do_open!(filename_, flags, mode)
}

#[no_mangle]
pub extern "C" fn open(filename_: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    do_open!(filename_, flags, mode)
}

#[no_mangle]
pub extern "C" fn creat(filename_: *const c_char, mode: mode_t) -> c_int {
    let flags = libc::O_CREAT|libc::O_WRONLY|libc::O_TRUNC; // TODO: manpage says this is equivalent but 
                                                            //       should we just call creat() instead?
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
#[allow(private_no_mangle_fns)]
#[allow(dead_code)]
/* pub */ extern "C" fn mmap(addr: *mut c_void, length_: size_t, prot: c_int,
                                 flags: c_int, fd: c_int, offset: off_t) -> *mut c_void {
    let mmap_func = get_libc_func!(MmapFunc, "mmap");

    injectFaults!(fd, "mmap", libc::MAP_FAILED);

    let ret: *mut c_void = mmap_func(addr, length_, prot, flags, fd, offset);

    ret
}

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    let close_func = get_libc_func!(CloseFunc, "close");

    let ret: c_int = close_func(fd);

    remove_fd_if_present(fd);

    ret
}

// For now we don't intercept this call for error injection,
//   only for fd tracking.
#[no_mangle]
pub extern "C" fn dup2(oldfd: c_int, newfd: c_int) -> c_int {
    let dup2_func = get_libc_func!(Dup2Func, "dup2");

    add_fd_if_old_present(oldfd, newfd);

    dup2_func(oldfd, newfd)
}


