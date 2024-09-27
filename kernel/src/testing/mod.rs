use crate::{hlt_loop, init_kernel, sprintln};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        sprintln!("Running test {}", core::any::type_name::<T>());
        self();
        sprintln!(".. [ok]");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    sprintln!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use crate::{hlt_loop, sprint, sprintln};

    sprintln!("{}", info);
    hlt_loop()
}

#[test_case]
fn trivial_test_case() {
    assert!(1 == 1);
}

#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start() -> ! {
    init_kernel();
    crate::test_main();
    hlt_loop()
}
