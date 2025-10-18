//! Testing tools.

use cake::Mutex;

mod qemu_exit;
pub mod test_fn;
pub use test_fn::TestFunction;

#[cfg(test)]
static TESTS: RwLock<&'static [&'static TestFunction]> = RwLock::new(&[]);
#[cfg(test)]
static CURRENT: Mutex<usize> = Mutex::new(0);
static IN_TEST_FRAMEWORK: Mutex<bool> = Mutex::new(true);

/// Runs the given tests.
#[cfg(test)]
pub fn test_runner(tests: &[&TestFunction]) /*-> ! */
{
    // SAFETY: transmute converts &'a [&'a TestFunction] to &'static [&'static TestFunction]
    // This is safe because after this function never returns, the reference will never be used again as `qemu_exit::exit` halts the CPU if it cannot exit.
    *TESTS.try_write().expect("unable to write") = unsafe { transmute(tests) };
    tests.iter().enumerate().for_each(|(i, test)| {
        *CURRENT.lock() = i;
        sprintln!("Running test: {} (i {})", test.human_name, i);
        test.run();
    });
    qemu_exit::exit(false)
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use alloc::vec::Vec;

    use crate::panic;

    // Then, check if the panic is a internal panic. If it is, panic with kernel handler.
    if let Some(r) = IN_TEST_FRAMEWORK.try_lock() {
        // Something has gone wrong, defer to kernel panic handler
        if *r {
            crate::panic::panic(info)
        }
    } else {
        // We failed to lock the mutex, meaning we are already somewhere in the test framework
        crate::panic::panic(info)
    }

    // If we got here, we are in a test panic, but because it is kernel code running, we set test framework to true.
    *IN_TEST_FRAMEWORK.lock() = true;
    // SAFETY: We are in a panic handler, no tests should be running so we can force unlock the mutex
    unsafe {
        TESTS.force_write_unlock();
    }

    let current = *CURRENT.lock();
    let tests = TESTS.try_read().expect("Failed to get tests");
    let test = tests[current];
    // If the test should panic, we should pass it. Otherwise, we should fail it.
    if test.should_panic {
        test.passed();
    } else {
        test.failed();
    }

    panic::panic_extended_info(info);

    // Attempt recovery if possible
    if test.can_recover || test.should_panic {
        if current + 1 < tests.len() {
            let run_tests = tests[current + 1..]
                .iter()
                .map(|t| *t)
                .collect::<Vec<&TestFunction>>();
            drop(tests);
            test_runner(&run_tests);
        } else {
            qemu_exit::exit(false);
        }
    }
    crate::panic::panic(info);
}

#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start() -> ! {
    use crate::init_kernel_services;

    unsafe {
        init_kernel_services();
    }
    crate::test_main();
    qemu_exit::exit(true)
}

/// Attempts to shut down QEMU with the given exit code.
pub fn try_shutdown_qemu(non_zero: bool) {
    qemu_exit::exit(non_zero);
}

#[kproc::test("Trivial test")]
fn trivial_test() {
    assert!(1 == 1);
}

#[kproc::test("Panic test", should_panic = true)]
fn failing_test() {
    assert!(1 == 2);
}
