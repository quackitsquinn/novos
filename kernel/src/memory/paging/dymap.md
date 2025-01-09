# Dynamic Physical Address Mapping

The dynamic physical address mapping is a feature that allows custom memory mapping for physical addresses to virtual addresses.

This is used for things like ACPI tables, where the OS needs to know the physical address of the table, but the table is not mapped to the virtual address space.

## Structures

### `PhysicalMapRequest`

The `PhysicalMapRequest` structure is a structure that holds the physical address mapping. This is used to request a physical address mapping.

```rust
pub struct PhysicalMapRequest {
    pub phys_addr: PhysAddr,
    pub size: usize,
    pub flags: PageTableFlags,
}
```

### `PhysicalMap`

This is the actual mapping structure that holds the request, the virtual address, and the flags.

```rust
pub struct PhysicalMap {
    pub request: PhysicalMapRequest,
    pub virt_addr: VirtAddr,
    pub page_base: VirtAddr,
}
```

### Operation

The operation of the dynamic physical address mapping is as follows:

1. The kernel creates a `PhysicalMapRequest` structure with the physical address, size, and flags.
2. The kernel calls the `map` function with the `PhysicalMapRequest` structure.
3. The `map` function iterates over already mapped physical addresses and checks if the requested physical address is already mapped.
    1. If the physical address is already mapped, and the flags are the same, the function returns the virtual address. If the flags are different, the function returns an error. `map` will also attempt to increase the size of the mapping if the requested size is larger than the current mapping. If this fails (ex: map 1 is made then resized after map 2 is made), the function will continue to the next step. (TODO: See if multiple mappings can be made for the same physical address)
4. If the physical address is not already mapped, the function will attempt to find a free virtual address range that is large enough to map the physical address.
5. If a free virtual address range is found, the function will map the physical address to the virtual address range.
6. The function will return a `PhysicalMap` structure with the request, virtual address, and the page base.
