macro_rules! aerror {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            log::error!($($arg)*);
        }
    };
}

macro_rules! awarn {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            log::warn!($($arg)*);
        }
    };
}

macro_rules! ainfo {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            log::info!($($arg)*);
        }
    };
}

macro_rules! adebug {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            log::debug!($($arg)*);
        }
    };
}

macro_rules! atrace {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            log::trace!($($arg)*);
        }
    };
}
