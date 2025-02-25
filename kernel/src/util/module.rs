use core::fmt::{Debug, Display};

use log::info;
use spin::Once;

/// A kernel module that can be initialized once. If the initialization fails, the kernel will panic.
pub struct KernelModule<T>
where
    T: Debug + Display,
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
    T: Debug + Display,
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
    pub fn init(&self) -> bool {
        let mut did_init = false;
        self.state.call_once(|| {
            did_init = true;
            info!("Initializing {}", self.name);
            (self.init)()
                .unwrap_or_else(|e| panic!("Error initializing {} module: {}", self.name, e));
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
    ($name: expr, $func: ident, $error_type: ty) => {
        pub static MODULE: $crate::util::KernelModule<$error_type> =
            $crate::util::KernelModule::new($name, $func);

        pub fn is_initialized() -> bool {
            MODULE.is_initialized()
        }
    };

    ($name: expr, $func: ident) => {
        declare_module!($name, $func, core::convert::Infallible);
    };
}
