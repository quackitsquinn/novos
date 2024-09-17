#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    use core::any::type_name_of_val;

    use crate::{init_kernel, sprint, sprintln};

    init_kernel();

    sprintln!("Running {} tests", tests.len());
    for test in tests {
        sprint!("Running test {} ...", type_name_of_val(test));
        test();
        sprintln!("[ok]");
    }
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
