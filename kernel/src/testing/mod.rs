use core::{mem::transmute, sync::atomic::AtomicBool};

use alloc::vec::Vec;
use log::info;
use spin::{Mutex, Once, RwLock};

use crate::{serial, sprintln, util::OnceMutex};

mod qemu_exit;
pub mod test_fn;
pub use test_fn::TestFunction;

static TESTS: RwLock<&'static [&'static TestFunction]> = RwLock::new(&[]);
static CURRENT: Mutex<usize> = Mutex::new(0);
static IN_TEST_FRAMEWORK: Mutex<bool> = Mutex::new(true);

//#[cfg(test)]
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
    use core::{panic::PanicInfo, sync::atomic::Ordering};

    if let Some(r) = IN_TEST_FRAMEWORK.try_lock() {
        // Something has gone wrong, defer to kernel panic handler
        if *r {
            crate::panic::panic(info)
        }
    } else {
        // We failed to lock the mutex, meaning we are already somewhere in the test framework
        crate::panic::panic(info)
    }

    *IN_TEST_FRAMEWORK.lock() = true;
    // SAFETY: We are in a panic handler, no tests should be running so we can force unlock the mutex
    unsafe {
        TESTS.force_write_unlock();
    }

    let current = *CURRENT.lock();
    let tests = TESTS.try_read().expect("Failed to get tests");
    let test = tests[current];
    if test.should_panic {
        test.passed();
    } else {
        test.failed();
    }

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
    use crate::init_kernel;

    init_kernel();
    crate::test_main();
    qemu_exit::exit(true)
}

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
