use core::alloc::Layout;

/// A trait for types that can allocate and deallocate memory. This is similar to the `Allocator` or `GlobalAlloc` traits in the standard library,
/// but with the additional requirement that the allocator must be mutable. This is used in `AllocatorWrapper` to allow the wrapped allocator to be
/// changed at runtime.
pub unsafe trait MutableAllocator {
    /// Allocates memory with the given layout. See the `GlobalAlloc` trait for more information.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;
    /// Deallocates memory with the given pointer and layout. See the `GlobalAlloc` trait for more information.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}
