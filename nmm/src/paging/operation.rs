//! This module defines the `Operation` trait and its implementations for various memory mapping operations,
//! such as zeroing memory and copying memory. It also provides a way to chain multiple operations together.
use crate::{
    MemError, arch,
    paging::{
        AddressExt, FragmentSize, Frame, Large, Medium, MemoryFragment, Page, Small, asm,
        map::SizedMemoryMapper,
    },
};

/// A trait representing a memory mapping operation for a specific page size.
pub trait Operation<S: FragmentSize> {
    /// Executes the memory mapping operation for a given page and frame.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided page and frame are valid and that the operation is safe to perform.
    unsafe fn execute(&mut self, dst: Page<S>, src: Frame<S>) -> Result<(), MemError>;
}

/// A trait representing a memory mapping operation for all page sizes (small, medium, and large).
pub trait OperationAllSizes: Operation<Small> + Operation<Medium> + Operation<Large> {}

impl<T> OperationAllSizes for T where T: Operation<Small> + Operation<Medium> + Operation<Large> {}

impl<S> Operation<S> for ()
where
    S: FragmentSize,
{
    unsafe fn execute(&mut self, _dst: Page<S>, _src: Frame<S>) -> Result<(), MemError> {
        Ok(())
    }
}

/// Zeros the memory of a frame.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ZeroMemory;

impl<S> Operation<S> for ZeroMemory
where
    S: FragmentSize,
    arch::Mapper: SizedMemoryMapper<S>,
{
    unsafe fn execute(&mut self, _: Page<S>, src: Frame<S>) -> Result<(), MemError> {
        unsafe { asm::zero_frame(src) }
    }
}

/// Two memory mapping operations chained together, where the second operation is executed after the first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Chain<A, B>(pub A, pub B);

impl<A, B, S> Operation<S> for Chain<A, B>
where
    S: FragmentSize,
    A: Operation<S>,
    B: Operation<S>,
{
    unsafe fn execute(&mut self, dst: Page<S>, src: Frame<S>) -> Result<(), MemError> {
        unsafe {
            self.0.execute(dst, src)?;
            self.1.execute(dst, src)
        }
    }
}
/// Copy the contents of the provided buffer into the mapped memory.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CopyMemory<'a> {
    /// The buffer to copy into the mapped memory.
    pub buf: &'a [u8],
}

impl CopyMemory<'_> {
    /// Creates a new `CopyMemory` operation with the provided buffer.
    pub fn new(buf: &[u8]) -> CopyMemory<'_> {
        CopyMemory { buf }
    }
}

impl<S> Operation<S> for CopyMemory<'_>
where
    S: FragmentSize,
    arch::Mapper: SizedMemoryMapper<S>,
{
    unsafe fn execute(&mut self, _: Page<S>, src: Frame<S>) -> Result<(), MemError> {
        let size = core::cmp::min(self.buf.len(), S::SIZE as usize);
        unsafe {
            asm::map_with_scratch_page(src, crate::MapFlags::WRITABLE, |page| {
                let ptr = page.start_address().as_mut_ptr::<u8>();
                core::ptr::copy_nonoverlapping(self.buf.as_ptr(), ptr, size);
            })?;
        }
        self.buf = &self.buf[size..];
        Ok(())
    }
}
