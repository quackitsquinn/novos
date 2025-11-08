use std::path::Path;

use krun::{CharDev, QemuConfig};

pub const MONITOR_PTY_LINK: &str = "target/monitor.pty";
pub fn main() {
    let mut cfg = QemuConfig::default().with_default_chardevs();
    let chardev = cfg
        .push_chardev(CharDev::pty(
            "monitor",
            Some((Path::new(MONITOR_PTY_LINK), true)),
        ))
        .expect("unable to create monitor chardev");
    cfg.monitor(chardev);
    cfg.run();
}
