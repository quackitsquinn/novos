use kvmm::KernelPage;
use x86_64::{VirtAddr, structures::paging::page::PageRange};

/// The entry point name that is looked for in the kernel binary.
pub const ENTRY_PONT_NAME: &str = "_start";
/// The size of the kernel stack.
pub const STACK_SIZE: usize = 0x1_000_000;
/// The base address of the kernel stack.
pub const STACK_TOP: VirtAddr = VirtAddr::new_truncate(0x800_000_000_000);

pub const VIRTUAL_MAP_PAGE_RANGE: PageRange = KernelPage::range(
    KernelPage::containing_address(VirtAddr::new(0x6ff_fff_fff_fff)),
    KernelPage::containing_address(VirtAddr::new(0x7ff_fff_fff_fff)),
);
