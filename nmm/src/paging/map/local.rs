use core::{
    cell::{Cell, Ref, RefCell, RefMut, UnsafeCell},
    sync::atomic::{AtomicU32, AtomicU64},
};

use cake::log::trace;

use crate::{
    MapFlags, MemError,
    arch::Mapper,
    paging::{
        Address, FragmentManager, FragmentSize, Frame, Large, Medium, Page, Small,
        fragment::GreedyFragmentMapper,
        map::{Flush, FullProvider, MemoryMapper, MemoryProvider, SizedMemoryMapper, Unmapped},
        primitives::{AnyFragment, PageClass},
    },
};

pub struct LocalMemoryMapper<M: MemoryMapper> {
    inner: RefCell<M>,
    state: Cell<bool>,
    belongs_to: AtomicU64,
}

impl<M: MemoryMapper> LocalMemoryMapper<M> {
    pub fn new(mapper: M) -> Self {
        Self {
            inner: RefCell::new(mapper),
            state: Cell::new(false),
            belongs_to: AtomicU64::new(cake::core_id()),
        }
    }

    fn check_core(&self) {
        if cake::core_id() != self.belongs_to.load(core::sync::atomic::Ordering::Acquire) {
            panic!("LocalMemoryMapper accessed from a different core than it was created on");
        }
    }

    /// Returns a mutable reference to the actual mapper object.
    fn mapper(&self) -> RefMut<'_, M> {
        self.check_core();
        self.inner.borrow_mut()
    }

    /// Returns a struct that implements `MemoryMapper`. This function is lock/borrowless.
    pub fn get_mapper(&self) -> MapperMut<'_, M> {
        self.check_core();
        MapperMut::new(self)
    }
}

pub(crate) struct MapperMut<'a, M: MemoryMapper> {
    inner: &'a LocalMemoryMapper<M>,
}

impl<'a, M: MemoryMapper> MapperMut<'a, M> {
    pub(crate) fn new(inner: &'a LocalMemoryMapper<M>) -> Self {
        Self { inner }
    }
}

impl<'a, S: FragmentSize, M: SizedMemoryMapper<S>> SizedMemoryMapper<S> for MapperMut<'a, M>
where
    M: MemoryMapper,
{
    fn map_primitive<A>(
        &mut self,
        dst: Page<S>,
        src: Frame<S>,
        flags: MapFlags,
        allocator: &mut A,
    ) -> Result<Flush, MemError>
    where
        A: FragmentManager<Frame<Small>, Small>,
    {
        self.inner
            .mapper()
            .map_primitive(dst, src, flags, allocator)
    }

    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError> {
        unsafe { self.inner.mapper().unmap_primitive(page) }
    }
}

impl<'a, M: MemoryMapper> MemoryMapper for MapperMut<'a, M> {}
