#![doc(html_root_url = "https://docs.rs/mio/0.6.16")]
#![deny(missing_docs, missing_debug_implementations)]
#![cfg_attr(test, deny(warnings))]

//! A fast, low-level IO library for Rust focusing on non-blocking APIs, event
//! notification, and other useful utilities for building high performance IO
//! apps.
//!
//! # Goals
//!
//! * Fast - minimal overhead over the equivalent OS facilities (epoll, kqueue, etc...)
//! * Zero allocations
//! * A scalable readiness-based API, similar to epoll on Linux
//! * Design to allow for stack allocated buffers when possible (avoid double buffering).
//! * Provide utilities such as a timers, a notification channel, buffer abstractions, and a slab.
//!
//! # Platforms
//!
//! Currently supported platforms:
//!
//! * Linux
//! * OS X
//! * Windows
//! * FreeBSD
//! * NetBSD
//! * Android
//! * iOS
//!
//! mio can handle interfacing with each of the event notification systems of the aforementioned platforms. The details of
//! their implementation are further discussed in [`Poll`].
//!
//! # Usage
//!
//! Using mio starts by creating a [`Poll`], which reads events from the OS and
//! put them into [`Events`]. You can handle IO events from the OS with it.
//!
//! For more detail, see [`Poll`].
//!
//! [`Poll`]: struct.Poll.html
//! [`Events`]: struct.Events.html
//!
//! # Example
//!
//! ```
//! # extern crate mio;
//! # extern crate mio_uds_windows;
//! # extern crate tempdir;
//! # use tempdir::TempDir;
//! use mio::*;
//! use mio_uds_windows::{UnixListener, UnixStream};
//!
//! // Setup some tokens to allow us to identify which event is
//! // for which socket.
//! const SERVER: Token = Token(0);
//! const CLIENT: Token = Token(1);
//! 
//! let path = "/tmp/sock";
//! # let path = TempDir::new("uds").unwrap();
//! # let path = path.path().join("sock");
//!
//! // Setup the server socket
//! let server = UnixListener::bind(&path).unwrap();
//!
//! // Create a poll instance
//! let poll = Poll::new().unwrap();
//!
//! // Start listening for incoming connections
//! poll.register(&server, SERVER, Ready::readable(),
//!               PollOpt::edge()).unwrap();
//!
//! // Setup the client socket
//! let sock = UnixStream::connect(&path).unwrap();
//!
//! // Register the socket
//! poll.register(&sock, CLIENT, Ready::readable(),
//!               PollOpt::edge()).unwrap();
//!
//! // Create storage for events
//! let mut events = Events::with_capacity(1024);
//!
//! loop {
//!     poll.poll(&mut events, None).unwrap();
//!
//!     for event in events.iter() {
//!         match event.token() {
//!             SERVER => {
//!                 // Accept and drop the socket immediately, this will close
//!                 // the socket and notify the client of the EOF.
//!                 let _ = server.accept();
//!             }
//!             CLIENT => {
//!                 // The server just shuts down the socket, let's just exit
//!                 // from our event loop.
//!                 return;
//!             }
//!             _ => unreachable!(),
//!         }
//!     }
//! }
//!
//! ```

extern crate lazycell;
extern crate mio;
extern crate net2;
extern crate iovec;
extern crate slab;

#[cfg(windows)]
extern crate miow;

#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
extern crate ws2_32;

#[cfg(windows)]
extern crate kernel32;

#[macro_use]
extern crate log;

mod listener;
mod poll;
mod stream;
mod sys;

#[allow(missing_docs)]
pub mod net;

pub use listener::UnixListener;
pub use stream::UnixStream;
