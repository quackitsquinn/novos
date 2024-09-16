use core::time::Duration;

use log::info;
use spin::Once;

use crate::{interrupts::hardware::timer::Timer, util::OnceMutex};

use super::raw::SerialPort;
// In the debug env, the harness connects almost instantly, so we can set a very low timeout.
const HARNESS_CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const DEBUG_HARNESS_PORT: u16 = 0x2f8;

static HARNESS_PORT: OnceMutex<SerialPort> = OnceMutex::new();
static HAS_HARNESS: Once<()> = Once::new();

pub fn init_debug_harness() {
    info!("Waiting for debug harness to connect...");
    HARNESS_PORT.init({
        let mut port = unsafe { SerialPort::new(DEBUG_HARNESS_PORT) };
        port.init();
        port.send(b'i');
        port
    });

    let timer = Timer::new(HARNESS_CONNECT_TIMEOUT);
    info!(
        "Waiting for debug harness to connect... (waiting for {} ticks)",
        timer.ticks
    );
    while !timer.is_done() {
        if let Ok(code) = HARNESS_PORT.get().try_receive() {
            if code == 255 {
                info!("Debug harness failed to connect");
                break;
            }
            info!("Debug harness connected with code: {}", code);
            HAS_HARNESS.call_once(|| ());
            break;
        }
    }

    if !HAS_HARNESS.is_completed() {
        info!("Debug harness failed to connect");
    }

    HARNESS_PORT.get().send(b'o');
}
