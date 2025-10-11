macro_rules! aerror {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            cake::log::error!($($arg)*);
        }
    };
}

macro_rules! awarn {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            cake::log::warn!($($arg)*);
        }
    };
}

macro_rules! ainfo {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            cake::log::info!($($arg)*);
        }
    };
}

macro_rules! adebug {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            cake::log::debug!($($arg)*);
        }
    };
}

macro_rules! atrace {
    ($($arg:tt)*) => {
        if $crate::should_log() {
            cake::log::trace!($($arg)*);
        }
    };
}
