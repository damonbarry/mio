#[cfg(windows)]
pub use self::windows::{
    Events,
    Selector,
    UnixStream,
    UnixListener,
    Overlapped,
    Binding,
};

#[cfg(windows)]
mod windows;

#[allow(dead_code)]
pub const READY_ALL: usize = 0;
