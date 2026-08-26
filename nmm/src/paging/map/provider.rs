use crate::{
    MemError,
    arch::Mapper,
    paging::{
        FragmentManager, FragmentSize, Frame, FullManager, Large, Medium, MemoryFragment, PhysAddr,
        Small, map::SizedMemoryMapper, primitives::FrameClass,
    },
};

/// A provider for memory for a mapping operation.
pub trait MemoryProvider<S: FragmentSize> {
    /// Allocates a frame of the specified size for use as data in a mapping operation.
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError>
    where
        Mapper: SizedMemoryMapper<S>;
    /// Allocates a frame of the specified size for use as a page table in a mapping operation.
    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError>;
}

/// Memory provider to map a linear range of physical addresses to a linear range of virtual addresses.
#[derive(Debug)]
pub struct PhysLinear<'a, Ta: FragmentManager<Frame<Small>, Small>>(pub PhysAddr, pub &'a mut Ta);

impl<'a, Ta: FragmentManager<Frame<Small>, Small>> PhysLinear<'a, Ta> {
    /// Creates a new `Linear` memory provider with the given base physical address.
    pub fn new(base: PhysAddr, table_allocator: &'a mut Ta) -> Self {
        Self(base, table_allocator)
    }
}

impl<'a, Ta: FragmentManager<Frame<Small>, Small>, S: FragmentSize> MemoryProvider<S>
    for PhysLinear<'a, Ta>
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError> {
        let frame =
            Frame::from_start_address(self.0).ok_or(MemError::InvalidFrameAddress(self.0))?;
        self.0 += S::SIZE;
        Ok(frame)
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.1.allocate_fragment()
    }
}

/// A memory provider that uses a single allocator for both data and page tables.
#[derive(Debug)]
pub struct DataAllocator<'a, Ta>(pub &'a mut Ta)
where
    Ta: FullManager<FrameClass>;

impl<'a, Ta: FullManager<FrameClass>, S: FragmentSize> MemoryProvider<S> for DataAllocator<'a, Ta>
where
    Ta: FragmentManager<Frame<S>, S>,
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError> {
        self.0.allocate_fragment()
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.0.allocate_fragment()
    }
}

/// A allocator that is intended for the allocation of page tables.
#[derive(Debug)]
pub struct TableAllocator<'a, T: MemoryProvider<Small>>(&'a mut T);

impl<'a, T> TableAllocator<'a, T>
where
    T: MemoryProvider<Small>,
{
    fn new(provider: &'a mut T) -> Self {
        Self(provider)
    }
}

unsafe impl<'a, T> FragmentManager<Frame<Small>, Small> for TableAllocator<'a, T>
where
    T: MemoryProvider<Small>,
{
    fn allocate_fragment(&mut self) -> Result<Frame<Small>, MemError> {
        self.0.allocate_table()
    }

    fn deallocate_fragment(&mut self, primitive: Frame<Small>) {
        // Deallocation is not supported in this implementation.
        // In a real implementation, you would want to add support for deallocation.
        unimplemented!("Deallocation is not supported in this implementation.");
    }
}

/// A memory provider that uses separate allocators for data and page tables.
#[derive(Debug, Clone, Copy)]
pub struct DataWithTableAllocator<D, T>(pub D, pub T)
where
    D: FullManager<FrameClass>,
    T: FragmentManager<Frame<Small>, Small>;

impl<D: FullManager<FrameClass>, S: FragmentSize, T> MemoryProvider<S>
    for DataWithTableAllocator<D, T>
where
    D: FragmentManager<Frame<S>, S>,
    T: FragmentManager<Frame<Small>, Small>,
{
    fn allocate_data(&mut self) -> Result<Frame<S>, MemError>
    where
        Mapper: SizedMemoryMapper<S>,
    {
        self.0.allocate_fragment()
    }

    fn allocate_table(&mut self) -> Result<Frame<Small>, MemError> {
        self.1.allocate_fragment()
    }
}

pub trait FullProvider:
    MemoryProvider<Small> + MemoryProvider<Medium> + MemoryProvider<Large>
{
    fn table_allocator(&mut self) -> TableAllocator<'_, Self>
    where
        Self: Sized,
    {
        TableAllocator::new(self)
    }
}

impl<T> FullProvider for T where
    T: MemoryProvider<Small> + MemoryProvider<Medium> + MemoryProvider<Large>
{
}
