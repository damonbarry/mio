use std::fmt;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::path::Path;

use iovec::IoVec;
use mio::{Evented, Ready, Poll, PollOpt, Token};

use net::{self, SocketAddr};
use poll::SelectorId;
use sys;

/// A non-blocking TCP stream between a local socket and a remote socket.
///
/// The socket will be closed when the value is dropped.
///
/// # Examples
///
/// ```
/// # extern crate mio;
/// # extern crate mio_uds_windows;
/// # use net::UnixListener;
/// # use std::error::Error;
/// #
/// # fn try_main() -> Result<(), Box<Error>> {
/// # let _listener = UnixListener::bind("/tmp/sock")?;
/// use mio::{Events, Ready, Poll, PollOpt, Token};
/// use mio_uds_windows::UnixStream;
/// use std::time::Duration;
///
/// let stream = UnixStream::connect("/tmp/sock".parse()?)?;
///
/// let poll = Poll::new()?;
/// let mut events = Events::with_capacity(128);
///
/// // Register the socket with `Poll`
/// poll.register(&stream, Token(0), Ready::writable(),
///               PollOpt::edge())?;
///
/// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
///
/// // The socket might be ready at this point
/// #     Ok(())
/// # }
/// #
/// # fn main() {
/// #     try_main().unwrap();
/// # }
/// ```
pub struct UnixStream {
    sys: sys::UnixStream,
    selector_id: SelectorId,
}

fn set_nonblocking(stream: &net::UnixStream) -> io::Result<()> {
    stream.set_nonblocking(true)
}


impl UnixStream {
    /// Create a new UDS stream and issue a non-blocking connect to the
    /// specified path.
    ///
    /// This convenience method is available and uses the system's default
    /// options when creating a socket which is then connected. If fine-grained
    /// control over the creation of the socket is desired, you can use
    /// `net::Socket` and/or `net::UnixStream` to configure a socket and then
    /// pass it to `UnixStream::connect_stream` to transfer ownership into mio
    /// and schedule the connect operation.
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        fn inner(path: &Path) -> io::Result<UnixStream> {
            let sock = net::UnixStream::new()?;
            let addr = SocketAddr::from_path(path)?;
            UnixStream::connect_stream(sock, &addr)
        }
        inner(path.as_ref())
    }

    /// Creates a new `UnixStream` from the pending socket inside the given
    /// `net::UnixStream`, connecting it to the address path.
    ///
    /// This constructor allows configuring the socket before it's actually
    /// connected, and this function will transfer ownership to the returned
    /// `UnixStream` if successful. An unconnected `UnixStream` can be created
    /// with the `net::UnixStream` type (and also configured via that route).
    ///
    /// The platform specific behavior of this function looks like:
    ///
    /// * On Windows, the path is stored internally and the connect operation
    ///   is issued when the returned `UnixStream` is registered with an event
    ///   loop. Note that on Windows you must `bind` a socket before it can be
    ///   connected, so `stream` must be bound before this method is called.
    pub fn connect_stream(stream: net::UnixStream,
                          addr: &SocketAddr) -> io::Result<UnixStream> {
        Ok(UnixStream {
            sys: sys::UnixStream::connect(stream, addr)?,
            selector_id: SelectorId::new(),
        })
    }

    /// Creates a new `UnixStream` from a `net::UnixStream`.
    ///
    /// This function is intended to be used to wrap a `net::UnixStream` in the
    /// mio equivalent. The conversion here will automatically set `stream` to
    /// nonblocking and the returned object should be ready to get associated
    /// with an event loop.
    ///
    /// Note that the UDS stream here will not have `connect` called on it, so
    /// it should already be connected via some other means.
    pub fn from_stream(stream: net::UnixStream) -> io::Result<UnixStream> {
        set_nonblocking(&stream)?;

        Ok(UnixStream {
            sys: sys::UnixStream::from_stream(stream),
            selector_id: SelectorId::new(),
        })
    }

    /// Returns the socket address of the remote peer of this connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.sys.peer_addr()
    }

    /// Returns the socket address of the local half of this connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.sys.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// The returned `UnixStream` is a reference to the same stream that this
    /// object references. Both handles will read and write the same stream of
    /// data, and options set on one stream will be propagated to the other
    /// stream.
    pub fn try_clone(&self) -> io::Result<UnixStream> {
        self.sys.try_clone().map(|s| {
            UnixStream {
                sys: s,
                selector_id: self.selector_id.clone(),
            }
        })
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value (see the
    /// documentation of `Shutdown`).
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.sys.shutdown(how)
    }

    /// Get the value of the `SO_ERROR` option on this socket.
    ///
    /// This will retrieve the stored error in the underlying socket, clearing
    /// the field in the process. This can be useful for checking errors between
    /// calls.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.sys.take_error()
    }

    /// Read in a list of buffers all at once.
    ///
    /// This operation will attempt to read bytes from this socket and place
    /// them into the list of buffers provided. Note that each buffer is an
    /// `IoVec` which can be created from a byte slice.
    ///
    /// The buffers provided will be filled in sequentially. A buffer will be
    /// entirely filled up before the next is written to.
    ///
    /// The number of bytes read is returned, if successful, or an error is
    /// returned otherwise. If no bytes are available to be read yet then
    /// a "would block" error is returned. This operation does not block.
    ///
    /// On Unix this corresponds to the `readv` syscall.
    pub fn read_bufs(&self, bufs: &mut [&mut IoVec]) -> io::Result<usize> {
        self.sys.readv(bufs)
    }

    /// Write a list of buffers all at once.
    ///
    /// This operation will attempt to write a list of byte buffers to this
    /// socket. Note that each buffer is an `IoVec` which can be created from a
    /// byte slice.
    ///
    /// The buffers provided will be written sequentially. A buffer will be
    /// entirely written before the next is written.
    ///
    /// The number of bytes written is returned, if successful, or an error is
    /// returned otherwise. If the socket is not currently writable then a
    /// "would block" error is returned. This operation does not block.
    ///
    /// On Unix this corresponds to the `writev` syscall.
    pub fn write_bufs(&self, bufs: &[&IoVec]) -> io::Result<usize> {
        self.sys.writev(bufs)
    }
}

impl Read for UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.sys).read(buf)
    }
}

impl<'a> Read for &'a UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.sys).read(buf)
    }
}

impl Write for UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.sys).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.sys).flush()
    }
}

impl<'a> Write for &'a UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.sys).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.sys).flush()
    }
}

impl Evented for UnixStream {
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

impl fmt::Debug for UnixStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.sys, f)
    }
}
