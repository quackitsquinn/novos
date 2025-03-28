macro_rules! cfg_value {
    ($name: ident, $at_type: ty, $inner: ty, $default: expr, $store_type: path, $load_type: path, $get_prefix: ident, $set_prefix: ident) => {
        static $name: $at_type = <$at_type>::new($default);

        pastey::paste! {
            pub fn [<$set_prefix _ $name:lower>](value: $inner) {
                $name.store(value, $store_type);
            }
            pub fn [<$get_prefix _ $name:lower>]() -> $inner {
                $name.load($load_type)
            }
        }
    };

    ($name: ident, $at_type: ty, $inner: ty, $default: expr) => {
        cfg_value!(
            $name,
            $at_type,
            $inner,
            $default,
            ::core::sync::atomic::Ordering::Relaxed,
            ::core::sync::atomic::Ordering::Relaxed,
            get,
            set
        );
    };

    ($name: ident, $at_type: ty, $inner: ty, $default: expr, set: $set_prefix: ident, get: $get_prefix: ident) => {
        cfg_value!(
            $name,
            $at_type,
            $inner,
            $default,
            ::core::sync::atomic::Ordering::Relaxed,
            ::core::sync::atomic::Ordering::Relaxed,
            $get_prefix,
            $set_prefix
        );
    };
}
use core::sync::atomic::AtomicBool;

pub(crate) use cfg_value;
