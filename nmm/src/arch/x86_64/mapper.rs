mod arch_lib {
    pub use x86_64::structures::paging::OffsetPageTable;

    #[cfg(target_arch = "x86_64")]
    pub use x86_64::structures::paging::RecursivePageTable;

    #[cfg(not(target_arch = "x86_64"))]
    mod recursive {
        use std::marker;

        use x86_64::structures::paging::{PageTable, Size4KiB};

        pub struct RecursivePageTable<'a>(marker::PhantomData<&mut PageTable>);

        impl RecursivePageTable {
            pub unsafe fn new_unchecked(
                table: &'a mut PageTable,
                recursive_index: PageTableIndex,
            ) -> Self {
                todo!()
            }

            pub fn recursive_index(&self) -> PageTableIndex {
                todo!()
            }

            pub fn p4(&self) -> &PageTable {
                todo!()
            }

            pub unsafe fn p4_mut(&mut self) -> &mut PageTable {
                todo!()
            }
        }

        macro_rules! noop_impl {
            ($size: path) => {
                impl x86_64::structures::paging::mapper::Mapper<$size> for RecursivePageTable {
                    unsafe fn map_to<A>(
                        &mut self,
                        _page: x86_64::structures::paging::Page<$size>,
                        _frame: x86_64::structures::paging::PhysFrame<$size>,
                        _flags: x86_64::structures::paging::PageTableFlags,
                        _allocator: &mut A,
                    ) -> Result<
                        x86_64::structures::paging::mapper::Flushable,
                        x86_64::structures::paging::mapper::MapToError<$size, A>,
                    >
                    where
                        A: x86_64::structures::paging::FrameAllocator<$size>,
                    {
                        todo!()
                    }

                    fn translate_page(
                        &self,
                        _page: x86_64::structures::paging::Page<$size>,
                    ) -> Option<x86_64::structures::paging::PhysFrame<$size>> {
                        todo!()
                    }

                    unsafe fn map_to_with_table_flags<A>(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                        frame: x86_64::structures::paging::PhysFrame<$size>,
                        flags: x86_64::structures::paging::PageTableFlags,
                        parent_table_flags: x86_64::structures::paging::PageTableFlags,
                        frame_allocator: &mut A,
                    ) -> Result<
                        x86_64::structures::paging::mapper::MapperFlush<$size>,
                        x86_64::structures::paging::mapper::MapToError<$size>,
                    >
                    where
                        Self: Sized,
                        A: x86_64::structures::paging::FrameAllocator<$size> + ?Sized,
                    {
                        todo!()
                    }

                    fn unmap(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                    ) -> Result<
                        (
                            x86_64::structures::paging::PhysFrame<$size>,
                            x86_64::structures::paging::mapper::MapperFlush<$size>,
                        ),
                        x86_64::structures::paging::mapper::UnmapError,
                    > {
                        todo!()
                    }

                    unsafe fn update_flags(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                        flags: x86_64::structures::paging::PageTableFlags,
                    ) -> Result<
                        x86_64::structures::paging::mapper::MapperFlush<$size>,
                        x86_64::structures::paging::mapper::FlagUpdateError,
                    > {
                        todo!()
                    }

                    unsafe fn set_flags_p4_entry(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                        flags: x86_64::structures::paging::PageTableFlags,
                    ) -> Result<
                        x86_64::structures::paging::mapper::MapperFlushAll,
                        x86_64::structures::paging::mapper::FlagUpdateError,
                    > {
                        todo!()
                    }

                    unsafe fn set_flags_p3_entry(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                        flags: x86_64::structures::paging::PageTableFlags,
                    ) -> Result<
                        x86_64::structures::paging::mapper::MapperFlushAll,
                        x86_64::structures::paging::mapper::FlagUpdateError,
                    > {
                        todo!()
                    }

                    unsafe fn set_flags_p2_entry(
                        &mut self,
                        page: x86_64::structures::paging::Page<$size>,
                        flags: x86_64::structures::paging::PageTableFlags,
                    ) -> Result<
                        x86_64::structures::paging::mapper::MapperFlushAll,
                        x86_64::structures::paging::mapper::FlagUpdateError,
                    > {
                        todo!()
                    }
                }
            };

            ($($size: path),*) => {
                $(noop_impl!($size);)*
            };
        }

        noop_impl!(Size4KiB, Size2MiB, Size1GiB);
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub use recursive::RecursivePageTable;
}

/// THe lowest level of mapper for x86_64.
pub enum Mapper {
    Offset(arch_lib::OffsetPageTable<'static>),
}
