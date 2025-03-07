macro_rules! aerror {
    ($($arg:tt)*) => {
        if $crate::ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed) {
            log::error!($($arg)*);
        }
    };
}

macro_rules! awarn {
    ($($arg:tt)*) => {
        if $crate::ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed) {
            log::warn!($($arg)*);
        }
    };
}

macro_rules! ainfo {
    ($($arg:tt)*) => {
        if $crate::ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed) {
            log::info!($($arg)*);
        }
    };
}

macro_rules! adebug {
    ($($arg:tt)*) => {
        if $crate::ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!($($arg)*);
        }
    };
}

macro_rules! atrace {
    ($($arg:tt)*) => {
        if $crate::ALLOC_LOG.load(core::sync::atomic::Ordering::Relaxed) {
            log::trace!($($arg)*);
        }
    };
}
