extern crate libc;

// use std::dynamic_lib::DynamicLibrary;
// const SYSTEM_C_LIBRARY: &'static str = "libc.so.6";
//  These wrappers might be effective, but they're not
//  permitted:
// unsafe impl Sync for DynamicLibrary { }
// unsafe impl Send for DynamicLibrary { }

use std::sync::RwLock;
use std::hash::Hasher;
use std::collections::hash_set::HashSet;
use std::hash::BuildHasher;

pub struct SomeHashState {
    // exists because on older linux systems w/o entropy
    //   syscall the default hash state will do open()
    //   which will recurse and deadlock on the RwLock
    //  resource.
    hash: u64,
}

impl BuildHasher for SomeHashState {
    type Hasher = SomeHashState;

    fn build_hasher(&self) -> SomeHashState {
        SomeHashState::default()
    }
}
impl Default for SomeHashState {
    #[inline]
    fn default() -> SomeHashState {
        SomeHashState { hash: 0 }
    }
}
impl Hasher for SomeHashState {
    fn write(&mut self, msg: &[u8]) {
        for byte in msg {
            self.hash ^= *byte as u64;
        }
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type AlternateHashSet = HashSet<c_int, SomeHashState>;

lazy_static! {
    pub static ref DELAY_FDS: RwLock<AlternateHashSet>
            = RwLock::new(HashSet::with_hasher(SomeHashState::default()));
    pub static ref ERR_FDS: RwLock<AlternateHashSet>
            = RwLock::new(HashSet::with_hasher(SomeHashState::default()));
// static ref LIBC: RwLock<DynamicLibrary>
// = RwLock::new(DynamicLibrary::open(Some(Path::new(SYSTEM_C_LIBRARY))).unwrap());
}


macro_rules! get_libc_func(
    ($destination_t:ty, $funcname:expr) =>
        (
            {
                use dylib::DynamicLibrary;
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





pub use libc::{c_char, c_int, c_ulong, c_void, off_t, size_t, mode_t, ssize_t, sockaddr,
               sockaddr_in};

#[allow(non_camel_case_types)]
pub type socklen_t = u8;

pub type OpenFunc = extern "C" fn(*const c_char, c_int, mode_t) -> c_int;
pub type ReadFunc = extern "C" fn(fd: c_int, buf: *mut c_void, nbytes: c_int) -> ssize_t;
pub type WriteFunc = ReadFunc;
pub type MmapFunc = extern "C" fn(addr: *mut c_void,
                                  length_: size_t,
                                  prot: c_int,
                                  flags: c_int,
                                  fd: c_int,
                                  offset: off_t)
                                  -> *mut c_void;
pub type CloseFunc = extern "C" fn(fd: c_int) -> c_int;
pub type IoctlFunc = extern "C" fn(c_int, c_ulong, ...) -> c_int;
pub type SeekFunc = extern "C" fn(c_int, off_t, c_int) -> off_t;
pub type Dup2Func = extern "C" fn(c_int, c_int) -> c_int;
pub type Dup3Func = extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type BindFunc = extern "C" fn(c_int, *const sockaddr, socklen_t) -> c_int;
pub type SocketFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type ConnectFunc = extern "C" fn(c_int, *const sockaddr, socklen_t) -> c_int;
pub type StatFunc = extern "C" fn(*const c_char, *mut libc::stat) -> c_int;
pub type FstatFunc = extern "C" fn(c_int, *const libc::stat) -> c_int;
pub type SendRecvFunc = extern "C" fn(c_int, *mut c_void, size_t, c_int) -> ssize_t;

macro_rules! get_delay_amount_ms(
        ($funcname: expr) =>
    ({
        use std::env;
        use std::time::Duration;

        let default_delay_amount : Duration = Duration::from_millis(200);;
        let err_prefix = "LIBFAULTINJ_DELAY_".to_string();
        let env_name = err_prefix + &String::from($funcname).to_uppercase() + "_MS";

        match env::var(env_name) {
            Ok(p) => match p.parse::<u64>() {
                Ok(i) => Duration::from_millis(i),
                Err(_) => default_delay_amount,
            },
            Err(_) => default_delay_amount,
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

pub static LIKELIHOOD_CERTAIN_PCT: f32 = 100f32;

macro_rules! returnError(
        ($fd: expr, $funcname:expr, $ret_err:expr) =>
    ({
        use errno::set_errno;
        use errors::get_rand_likelihood;
        use errors::get_item_likelihood;

        if ERR_FDS.read().unwrap().contains(&$fd) {
            let err_thresh = get_item_likelihood("LIBFAULTINJ_ERROR_LIKELIHOOD_PCT");

            if  get_rand_likelihood() < (err_thresh) {
                let errno = checkErrno!($funcname);

                if let Some(err) = errno {
                    set_errno(err);
                    return $ret_err;
                }

            };
        }
    })
);

pub fn get_item_likelihood(env_var: &'static str) -> f32 {
    use std::env;
    use errors::LIKELIHOOD_CERTAIN_PCT;

    match env::var(env_var) {
        Ok(p) => {
            match p.parse::<f32>() {
                Ok(i) => i,
                Err(_) => LIKELIHOOD_CERTAIN_PCT,
            }
        }
        Err(_) => LIKELIHOOD_CERTAIN_PCT,
    }
}

pub fn get_rand_likelihood() -> f32 {
    use rand;
    use rand::Rng;
    use errors::LIKELIHOOD_CERTAIN_PCT;

    let mut rng = rand::thread_rng();
    rng.gen_range::<f32>(0., LIKELIHOOD_CERTAIN_PCT)
}

macro_rules! injectFaults(
        ($fd: expr, $funcname:expr, $err:expr) =>
        ({
            use std::thread::sleep;
            use errors::get_item_likelihood;
            use errors::get_rand_likelihood;

            let delay_match = DELAY_FDS.read().unwrap().contains(&$fd);
            let delay_likelihood =  get_item_likelihood("LIBFAULTINJ_ERROR_LIKELIHOOD_PCT");

            if delay_match && (get_rand_likelihood() < delay_likelihood) {
                sleep(get_delay_amount_ms!($funcname));
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
                let delay_path_match = filename_path.strip_prefix(Path::new(&p)).is_ok();
                let filename_match = filename_path == Path::new(&p);

                delay_path_match || filename_match
            }
            Err(_) => false
        }
    }));

/**
 * @return true if $addr matches any of the addresses specified by
 *      std::env::var($env_name), false otherwise.
 */
pub unsafe fn matches_addr(addr: *const libc::sockaddr, env_name: &str) -> bool {
    use std::env;
    use std::net::ToSocketAddrs;
    use std::net::{SocketAddrV4, Ipv4Addr, IpAddr, SocketAddr};
    use std::net::lookup_host;

    let addr_ = {
        let ptr: *const libc::sockaddr = addr;
        *(ptr as *const libc::sockaddr_in)
    };

    let s_addr = addr_.sin_addr.s_addr;
    let ipv4_addr = SocketAddrV4::new(Ipv4Addr::new(((s_addr & 0x0ff000000) >> 24) as u8,
                                                    ((s_addr & 0x000ff0000) >> 16) as u8,
                                                    ((s_addr & 0x00000ff00) >> 8) as u8,
                                                    (s_addr & 0x0000000ff) as u8),
                                      addr_.sin_port);

    //  TODO: when we are called from the connect() intercept, this recurses
    // on itself:
    let socket_addr = ToSocketAddrs::to_socket_addrs(&ipv4_addr).unwrap();
    let addr_to_match = socket_addr.clone().next().unwrap();

    match env::var(env_name) {
        Ok(p) => {
            match lookup_host(p.as_ref()) {
                Ok(mut list) => list.any(|addr| addr.ip() == addr_to_match.ip()),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}




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
    use super::matches_addr;
    extern crate libc;

    #[test]
    fn test_addr() {
        use std::net::Ipv4Addr;
        use std::mem;

        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let sock_ = libc::sockaddr_in {
            sin_port: 0,
            sin_addr: libc::in_addr { s_addr: libc::in_addr_t::from(ip) },
            sin_family: libc::AF_INET as u16,
            sin_zero: [0 as u8; 8],
        };
        let sock =
            unsafe { mem::transmute::<*const libc::sockaddr_in, *const libc::sockaddr>(&sock_) };

        println!("sin_addr: {:?}", libc::in_addr_t::from(ip));

        env::set_var("TEST_ADDR", "127.0.0.1");
        assert!(unsafe { matches_addr(sock, "TEST_ADDR") });
    }

    #[test]
    fn test_delay() {
        use std::time::Duration;

        assert_eq!(get_delay_amount_ms!("open"), Duration::from_millis(200));

        assert_eq!(get_delay_amount_ms!("read"), Duration::from_millis(200));

        env::set_var("LIBFAULTINJ_DELAY_READ_MS", "99bogus");
        assert_eq!(get_delay_amount_ms!("read"), Duration::from_millis(200));

        env::set_var("LIBFAULTINJ_DELAY_READ_MS", "99");
        assert_eq!(get_delay_amount_ms!("read"), Duration::from_millis(99));
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
