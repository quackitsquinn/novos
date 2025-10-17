//! Kernel stacktracing and symbolication.
use core::{
    arch::asm,
    fmt::{Debug, Write},
};

use kelp::goblin::elf64::sym;
use rustc_demangle::demangle;

use crate::{print, println, requests::KERNEL_ELF};

/// A stack frame in the x86_64 architecture.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StackFrame {
    /// The base pointer of the previous stack frame.
    pub rbp: *const StackFrame,
    /// The instruction pointer for this stack frame.
    pub rip: *const (),
}

/// Prints a stack trace of the current call stack.
pub fn print_trace() {
    let mut rbp: *const StackFrame;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }
    unsafe { print_trace_raw(rbp) };
}

/// Prints a stack trace of the given base pointer.
pub unsafe fn print_trace_raw(rbp: *const StackFrame) {
    let mut rbp = rbp;
    while !rbp.is_null() {
        let frame = unsafe { *rbp };
        println!("{:p}: {}", frame.rip, fmt_symbol(frame.rip));
        rbp = frame.rbp;
    }
}

/// Prints the symbolicated demangled name of the given address.
pub unsafe fn symbol_trace(addr: *const ()) {
    print!("{}", fmt_symbol(addr));
}

unsafe fn sym_trace_inner(addr: *const (), writer: &mut dyn Write) {
    let elf = unsafe { KERNEL_ELF.get().elf_unchecked() };
    let strings = elf.strings().expect("Failed to get kernel strings");
    let symbols = elf.symbols().expect("Failed to get kernel symbols");

    let sym = symbols.iter().find(|sym| {
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

/// A struct that implements `core::fmt::Display` and `core::fmt::Debug` for formatting a symbol name.
pub struct FormattableSymbol(*const ());

impl core::fmt::Display for FormattableSymbol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe { sym_trace_inner(self.0, f) };
        Ok(())
    }
}

impl core::fmt::Debug for FormattableSymbol {
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
