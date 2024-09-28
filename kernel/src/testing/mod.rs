use crate::{hlt_loop, sprintln};

mod qemu_exit;

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

//#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
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

#[test_case]
fn trivial_test() {
    assert!(1 == 1);
}
