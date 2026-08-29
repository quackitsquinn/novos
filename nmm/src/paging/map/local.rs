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
    inner: UnsafeCell<M>,
    state: Cell<bool>,
    belongs_to: AtomicU64,
}

impl<M: MemoryMapper> LocalMemoryMapper<M> {
    pub fn new(mapper: M) -> Self {
        Self {
            inner: UnsafeCell::new(mapper),
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
    pub(crate) fn mapper(&self) -> MapperRefMut<'_, M> {
        self.check_core();
        if self.state.get() {
            panic!("LocalMemoryMapper is already borrowed");
        }
        self.state.set(true);
        MapperRefMut { inner: self }
    }
}

pub(crate) struct MapperRefMut<'a, M: MemoryMapper> {
    inner: &'a LocalMemoryMapper<M>,
}

impl<'a, M: MemoryMapper> MapperRefMut<'a, M> {
    /// Returns a guard that temporarily yields the lock on the mapper,
    /// allowing other code to access it. The lock will be reacquired when the guard is dropped.
    ///
    /// This borrows the mapper mutably for the lifetime of the guard, so it cannot be used while the guard is active.
    pub(crate) fn yield_lock<'b>(&'b mut self) -> YieldGuard<'b, 'a, M>
    where
        'a: 'b,
    {
        self.inner.state.set(false);
        YieldGuard::new(self)
    }

    fn as_mut(&mut self) -> &mut M {
        unsafe { &mut *self.inner.inner.get() }
    }
}

impl<M: MemoryMapper> Drop for MapperRefMut<'_, M> {
    fn drop(&mut self) {
        self.inner.state.set(false);
    }
}

impl<'a, S: FragmentSize, M: SizedMemoryMapper<S>> SizedMemoryMapper<S> for LocalMemoryMapper<M>
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
        self.mapper().map_primitive(dst, src, flags, allocator)
    }

    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError> {
        unsafe { self.mapper().unmap_primitive(page) }
    }
}

impl<'a, S: FragmentSize, M> SizedMemoryMapper<S> for MapperRefMut<'a, M>
where
    M: MemoryMapper + SizedMemoryMapper<S>, // This doesn't change anything, but it makes the compiler happy
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
        self.as_mut().map_primitive(dst, src, flags, allocator)
    }

    unsafe fn unmap_primitive(&mut self, page: Page<S>) -> Result<Unmapped<S>, MemError> {
        unsafe { self.as_mut().unmap_primitive(page) }
    }
}

impl<'a, M: MemoryMapper> MemoryMapper for MapperRefMut<'a, M> {
    unsafe fn map_from_with_operation<D>(
        &mut self,
        base: crate::paging::VirtAddr,
        len: u64,
        flags: MapFlags,
        provider: &mut D,
        mut op: impl crate::paging::operation::OperationAllSizes,
    ) -> Result<(), MemError>
    where
        D: FullProvider,
    {
        trace!(
            "Mapping from base address {:x?} with length {:?} and flags {:?}",
            base.as_u64(),
            len,
            flags
        );

        let mapper = GreedyFragmentMapper::<PageClass>::new(base, len);
        for frag in mapper {
            match frag {
                AnyFragment::Small(prim) => {
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                    let _guard = self.yield_lock();

                    unsafe { op.execute(prim, frame)? };
                }
                AnyFragment::Medium(prim) => {
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                    let _guard = self.yield_lock();

                    unsafe { op.execute(prim, frame)? };
                }
                AnyFragment::Large(prim) => {
                    let frame = provider.allocate_data()?;
                    self.map_primitive(prim, frame, flags, &mut provider.table_allocator())?
                        .flush();
                    let _guard = self.yield_lock();

                    unsafe { op.execute(prim, frame)? };
                }
            }
        }

        Ok(())
    }
}

pub(crate) struct YieldGuard<'a, 'b, M: MemoryMapper> {
    inner: &'a mut MapperRefMut<'b, M>,
}

impl<'a, 'b, M: MemoryMapper> YieldGuard<'a, 'b, M> {
    fn new(inner: &'a mut MapperRefMut<'b, M>) -> Self {
        Self { inner }
    }
}

impl<M: MemoryMapper> Drop for YieldGuard<'_, '_, M> {
    fn drop(&mut self) {
        self.inner.inner.state.set(true);
    }
}
