//! Per-core Global Descriptor Table (GDT) management.
use core::alloc::Layout;

use crate::mp::{Constructor, CoreLocal};
use alloc::alloc::{Allocator, Global};
use cake::RwLockReadGuard;

use x86_64::{
    VirtAddr,
    instructions::{segmentation::Segment, tables::load_tss},
    registers::segmentation::{CS, DS, ES, SS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
};

/// The index in the Interrupt Stack Table (IST) for interrupts that require a separate stack.
pub const IST_FAULT_INDEX: u16 = 0;

/// A structure that holds the segment selectors for a GDT.
#[derive(Debug)]
pub struct Selectors {
    /// The kernel code segment selector.
    pub kernel_code: SegmentSelector,
    /// The kernel data segment selector.
    pub kernel_data: SegmentSelector,
    /// The user code segment selector.
    pub user_code: SegmentSelector,
    /// The user data segment selector.
    pub user_data: SegmentSelector,
    /// The TSS segment selector.
    pub tss_selector: SegmentSelector,
}

/// A structure that holds the GDT and TSS for a core.
#[derive(Debug)]
pub struct DescriptorState {
    /// The Global Descriptor Table.
    pub gdt: GlobalDescriptorTable,
    /// The segment selectors for this GDT.
    pub selectors: Selectors,
    /// The TSS for this GDT.
    pub tss: &'static TaskStateSegment,
    /// The interrupt stack pointer.
    interrupt_stack: *mut u8,
}

// Use a u16 to force 2 byte alignment for the stacks.

impl DescriptorState {
    /// Create a new GDT for the bootstrap processor.
    /// # Safety
    /// The caller must ensure this is called once and only once.
    #[allow(static_mut_refs)] // This will be upheld by the safety contract.
    unsafe fn create_bsp() -> Self {
        static mut CORE0: [u16; crate::STACK_SIZE as usize / 2] =
            [0; crate::STACK_SIZE as usize / 2];
        static mut CORE0_TSS: TaskStateSegment = TaskStateSegment::new();
        let mut gdt = GlobalDescriptorTable::new();

        let mut tss = TaskStateSegment::new();
        let interrupt_stack = unsafe { CORE0.as_mut_ptr().add(CORE0.len() * 2).cast() };
        tss.interrupt_stack_table[IST_FAULT_INDEX as usize] = VirtAddr::from_ptr(interrupt_stack);
        unsafe { CORE0_TSS = tss };

        let k_code = gdt.append(Descriptor::kernel_code_segment());
        let k_data = gdt.append(Descriptor::kernel_data_segment());
        let u_code = gdt.append(Descriptor::user_code_segment());
        let u_data = gdt.append(Descriptor::user_data_segment());

        let tss_selector = gdt.append(Descriptor::tss_segment(unsafe { &CORE0_TSS }));

        DescriptorState {
            gdt,
            selectors: Selectors {
                kernel_code: k_code,
                kernel_data: k_data,
                user_code: u_code,
                user_data: u_data,
                tss_selector,
            },
            tss: unsafe { &mut CORE0_TSS },
            interrupt_stack,
        }
    }

    unsafe fn for_core() -> Self {
        let mut gdt = GlobalDescriptorTable::new();

        // Use the allocation API to explicitly allocate memory for the stack and TSS. This removes any weird indirection when using `Box::leak` or other methods.
        // These do not need to be deallocated as they are per-core structures that exist for the lifetime of the kernel.
        let stack = Global
            .allocate(Layout::from_size_align(crate::STACK_SIZE as usize, 2).unwrap())
            .unwrap()
            .as_ptr()
            .cast::<u8>();

        let tss = Global
            .allocate(Layout::new::<TaskStateSegment>())
            .unwrap()
            .as_ptr()
            .cast::<TaskStateSegment>();

        let tss = unsafe {
            tss.write(TaskStateSegment::new());
            &mut *tss
        };

        tss.interrupt_stack_table[IST_FAULT_INDEX as usize] =
            VirtAddr::from_ptr(unsafe { stack.add(crate::STACK_SIZE as usize) });

        let k_code = gdt.append(Descriptor::kernel_code_segment());
        let k_data = gdt.append(Descriptor::kernel_data_segment());
        let u_code = gdt.append(Descriptor::user_code_segment());
        let u_data = gdt.append(Descriptor::user_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(tss));

        DescriptorState {
            gdt,
            selectors: Selectors {
                kernel_code: k_code,
                kernel_data: k_data,
                user_code: u_code,
                user_data: u_data,
                tss_selector,
            },
            tss,
            interrupt_stack: stack,
        }
    }
}

/// A per-core Global Descriptor Table (GDT).
#[derive(Debug)]
pub struct LocalGdt {
    gdt: CoreLocal<DescriptorState>,
}

impl LocalGdt {
    /// Create a new LocalGdt structure.
    pub fn new() -> Self {
        LocalGdt {
            gdt: CoreLocal::new(
                unsafe { DescriptorState::create_bsp() },
                Constructor(|| unsafe { DescriptorState::for_core() }),
            ),
        }
    }

    /// Get a read-only reference to the local GDT.
    pub fn get(&self) -> RwLockReadGuard<'_, DescriptorState> {
        self.gdt.read()
    }

    /// Load the local GDT into the CPU's GDT register. This only needs to be done once per core.
    ///
    /// # Safety
    /// The caller must ensure that no interrupts occur while the GDT is being modified.
    pub unsafe fn load(&'static self) {
        let gdt = self.gdt.read();

        unsafe {
            (&*(&gdt.gdt as *const GlobalDescriptorTable)).load();
            CS::set_reg(gdt.selectors.kernel_code);
            DS::set_reg(gdt.selectors.kernel_data);
            SS::set_reg(gdt.selectors.kernel_data);
            ES::set_reg(gdt.selectors.kernel_data);
            load_tss(gdt.selectors.tss_selector);
        }
    }
}
