use core::{arch::asm, fmt::Write, slice};

use kelp::{goblin::elf64::sym, Elf};
use rustc_demangle::demangle;
use spin::Once;

use crate::{print, println, requests::EXECUTABLE_FILE};

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
    print_trace_raw(rbp);
}

pub fn print_trace_raw(rbp: *const StackFrame) {
    let mut rbp = rbp;
    while !rbp.is_null() {
        let frame = unsafe { *rbp };
        print!("{:p}:{:p} = ", frame.rbp, frame.rip);
        unsafe { symbol_trace(frame.rip) };
        println!();
        rbp = frame.rbp;
    }
}

static KERNEL_FILE: Once<&[u8]> = Once::new();

pub unsafe fn symbol_trace(addr: *const ()) {
    print!("{}", fmt_symbol(addr));
}

unsafe fn sym_trace_inner(addr: *const (), writer: &mut dyn Write) {
    // TODO: Im not entirely confident that the implementation of this function is correct. I think addr should be offset by the kernel base address.
    // The current implementation works, so I'm not going to touch it unless it breaks.
    let kernel_slice = KERNEL_FILE.get().expect("Kernel file not loaded");

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
        write!(writer, "UNKNOWN (Unable to find symbol!) ").unwrap();
        return;
    }

    let sym = sym.unwrap();

    let name = unsafe { strings.get_str(sym.st_name as usize) };

    if let Err(err) = name {
        write!(writer, "UNKNOWN (name error: {:?})", err).unwrap();
        return;
    }

    let name = name.unwrap();

    let demangled = demangle(name);

    write!(writer, "{}", demangled).unwrap();
}

pub fn init() {
    let kern_file = EXECUTABLE_FILE
        .get()
        .expect("Kernel file not loaded")
        .file();
    let size = kern_file.size();
    let ptr = kern_file.addr();
    let slice = unsafe { slice::from_raw_parts(ptr as *const u8, size as usize) };
    KERNEL_FILE.call_once(|| slice);
}

pub struct FormattableSymbol(*const ());

impl core::fmt::Display for FormattableSymbol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe { sym_trace_inner(self.0, f) };
        Ok(())
    }
}

/// Formats the symbol name of the given address. Returns a struct that implements `core::fmt::Display` and `core::fmt::Debug`.

pub fn fmt_symbol(addr: *const ()) -> FormattableSymbol {
    FormattableSymbol(addr)
}

/// Returns the RIP of the caller of this function.
pub fn get_caller_rip() -> *const () {
    let mut rbp: *const StackFrame;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }
    let frame = unsafe { *rbp };
    let last_frame = frame.rbp;
    let frame = unsafe { *last_frame };
    frame.rip
}
