use core::convert::Infallible;

use lazy_static::lazy_static;
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{Segment, CS, DS, ES, SS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::util::KernelModule;

pub const IST_FAULT_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[IST_FAULT_INDEX as usize] = {
            const STACK_SIZE: u64 = crate::STACK_SIZE;
            static mut STACK: [u8; STACK_SIZE as usize] = [0; STACK_SIZE as usize];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kcode = gdt.append(Descriptor::kernel_code_segment());
        let kdata = gdt.append(Descriptor::kernel_data_segment());
        let tss = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector: kcode,
                data_selector: kdata,
                tss_selector: tss,
            },
        )
    };
}

pub static MODULE: KernelModule<Infallible> = KernelModule::new("gdt", init);

fn init() -> Result<(), Infallible> {
    GDT.0.load();

    unsafe {
        CS::set_reg(GDT.1.code_selector);
        SS::set_reg(GDT.1.data_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
    Ok(())
}

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}
