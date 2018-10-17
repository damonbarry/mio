use std::fmt;
use std::io;
use std::path::Path;

use mio::{Evented, Ready, Poll, PollOpt, Token};

use net::{self, SocketAddr};
use poll::SelectorId;
use stream::UnixStream;
use sys;

/// A structure representing a socket server
///
/// # Examples
///
/// ```
/// # extern crate mio;
/// # extern crate mio_uds_windows;
/// # use std::error::Error;
/// # fn try_main() -> Result<(), Box<Error>> {
/// use mio::{Events, Ready, Poll, PollOpt, Token};
/// use mio_uds_windows::UnixListener;
/// use std::time::Duration;
///
/// let listener = UnixListener::bind("/tmp/sock")?;
///
/// let poll = Poll::new()?;
/// let mut events = Events::with_capacity(128);
///
/// // Register the socket with `Poll`
/// poll.register(&listener, Token(0), Ready::writable(),
///               PollOpt::edge())?;
///
/// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
///
/// // There may be a socket ready to be accepted
/// #     Ok(())
/// # }
/// #
/// # fn main() {
/// #     try_main().unwrap();
/// # }
/// ```
pub struct UnixListener {
    sys: sys::UnixListener,
    selector_id: SelectorId,
}

impl UnixListener {
    /// Convenience method to bind a new `UnixListener` to the specified path
    /// to receive new connections.
    ///
    /// This function will take the following steps:
    ///
    /// 1. Create a new Unix domain socket.
    /// 2. Bind the socket to the specified path.
    /// 3. Call `listen` on the socket to prepare it to receive new connections.
    ///
    /// If fine-grained control over the binding and listening process for a
    /// socket is desired, create an instance of `net::UnixListener` (possibly
    /// via an instance of `net::Socket`) and use the `UnixListener::from_std`
    /// method to transfer it into mio.
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
        let sock = net::UnixListener::bind(path)?;
        UnixListener::from_std(sock)
    }

    /// Creates a new `UnixListener` from an instance of a
    /// `net::UnixListener` type.
    ///
    /// This function will set the `listener` provided into nonblocking mode on
    /// Unix, and otherwise the stream will just be wrapped up in an mio stream
    /// ready to accept new connections and become associated with an event
    /// loop.
    ///
    /// The address provided must be the address that the listener is bound to.
    pub fn from_std(listener: net::UnixListener) -> io::Result<UnixListener> {
        sys::UnixListener::new(listener).map(|s| {
            UnixListener {
                sys: s,
                selector_id: SelectorId::new(),
            }
        })
    }

    /// Accepts a new `UnixStream`.
    ///
    /// This may return an `Err(e)` where `e.kind()` is
    /// `io::ErrorKind::WouldBlock`. This means a stream may be ready at a later
    /// point and one should wait for a notification before calling `accept`
    /// again.
    ///
    /// If an accepted stream is returned, the remote address of the peer is
    /// returned along with it.
    pub fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
        let (s, a) = try!(self.accept_std());
        Ok((UnixStream::from_stream(s)?, a))
    }

    /// Accepts a new `net::UnixStream`.
    ///
    /// This method is the same as `accept`, except that it returns a socket
    /// *in blocking mode* which isn't bound to `mio`. This can be later then
    /// converted to a `mio` type, if necessary.
    pub fn accept_std(&self) -> io::Result<(net::UnixStream, SocketAddr)> {
        self.sys.accept()
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.sys.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// The returned `UnixListener` is a reference to the same socket that this
    /// object references. Both handles can be used to accept incoming
    /// connections and options set on one listener will affect the other.
    pub fn try_clone(&self) -> io::Result<UnixListener> {
        self.sys.try_clone().map(|s| {
            UnixListener {
                sys: s,
                selector_id: self.selector_id.clone(),
            }
        })
    }

    /// Get the value of the `SO_ERROR` option on this socket.
    ///
    /// This will retrieve the stored error in the underlying socket, clearing
    /// the field in the process. This can be useful for checking errors between
    /// calls.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.sys.take_error()
    }
}

impl Evented for UnixListener {
    fn register(&self, poll: &Poll, token: Token,
                interest: Ready, opts: PollOpt) -> io::Result<()> {
        self.selector_id.associate_selector(poll)?;
        self.sys.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token,
                  interest: Ready, opts: PollOpt) -> io::Result<()> {
        self.sys.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.sys.deregister(poll)
    }
}

impl fmt::Debug for UnixListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.sys, f)
    }
}
