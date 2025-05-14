use krun::Config;

pub fn main() {
    let mut cfg = Config::default();
    // TODO: Refactor so that you don't have to do this
    cfg.serial.clear();
    cfg.serial.push("chardev:output".to_string());
    cfg.run();
}
