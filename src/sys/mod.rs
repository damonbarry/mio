#[cfg(windows)]
pub use self::windows::{
    UnixStream,
    UnixListener,
};

#[cfg(windows)]
mod windows;

#[allow(dead_code)]
pub const READY_ALL: usize = 0;
