use core::{
    fmt::Debug,
    sync::atomic::{AtomicBool, AtomicU8},
};

const FUSE_INTACT: u8 = 0;
const FUSE_BLOWING: u8 = 1;
const FUSE_BLOWN: u8 = 2;

/// A fuse that can be "blown" exactly once.
///
/// This is useful as a initialization primitive to ensure that things are initialized once.
pub struct Fuse {
    /// Whether the fuse has been blown.
    /// 0 = intact, 1 = blowing, 2 = blown
    is_blown: AtomicU8,
}

impl Fuse {
    /// Creates a new fuse.
    pub const fn new() -> Self {
        Self {
            is_blown: AtomicU8::new(FUSE_INTACT),
        }
    }

    /// Returns true if the fuse has been blown.
    pub fn is_blown(&self) -> bool {
        self.is_blown.load(core::sync::atomic::Ordering::SeqCst) == FUSE_BLOWN
    }

    /// "Blows" the fuse. Once blown, it cannot be reset.
    pub fn blow(&self) {
        self.is_blown
            .store(FUSE_BLOWN, core::sync::atomic::Ordering::SeqCst);
    }

    /// Runs the given closure and "blows" the fuse.
    /// If the fuse is already blown, the closure will not be run.
    pub fn blow_once<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        if self
            .is_blown
            .compare_exchange(
                FUSE_INTACT,
                FUSE_BLOWING,
                core::sync::atomic::Ordering::SeqCst,
                core::sync::atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            let ret = Some(f());
            self.is_blown
                .store(FUSE_BLOWN, core::sync::atomic::Ordering::SeqCst);
            ret
        } else {
            None
        }
    }
}

impl Debug for Fuse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut fmt = f.debug_tuple("Fuse");
        if self.is_blown() {
            fmt.field_with(|f| write!(f, "blown"));
        } else {
            fmt.field_with(|f| write!(f, "intact"));
        }
        fmt.finish()
    }
}
