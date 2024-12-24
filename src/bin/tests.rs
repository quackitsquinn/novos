use novos::Config;

// TODO: Figure out how to pass test args to the test build command to be able to run specific tests
fn main() {
    let mut cfg = Config::default();
    cfg.iso = "target/artifacts/kernel_tests.iso".to_string();
    cfg.dev_exit = true;
    cfg.graphics = false;
    cfg.serial.clear();
    cfg.serial.push("chardev:output".to_string());
    //cfg.wait_for_debugger = true;
    cfg.run();
}
