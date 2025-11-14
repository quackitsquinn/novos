//! GDT setup.
use core::convert::Infallible;

use cake::{Lazy, declare_module};

pub mod local;

use crate::gdt::local::LocalGdt;

/// The global LocalGdt instance. Contains the GDT and TSS for the current core.
pub static LGDT: Lazy<LocalGdt> = Lazy::new(LocalGdt::new);

fn init() -> Result<(), Infallible> {
    unsafe { LGDT.load() };
    Ok(())
}

declare_module!("gdt", init);
