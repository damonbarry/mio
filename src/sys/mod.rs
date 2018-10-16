#[cfg(windows)]
pub use self::windows::{
    Events,
    Selector,
    TcpStream,
    TcpListener,
    Overlapped,
    Binding,
};

#[cfg(windows)]
mod windows;

#[cfg(not(all(unix, not(target_os = "fuchsia"))))]
#[allow(dead_code)]
pub const READY_ALL: usize = 0;
