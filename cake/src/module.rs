use core::fmt::Debug;

use log::info;
use spin::Once;

/// A kernel module that can be initialized once. If the initialization fails, the kernel will panic.
#[derive(Debug)]
pub struct KernelModule<T>
where
    T: Debug,
{
    /// The name of the module.s
    pub name: &'static str,
    /// The initialization function.
    pub init: fn() -> Result<(), T>,
    // The state of the module.
    state: Once<()>,
}

impl<T> KernelModule<T>
where
    T: Debug,
{
    /// Create a new kernel module.
    pub const fn new(name: &'static str, init: fn() -> Result<(), T>) -> Self {
        Self {
            name,
            init,
            state: Once::new(),
        }
    }
    /// Initialize the module if it has not been initialized yet. Returns true if the module was initialized.
    /// False does not mean that the module failed to initialize, but rather that it was already initialized.
    #[track_caller]
    pub fn init(&self) -> bool {
        let mut did_init = false;
        self.state.call_once(|| {
            did_init = true;
            info!("Initializing {}", self.name);
            (self.init)()
                .unwrap_or_else(|e| panic!("Error initializing {} module: {:#?}", self.name, e));
            info!("Initialized {}", self.name);
        });
        did_init
    }

    /// Returns true if the module has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.state.is_completed()
    }
}

#[macro_export]
macro_rules! declare_module {
    ($name: literal, $func: ident, $error_type: ty) => {
        #[doc = concat!("The ", $name, " module. This contains logic to protect its internal state and ensure it is only initialized once.")]
        pub static MODULE: $crate::KernelModule<$error_type> =
            $crate::KernelModule::new($name, $func);

        #[doc = concat!("Returns true if the ", $name, " module has been initialized.")]
        #[allow(dead_code)]
        pub fn is_initialized() -> bool {
            MODULE.is_initialized()
        }
    };

    ($name: literal, $func: ident) => {
        declare_module!($name, $func, core::convert::Infallible);
    };
}
