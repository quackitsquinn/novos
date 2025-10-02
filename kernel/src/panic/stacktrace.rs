use core::{arch::asm, fmt::Write};

use kelp::goblin::elf64::sym;
use log::error;
use rustc_demangle::demangle;

use crate::{print, println, requests::KERNEL_ELF};

/// A stack frame in the x86_64 architecture.
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

pub unsafe fn symbol_trace(addr: *const ()) {
    print!("{}", fmt_symbol(addr));
}

pub fn get_symbol_name(addr: usize) -> Option<&'static str> {
    let addr = addr as *const ();
    let elf = unsafe { KERNEL_ELF.get().elf_unchecked() };
    let strings = elf.strings().expect("Failed to get kernel strings");
    let mut symbols = elf.symbols().expect("Failed to get kernel symbols");

    let sym = symbols.find(|sym| {
        let sym_addr = sym.st_value as *const ();
        sym_addr <= addr
            && addr < (sym.st_value as usize + sym.st_size as usize) as *mut ()
            && sym::st_type(sym.st_info) == sym::STT_FUNC
    })?;

    let name = unsafe { strings.get_str(sym.st_name as usize) };

    if let Err(err) = name {
        error!("Unable to get symbol name: {:?}", err);
        return None;
    }
    let name = name.unwrap();
    Some(name)
}

unsafe fn sym_trace_inner(addr: *const (), writer: &mut dyn Write) {
    let elf = unsafe { KERNEL_ELF.get().elf_unchecked() };
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

pub fn get_caller_rip_2_up() -> usize {
    let mut rbp: *const StackFrame;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }
    let frame = unsafe { *rbp };
    let last_frame = frame.rbp;
    let frame = unsafe { *last_frame };
    let last_frame = frame.rbp;
    let frame = unsafe { *last_frame };
    frame.rip as usize
}
