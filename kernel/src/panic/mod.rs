use core::panic::PanicInfo;

use log::error;
use spin::Once;

use crate::{hlt_loop, memory::allocator, sprint, sprintln, testing};

mod elf;

pub fn panic_basic(pi: &PanicInfo) {
    error!("PANIC {}", pi);
}

/// A more traditional panic handler that includes more information.
pub fn panic_extended_info(pi: &PanicInfo) {
    sprintln!("=== KERNEL PANIC ===");
    sprint!("Panic at ");
    write_location(pi);
    sprintln!();
    sprintln!("{}", pi.message());
    sprintln!("=== HEAP STATE ===");
    sprintln!("Main heap:");
    // Safety: We are in a panic, so the allocator should be completely halted
    let alloc = unsafe { allocator::ALLOCATOR.force_get() };
    alloc.blocks.print_state();
    sprintln!("Sending heap state to serial");
    alloc.blocks.send_blocks_aux("heap.raw");
    if cfg!(test) {
        sprintln!("Test heap:");
        // Safety: Same as above
        let alloc = unsafe { crate::memory::allocator::TEST_ALLOCATOR.force_get() };
        alloc.blocks.print_state();
        sprintln!("Sending test heap state to serial");
        alloc.blocks.send_blocks_aux("test_heap.raw");
    }
    sprintln!("=== STACK TRACE ===");
    stacktrace::print_trace();
    sprintln!("=== END OF PANIC ===");
}

fn write_location(pi: &PanicInfo) {
    if let Some(location) = pi.location() {
        sprint!("{}:{}", location.file(), location.line())
    } else {
        sprint!("Unknown location")
    }
}

static PANIC_CHECK: Once<()> = Once::new();

pub fn panic(pi: &PanicInfo) -> ! {
    if PANIC_CHECK.is_completed() {
        sprintln!("Double panic!");
        panic_basic(pi);
        hlt_loop();
    }
    PANIC_CHECK.call_once(|| ());
    panic_extended_info(pi);
    sprintln!("Done; attempting QEMU exit");
    testing::try_shutdown_qemu(true);
    sprintln!("Failed to exit QEMU; halting");
    hlt_loop();
}

pub fn init() {
    stacktrace::init();
}

mod stacktrace {
    use core::{arch::asm, slice};

    use goblin::elf64::sym;
    use limine::request::{KernelAddressRequest, KernelFileRequest};
    use log::info;
    use rustc_demangle::demangle;

    use crate::{sprint, sprintln};

    use super::elf::Elf;
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct StackFrame {
        pub rbp: *const StackFrame,
        pub rip: *const (),
    }

    pub fn print_trace() {
        let mut rbp: *const StackFrame;
        unsafe {
            asm!("mov {}, rbp", out(reg) rbp);
        }
        while !rbp.is_null() {
            let frame = unsafe { *rbp };
            sprint!("{:p}:{:p} = ", frame.rbp, frame.rip);
            unsafe { symbol_trace(frame.rip) };
            sprintln!();
            rbp = frame.rbp;
        }
    }
    static KERNEL_ADDR: KernelAddressRequest = KernelAddressRequest::new();
    static KERNEL_FILE: KernelFileRequest = KernelFileRequest::new();

    pub unsafe fn symbol_trace(addr: *const ()) {
        // TODO: This should be put in an init function, along with a Once of the kernel ELF
        let kernel_ptr = KERNEL_FILE
            .get_response()
            .expect("Failed to get kernel file")
            .file()
            .addr();
        let kernel_size = KERNEL_FILE
            .get_response()
            .expect("Failed to get kernel file")
            .file()
            .size();
        let kernel_slice = unsafe { slice::from_raw_parts(kernel_ptr, kernel_size as usize) };

        let elf = Elf::new(kernel_slice).expect("Failed to get kernel ELF");
        let strings = elf.strings().expect("Failed to get kernel strings");
        let mut symbols = elf.symbols().expect("Failed to get kernel symbols");

        let sym = symbols.find(|sym| {
            let sym_addr = sym.st_value as *const ();
            sym_addr <= addr
                && addr < (sym.st_value as usize + sym.st_size as usize) as *mut ()
                && sym::st_type(sym.st_info) == sym::STT_FUNC
        });

        if sym.is_none() {
            sprint!("UNKNOWN (Unable to find symbol!) ");
            return;
        }

        let sym = sym.unwrap();

        let name = unsafe { strings.get_str(sym.st_name as usize) };

        if let Err(err) = name {
            sprint!("UNKNOWN (name error: {:?})", err);
            return;
        }

        let name = name.unwrap();

        let demangled = demangle(name);

        sprint!("{}", demangled);
    }

    pub fn init() {
        info!("Initialized stacktrace");
    }
}
