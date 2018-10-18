use mio::{Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use std::io;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Used to associate an IO type with a Selector
#[derive(Debug)]
pub struct SelectorId {
    id: AtomicUsize,
}

// TODO: get rid of this, windows depends on it for now
#[allow(dead_code)]
pub fn new_registration(poll: &Poll, token: Token, ready: Ready, opt: PollOpt)
        -> (Registration, SetReadiness)
{
    #[allow(deprecated)]
    Registration::new(poll, token, ready, opt)
}

pub mod skinny {
    use std::mem;
    use mio;

    pub mod sys {
        use std::mem;
        use std::sync::{Arc, Mutex};
        use lazycell::AtomicLazyCell;
        use mio::windows::Binding as MiowBinding;
        use miow::iocp::CompletionPort;

        struct Binding {
            selector: AtomicLazyCell<Arc<SelectorInner>>,
        }

        struct BufferPool {
            pool: Vec<Vec<u8>>,
        }

        impl BufferPool {
            #[allow(dead_code)]
            pub fn new(cap: usize) -> BufferPool {
                BufferPool { pool: Vec::with_capacity(cap) }
            }

            pub fn get(&mut self, default_cap: usize) -> Vec<u8> {
                self.pool.pop().unwrap_or_else(|| Vec::with_capacity(default_cap))
            }

            pub fn put(&mut self, mut buf: Vec<u8>) {
                if self.pool.len() < self.pool.capacity(){
                    unsafe { buf.set_len(0); }
                    self.pool.push(buf);
                }
            }
        }

        pub struct Selector {
            inner: Arc<SelectorInner>,
        }

        #[allow(dead_code)]
        struct SelectorInner {
            /// Unique identifier of the `Selector`
            id: usize,

            /// The actual completion port that's used to manage all I/O
            port: CompletionPort,

            /// A pool of buffers usable by this selector.
            ///
            /// Primitives will take buffers from this pool to perform I/O operations,
            /// and once complete they'll be put back in.
            buffers: Mutex<BufferPool>,
        }

        impl Selector {
            /// Return the `Selector`'s identifier
            pub fn id(&self) -> usize {
                self.inner.id
            }
        }

        fn as_binding(binding: &MiowBinding) -> &Binding {
            unsafe { mem::transmute(&binding as *const _ as * const _) }
        }

        pub fn get_buffer(binding: &MiowBinding, size: usize) -> Vec<u8> {
            match as_binding(binding).selector.borrow() {
                Some(i) => i.buffers.lock().unwrap().get(size),
                None => Vec::with_capacity(size),
            }
        }

        pub fn put_buffer(binding: &MiowBinding, buf: Vec<u8>) {
            if let Some(i) = as_binding(binding).selector.borrow() {
                i.buffers.lock().unwrap().put(buf);
            }
        }
    }

    struct Poll {
        // Platform specific IO selector
        selector: sys::Selector,
    }

    fn as_poll(poll: &mio::Poll) -> &Poll {
        unsafe { mem::transmute(&poll as *const _ as *const _) }
    }

    pub fn selector_id(poll: &mio::Poll) -> usize {
        as_poll(poll).selector.id()
    }

    pub fn get_buffer(binding: &mio::windows::Binding, size: usize) -> Vec<u8> {
        sys::get_buffer(binding, size)
    }

    pub fn put_buffer(binding: &mio::windows::Binding, buf: Vec<u8>) {
        sys::put_buffer(binding, buf)
    }
}

impl SelectorId {
    pub fn new() -> SelectorId {
        SelectorId {
            id: AtomicUsize::new(0),
        }
    }

    pub fn associate_selector(&self, poll: &Poll) -> io::Result<()> {
        let selector_id = self.id.load(Ordering::SeqCst);
        let poll_id = skinny::selector_id(poll);

        if selector_id != 0 && selector_id != poll_id {
            Err(io::Error::new(io::ErrorKind::Other, "socket already registered"))
        } else {
            self.id.store(poll_id, Ordering::SeqCst);
            Ok(())
        }
    }
}

impl Clone for SelectorId {
    fn clone(&self) -> SelectorId {
        SelectorId {
            id: AtomicUsize::new(self.id.load(Ordering::SeqCst)),
        }
    }
}

