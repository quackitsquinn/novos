pub(super) const ALLOC_DEBUG: bool = option_env!("ALLOC_DEBUG").is_some();

macro_rules! alloc_debug {
    ($($arg:tt)*) => {
        if $crate::memory::allocator::log::ALLOC_DEBUG {
            log::debug!($($arg)*);
        }
    };
}

macro_rules! alloc_trace {
    ($($arg:tt)*) => {
        if $crate::memory::allocator::log::ALLOC_DEBUG {
            log::trace!($($arg)*);
        }
    };
}

macro_rules! alloc_info {
    ($($arg:tt)*) => {
        if $crate::memory::allocator::log::ALLOC_DEBUG {
            log::info!($($arg)*);
        }
    };
}

macro_rules! alloc_warn {
    ($($arg:tt)*) => {
        if $crate::memory::allocator::log::ALLOC_DEBUG {
            log::warn!($($arg)*);
        }
    };
}

macro_rules! alloc_error {
    ($($arg:tt)*) => {
        if $crate::memory::allocator::log::ALLOC_DEBUG {
            log::error!($($arg)*);
        }
    };
}

pub(super) use {alloc_debug, alloc_error, alloc_info, alloc_trace, alloc_warn};
