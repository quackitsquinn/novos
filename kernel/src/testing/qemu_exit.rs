
use spin::Mutex;
use x86_64::instructions::port::Port;


const QEMU_EXIT_PORT: u16 = 0xf4;
static PORT: Mutex<Port<u32>> = Mutex::new(Port::new(QEMU_EXIT_PORT));

pub fn exit(non_zero: bool) -> ! {
    let value = if non_zero { 1 } else { 0 };
    unsafe {
        PORT.lock().write(value);
    }
    panic!("QEMU exit failed");
}
