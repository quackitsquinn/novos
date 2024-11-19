use core::mem;

use log::info;
use spin::Mutex;

use crate::{serial, sprintln, util::OnceMutex};

mod qemu_exit;

static TESTS: OnceMutex<&[&TestFunction]> = OnceMutex::new();
static CURRENT: Mutex<usize> = Mutex::new(0);
static NEXT: Mutex<Option<usize>> = Mutex::new(None);

//#[cfg(test)]
pub fn test_runner(tests: &[&TestFunction]) {
    TESTS.init(unsafe { mem::transmute(tests) });
    sprintln!("Running {} tests", tests.len());
    let len = tests.len();
    for (i, test) in tests.iter().enumerate() {
        *CURRENT.lock() = i;
        if i + 1 < len {
            *NEXT.lock() = Some(i + 1);
        } else {
            *NEXT.lock() = None;
        }
        test.run();
    }
    qemu_exit::exit(false);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use crate::sprintln;

    sprintln!("{}", info);
    qemu_exit::exit(true);
}

#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start() -> ! {
    use crate::init_kernel;

    init_kernel();
    crate::test_main();
    qemu_exit::exit(false);
}

pub fn try_shutdown_qemu(non_zero: bool) {
    qemu_exit::exit(non_zero);
}

#[kproc::test("Trivial test")]
fn trivial_test() {
    assert!(1 == 1);
}
// TODO: Recovery implementation and prioritize recoverable tests over unrecoverable tests
pub struct TestFunction {
    /// The function to run.
    pub function: fn(),
    /// The name of the function.
    pub function_name: &'static str,
    /// The name of the test that will be displayed to the user
    /// This should be a human readable name.
    pub human_name: &'static str,
    /// If this test fails/panics, should we continue running tests?
    /// This should be false for tests that test the kernel's core functionality.
    pub can_recover: bool,
    /// The number of times this test should be run.
    pub bench_count: Option<usize>,
}

impl Default for TestFunction {
    fn default() -> Self {
        Self::const_default()
    }
}

impl TestFunction {
    pub const fn const_default() -> Self {
        Self {
            function: || {},
            function_name: "",
            human_name: "",
            can_recover: false,
            bench_count: None,
        }
    }
    pub fn run(&self) {
        sprintln!("Running test: {} ({})", self.human_name, self.function_name);
        #[allow(unused_unsafe)]
        unsafe {
            #[cfg(test)]
            crate::memory::allocator::TEST_ALLOCATOR
                .get()
                .blocks
                .clear();
        }
        let log_level = serial::LOG_LEVEL;
        if let Some(count) = self.bench_count {
            (self.function)();
            info!("Reducing log level to error for benchmarking");
            log::set_max_level(log::LevelFilter::Error);
            for _ in 0..count - 1 {
                (self.function)();
            }
            log::set_max_level(log_level.to_level_filter());
            info!("Restored log level to {}", log_level);
        } else {
            (self.function)();
        }
        sprintln!("Test passed: {} ({})", self.human_name, self.function_name);
    }
}
