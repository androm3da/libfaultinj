

//use std::dynamic_lib::DynamicLibrary;
//const SYSTEM_C_LIBRARY: &'static str = "libc.so.6";
//unsafe impl Sync for DynamicLibrary { }
//unsafe impl Send for DynamicLibrary { }

use std::sync::RwLock;
use std::hash::{Hasher, SipHasher};
use std::collections::hash_set::HashSet;
use std::collections::hash_state::DefaultState;
lazy_static! {
    pub static ref DELAY_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
    pub static ref ERR_FDS: RwLock<HashSet<c_int, DefaultState<SipHasher>>>
                                                = RwLock::new(Default::default());
    //static ref LIBC: RwLock<DynamicLibrary>
        // = RwLock::new(DynamicLibrary::open(Some(Path::new(SYSTEM_C_LIBRARY))).unwrap());
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





pub use libc::{c_char, c_int, c_ulong, c_void, off_t, size_t, mode_t};
pub use libc::types::os::arch::posix88::ssize_t;

pub type OpenFunc = extern "C" fn(* const c_char, c_int, mode_t) -> c_int;
pub type ReadFunc = extern "C" fn(fd: c_int, buf: * mut c_void, nbytes: c_int) -> ssize_t;
pub type WriteFunc = ReadFunc;
pub type MmapFunc = extern "C" fn(addr: *mut c_void, length_: size_t, prot: c_int,
                   flags: c_int, fd: c_int, offset: off_t) -> *mut c_void;
pub type CloseFunc = extern "C" fn(fd: c_int) -> c_int;
pub type IoctlFunc = extern "C" fn(c_int, c_ulong, ...) -> c_int;
pub type SeekFunc = extern "C" fn(c_int, off_t, c_int) -> off_t;
pub type Dup2Func = extern "C" fn(c_int, c_int) -> c_int;
pub type Dup3Func = extern "C" fn(c_int, c_int, c_int) -> c_int;

macro_rules! get_delay_amount_ms(
        ($funcname: expr) =>
    ({
        use std::env;

        const DEFAULT_DELAY_AMOUNT_MS: u32 = 200;
        let err_prefix = "LIBFAULTINJ_DELAY_".to_string();
        let env_name = err_prefix + &String::from($funcname).to_uppercase() + "_MS";

        match env::var(env_name) {
            Ok(p) => match p.parse::<u32>() {
                Ok(i) => i,
                Err(_) => DEFAULT_DELAY_AMOUNT_MS,
            },
            Err(_) => DEFAULT_DELAY_AMOUNT_MS,
        }
    })
);

macro_rules! checkErrno(
    ($funcname: expr) =>
    ({
        use errno::Errno;
        use std::string::String;
        use std::env;

        let err_prefix = "LIBFAULTINJ_ERROR_".to_string();
        let env_name = err_prefix + &String::from($funcname).to_uppercase() + "_ERRNO";

        match env::var(env_name) {
            Ok(p) => match p.parse::<i32>() {
                Ok(i) => Some(Errno(i)),
                Err(_) => None,
            },
            Err(_) => None,
        }
    })
);


macro_rules! returnError(
        ($fd: expr, $funcname:expr, $ret_err:expr) =>
    ({
        use rand::Rng;
        use errno::set_errno;

        const LIKELIHOOD_CERTAIN_PCT: f32 = 100f32;

        if ERR_FDS.read().unwrap().contains(&$fd) {
            let err_thresh = match std::env::var("LIBFAULTINJ_ERROR_LIKELIHOOD_PCT") {
                Ok(p) => match p.parse::<f32>() {
                    Ok(i) => i,
                    Err(_) => LIKELIHOOD_CERTAIN_PCT,
                },
                Err(_) => LIKELIHOOD_CERTAIN_PCT,
            };
            let mut rng = rand::thread_rng();
            let rand_val = rng.gen_range::<f32>(0., LIKELIHOOD_CERTAIN_PCT);

            if  rand_val < (err_thresh) {
                let errno = checkErrno!($funcname);

                if let Some(err) = errno {
                    set_errno(err);
                    return $ret_err;
                }

            };
        }
    })
);

macro_rules! injectFaults(
        ($fd: expr, $funcname:expr, $err:expr) =>
        ({
            use std::thread;

            let delay_match = DELAY_FDS.read().unwrap().contains(&$fd);

            if delay_match {
                thread::sleep_ms(get_delay_amount_ms!($funcname));
            }

            returnError!($fd, $funcname, $err);
        }));

/**
 * @return true if $filename is located within the path specified by
 *      std::env::var($env_name), false otherwise.
 */
macro_rules! matchesPath(
        ($filename: expr, $env_name: expr) =>
    (
    {
        use std::path::Path;
        use std::env;

        match env::var($env_name) {
            Ok(p) => {
                let filename_path = Path::new(&$filename);
                let delay_path_match = (filename_path.relative_from(Path::new(&p))) != None;
                let filename_match = filename_path == Path::new(&p);

                delay_path_match || filename_match
            }
            Err(_) => false
        }
    }));

macro_rules! do_open(
    ($filename_:expr, $flags:expr, $mode:expr) =>
    ({
        let filename: String = unsafe {
            std::ffi::CStr::from_ptr($filename_).to_string_lossy().into_owned()
        };
        let open_func = get_libc_func!(OpenFunc, "open");
        let fd: c_int = open_func($filename_, $flags, $mode);

        if matchesPath!(filename, "LIBFAULTINJ_ERROR_PATH") {
            ERR_FDS.write().unwrap().insert(fd);
        }

        if matchesPath!(filename, "LIBFAULTINJ_DELAY_PATH") {
            DELAY_FDS.write().unwrap().insert(fd);
        }

        const INVALID_FD: c_int = -1;
        returnError!(fd, "open", INVALID_FD);

        fd
    })
    );


pub fn remove_fd_if_present(fd: c_int) {
    let mut err_fds = ERR_FDS.write().unwrap();
    if err_fds.contains(&fd) {
        err_fds.remove(&fd);
    }

    let mut delay_fds = DELAY_FDS.write().unwrap();
    if delay_fds.contains(&fd) {
        delay_fds.remove(&fd);
    }

}

pub fn add_fd_if_old_present(oldfd: c_int, newfd: c_int) {
    let mut err_fds = ERR_FDS.write().unwrap();
    if err_fds.contains(&oldfd) {
        err_fds.insert(newfd);
    }

    let mut delay_fds = DELAY_FDS.write().unwrap();
    if delay_fds.contains(&oldfd) {
        delay_fds.insert(newfd);
    }

}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::Path;

    #[test]
    fn test_delay() {

        assert_eq!(get_delay_amount_ms!("open"), 200);

        assert_eq!(get_delay_amount_ms!("read"), 200);

        env::set_var("LIBFAULTINJ_DELAY_READ_MS", "99bogus");
        assert_eq!(get_delay_amount_ms!("read"), 200);

        env::set_var("LIBFAULTINJ_DELAY_READ_MS", "99");
        assert_eq!(get_delay_amount_ms!("read"), 99);
    }

    #[test]
    fn test_path() {
        let base = ".";
        let base_path = Path::new(&base);
        assert!(!matchesPath!(base, "LIBFAULTINJ_ERROR_PATH"));

        env::set_var("LIBFAULTINJ_ERROR_PATH", ".");
        assert!(matchesPath!(base, "LIBFAULTINJ_ERROR_PATH"));

        let foo_path = base_path.join("foo");
        assert!(matchesPath!(foo_path, "LIBFAULTINJ_ERROR_PATH"));


        let bar_path = base_path.join("bar");
        env::set_var("LIBFAULTINJ_ERROR_PATH", bar_path.to_str().unwrap());

        assert!(!matchesPath!(foo_path, "LIBFAULTINJ_ERROR_PATH"));

        let bar_x_path = bar_path.join("x");
        assert!(matchesPath!(bar_x_path, "LIBFAULTINJ_ERROR_PATH"));

        let bard_path = base_path.join("bard");
        assert!(!matchesPath!(bard_path, "LIBFAULTINJ_ERROR_PATH"));

        let bard_x_path = bard_path.join("x");
        assert!(!matchesPath!(bard_x_path, "LIBFAULTINJ_ERROR_PATH"));

        env::set_var("LIBFAULTINJ_ERROR_PATH", bard_x_path.to_str().unwrap());
        assert!(matchesPath!(bard_x_path, "LIBFAULTINJ_ERROR_PATH"));
    }
}
