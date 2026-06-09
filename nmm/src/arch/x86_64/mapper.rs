mod arch_lib {
    pub use x86_64::structures::paging::OffsetPageTable;
}
/// THe lowest level of mapper for x86_64.
pub enum Mapper {
    Offset(arch_lib::OffsetPageTable<'static>),
}
