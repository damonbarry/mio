use mio::{Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use {sys};
use std::io;
use std::mem;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Used to associate an IO type with a Selector
#[derive(Debug)]
pub struct SelectorId {
    id: AtomicUsize,
}


// ===== Accessors for internal usage =====

pub fn selector(poll: &Poll) -> &sys::Selector {
    // unsavory conversion from Poll's selector to our internal selector
    // (which is the same code, but from a different build)
    unsafe { mem::transmute(&poll as *const _ as *const _) }
}

/*
 *
 * ===== Registration =====
 *
 */

// TODO: get rid of this, windows depends on it for now
#[allow(dead_code)]
pub fn new_registration(poll: &Poll, token: Token, ready: Ready, opt: PollOpt)
        -> (Registration, SetReadiness)
{
    #[allow(deprecated)]
    Registration::new(poll, token, ready, opt)
}

impl SelectorId {
    pub fn new() -> SelectorId {
        SelectorId {
            id: AtomicUsize::new(0),
        }
    }

    pub fn associate_selector(&self, poll: &Poll) -> io::Result<()> {
        let selector_id = self.id.load(Ordering::SeqCst);

        if selector_id != 0 && selector_id != selector(poll).id() {
            Err(io::Error::new(io::ErrorKind::Other, "socket already registered"))
        } else {
            self.id.store(selector(poll).id(), Ordering::SeqCst);
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

