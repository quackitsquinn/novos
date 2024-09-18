use crate::{hlt_loop, sprintln};

// This module is for the testing framework. (IF IT WORKS)
// For some reason, I keep running into issues with getting this working.
// Most currently, I can't get `cargo test --no-run` to output a binary with symbols.
// It outputs a binary, but it doesn't have symbols (including no _start function), so limine can't boot it.
// This is going on the backburner for now, but I will finish it eventually.

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
fn test_test_runner() {
    assert!(1 == 1);
}
