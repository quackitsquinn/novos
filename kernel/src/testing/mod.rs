use crate::{hlt_loop, sprintln};

mod qemu_exit;

//#[cfg(test)]
pub fn test_runner(tests: &[&TestFunction]) {
    sprintln!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    qemu_exit::exit(false);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use crate::{hlt_loop, sprint, sprintln};

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

#[kproc::test("Trivial test")]
fn trivial_test() {
    assert!(1 == 1);
}

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
        }
    }
    pub fn run(&self) {
        sprintln!("Running test: {} ({})", self.human_name, self.function_name);
        (self.function)();
        sprintln!("Test passed: {} ({})", self.human_name, self.function_name);
    }
}
