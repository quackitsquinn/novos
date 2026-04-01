use crate::arch::VirtAddr;

/// A fancy macro for defining the layout of the kernel's virtual address space.
#[macro_export]
macro_rules! kernel_map {
    (

            . = $start:tt,
            $($rest:tt)*
    ) => {
        /// The kernel map, which defines the layout of the kernel's virtual address space.
        /// For now, this doesn't have much use. Down the road this will be used for KASLR.
        pub static KERNEL_MAP: $crate::kernel_map::KernelMap = $crate::kernel_map::KernelMap {
            sections:
                $crate::kernel_map!(gen_sections $($rest)*)

        };

        /// The kernel map modules, which define the start and size of each section of the kernel map as constants.
        pub mod map {

            $crate::kernel_map!(gen_modules @munch $crate::kernel_map!(read_start $start), $($rest)*);
        }
    };
    (read_start higher_half) => {
        $crate::arch::VirtAddr::HIGHER_HALF_START
    };
    (read_start (higher_half + $size:tt $($unit:ident)?)) => {
        $crate::arch::VirtAddr::HIGHER_HALF_START.checked_add($crate::kernel_map!(size $size $($unit)?)).expect("Kernel map section overflow")
    };
    (read_start $rest:tt) => {
       $crate::arch::VirtAddr::new($rest)
    };



    (path $name:ident$(::$part:ident)*) => {
        $crate::_pastey::paste! {
            map::[<$name:lower>]$(::$part)*
        }
    };


    (gen_module $name:ident, $base: expr, $size: expr) => {
        $crate::_pastey::paste!{ pub mod [<$name:lower>] { // TODO: figure out how to allow for documenting these modules
            /// The start address of the section.
            pub const START: $crate::arch::VirtAddr = $base;
            /// The start address of the section as a raw u64.
            pub const START_RAW: u64 = START.as_u64();
            /// The size of the section.
            pub const SIZE: u64 = $size;
            /// The end address of the section.
            pub const END: $crate::arch::VirtAddr = $base.checked_add($size).expect("Kernel map section overflow");
            /// The end address of the section as a raw u64.
            pub const END_RAW: u64 = END.as_u64();
            /// The virtual memory range of the section.
            pub const RANGE: $crate::VirtualMemoryRange = $crate::VirtualMemoryRange::new(START, SIZE);
        }}

    };

    (gen_modules @munch $start:expr,) => {};

    (gen_modules @munch $start:expr, $name:ident = $size:tt $($size_unit: ident)?, $($rest:tt)*) => {
        $crate::kernel_map!(gen_module $name, $start, $size);
        $crate::kernel_map!(gen_modules @munch ($start.checked_add($crate::kernel_map!(size $size $($size_unit)?)).expect("overflow")), $($rest)*);
    };

    (gen_modules @munch $start:expr, $name:ident = $size:tt $($size_unit: ident)?; align $alignment:tt $($align_unit: ident)?, $($rest:tt)*) => {
        $crate::kernel_map!(gen_module $name,
            $crate::arch::VirtAddr::new_truncate(
                $crate::align!(up, $start.as_u64(), $crate::kernel_map!(size $alignment $($align_unit)?))),
                $crate::kernel_map!(size $size $($size_unit)?)

        );
        $crate::kernel_map!(gen_modules @munch
            $crate::arch::VirtAddr::new_truncate(
                $crate::align!(up, $start.as_u64(), $crate::kernel_map!(size $alignment $($align_unit)?)) + $crate::kernel_map!(size $size $($size_unit)?) )
                .checked_add($size).expect("overflow"),
                $($rest)*
            );
    };

    (gen_section $name:ident, $base: expr, $size:expr ) => {
        $crate::kernel_map::KernelMapSection {
            name: stringify!($name),
            start: $base,
            size: $size,
        }
    };

    (gen_sections $($name:ident = $size:tt $($size_unit: ident)? $(; align $alignment:tt $($align_unit: ident)?)?,)*) => {
        &[
            $(
                $crate::kernel_map!(gen_section $name, $crate::kernel_map!(path $name::START), $crate::kernel_map!(path $name::SIZE)),
            )*
        ]
    };

    (size $size:literal KiB) =>{
        $size * 1024
    };

    (size $size:literal MiB) =>{
        $size * 1024 * 1024
    };

    (size $size:literal GiB) =>{
        $size * 1024 * 1024 * 1024
    };

    (size $expr:expr) => {
        $expr
    };
}

// somehow define this syntax in a macro:
//
// . = n
// NMM_MANAGED_RANGE = 0x200000000, ALIGN 0x100000000
// ADDRESS_SPACE_INFO = 0x1
//
// e.g.
// . = n
// NMM_MANAGED_RANGE = SIZE ALIGN ALIGNMENT
// each line is placed directly after the previous one, unless an alignment is specified, in which case it is aligned up to the next multiple of the alignment after the previous section. The start address of each section is stored in a constant with the same name as the section, and the size of each section is stored in a constant with the name of the section followed by _SIZE. Additionally, a constant with the name of the section followed by _END is defined, which is the end address of the section (start + size). All addresses should be page aligned (multiples of 0x1000).
// ADDRESS_SPACE_INFO = SIZE
//
//

/// The kernel map, which defines the layout of the kernel's virtual address space. This is used for documentation and debugging purposes, and will eventually be used for KASLR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelMap {
    /// The sections of the kernel map, which define the contiguous ranges of virtual addresses in the kernel's address space.
    pub sections: &'static [KernelMapSection],
}

/// A section of the kernel map, which defines a contiguous range of virtual addresses in the kernel's address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelMapSection {
    /// The name of the section, which is used for documentation and debugging purposes.
    pub name: &'static str,
    /// The start address of the section.
    pub start: VirtAddr,
    /// The size of the section.
    pub size: u64,
}

kernel_map! {
    . = (higher_half + 512 GiB),
    NMM_MANAGED_RANGE = 8 GiB; align 1 GiB,
    KERNEL_HEAP = 16 MiB; align 2 MiB,
    KERNEL_PHYS_MAP = 256 MiB; align 2 MiB,
    KERNEL_REMAP = 256 MiB; align 2 MiB,
    FRAMEBUFFER = 2 MiB; align 2 MiB,
    ADDRESS_SPACE_INFO = 4 KiB; align 4 KiB,
}
