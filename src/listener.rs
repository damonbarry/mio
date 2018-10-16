use std::fmt;
use std::io;
use std::net::{self, SocketAddr};

use mio::{Evented, Ready, Poll, PollOpt, Token};
use net2::TcpBuilder;

use poll::SelectorId;
use stream::UnixStream;
use sys;

/*
 *
 * ===== UnixListener =====
 *
 */

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
/// let listener = UnixListener::bind(&"127.0.0.1:34255".parse()?)?;
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
    /// Convenience method to bind a new TCP listener to the specified address
    /// to receive new connections.
    ///
    /// This function will take the following steps:
    ///
    /// 1. Create a new TCP socket.
    /// 2. Set the `SO_REUSEADDR` option on the socket.
    /// 3. Bind the socket to the specified address.
    /// 4. Call `listen` on the socket to prepare it to receive new connections.
    ///
    /// If fine-grained control over the binding and listening process for a
    /// socket is desired then the `net2::TcpBuilder` methods can be used in
    /// combination with the `UnixListener::from_listener` method to transfer
    /// ownership into mio.
    pub fn bind(addr: &SocketAddr) -> io::Result<UnixListener> {
        // Create the socket
        let sock = match *addr {
            SocketAddr::V4(..) => TcpBuilder::new_v4(),
            SocketAddr::V6(..) => TcpBuilder::new_v6(),
        }?;

        // Set SO_REUSEADDR, but only on Unix (mirrors what libstd does)
        if cfg!(unix) {
            sock.reuse_address(true)?;
        }

        // Bind the socket
        sock.bind(addr)?;

        // listen
        let listener = sock.listen(1024)?;
        Ok(UnixListener {
            sys: sys::UnixListener::new(listener)?,
            selector_id: SelectorId::new(),
        })
    }

    #[deprecated(since = "0.6.13", note = "use from_std instead")]
    #[cfg(feature = "with-deprecated")]
    #[doc(hidden)]
    pub fn from_listener(listener: net::TcpListener, _: &SocketAddr)
                         -> io::Result<UnixListener> {
        UnixListener::from_std(listener)
    }

    /// Creates a new `UnixListener` from an instance of a
    /// `std::net::TcpListener` type.
    ///
    /// This function will set the `listener` provided into nonblocking mode on
    /// Unix, and otherwise the stream will just be wrapped up in an mio stream
    /// ready to accept new connections and become associated with an event
    /// loop.
    ///
    /// The address provided must be the address that the listener is bound to.
    pub fn from_std(listener: net::TcpListener) -> io::Result<UnixListener> {
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

    /// Accepts a new `std::net::TcpStream`.
    ///
    /// This method is the same as `accept`, except that it returns a TCP socket
    /// *in blocking mode* which isn't bound to `mio`. This can be later then
    /// converted to a `mio` type, if necessary.
    pub fn accept_std(&self) -> io::Result<(net::TcpStream, SocketAddr)> {
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

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.sys.set_ttl(ttl)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`][link].
    ///
    /// [link]: #method.set_ttl
    pub fn ttl(&self) -> io::Result<u32> {
        self.sys.ttl()
    }

    /// Sets the value for the `IPV6_V6ONLY` option on this socket.
    ///
    /// If this is set to `true` then the socket is restricted to sending and
    /// receiving IPv6 packets only. In this case two IPv4 and IPv6 applications
    /// can bind the same port at the same time.
    ///
    /// If this is set to `false` then the socket can be used to send and
    /// receive packets from an IPv4-mapped IPv6 address.
    pub fn set_only_v6(&self, only_v6: bool) -> io::Result<()> {
        self.sys.set_only_v6(only_v6)
    }

    /// Gets the value of the `IPV6_V6ONLY` option for this socket.
    ///
    /// For more information about this option, see [`set_only_v6`][link].
    ///
    /// [link]: #method.set_only_v6
    pub fn only_v6(&self) -> io::Result<bool> {
        self.sys.only_v6()
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
