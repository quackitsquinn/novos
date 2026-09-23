from enum import IntFlag
import phys
import gdb

PRESENT_SHORT = "R"
WRITABLE_SHORT = "W"
USER_SHORT = "US"
EXEC_SHORT = "X"
HUGE_SHORT = "H"
GLOBAL_SHORT = "G"

class PteFlags(IntFlag):
    PRESENT       = 1 << 0
    WRITABLE      = 1 << 1
    USER          = 1 << 2
    WRITE_THROUGH = 1 << 3
    CACHE_DISABLE = 1 << 4
    ACCESSED      = 1 << 5
    DIRTY         = 1 << 6
    HUGE          = 1 << 7
    GLOBAL        = 1 << 8

    # bits 9-11 are available for software use

    NX            = 1 << 63

    def format_with(self, formatter: str) -> str:
        """Format the flags as a string using the given formatter."""
        return formatter.format(
            P=PRESENT_SHORT[0] if self & self.PRESENT else "*",
            W=WRITABLE_SHORT[0] if self & self.WRITABLE else "*",
            U=USER_SHORT[0] if self & self.USER else "*",
            H=HUGE_SHORT[0] if self & self.HUGE else "*",
            G=GLOBAL_SHORT[0] if self & self.GLOBAL else "*",
            X=EXEC_SHORT[0] if not self & self.NX else "*",
        )

def pte_flags(pte: int) -> PteFlags:
    """Extract the flags from a page table entry."""
    return PteFlags(pte & 0x8000000000000fff)

def pte_phys_addr(pte: int, offset: int = 0) -> int:
     """Extract the physical address from a page table entry."""
     res =  (pte & 0x000ffffffffff000) | (offset & 0xfff)
     return res

def read_entry(pbase: int, entry: int) -> int:
    """Read a page table entry from physical memory."""
    if entry < 0 or entry >= 512:
        raise ValueError(f"Entry index {entry} is out of bounds (0-511)")
    entry_address = pbase + entry * 8
    return int.from_bytes(phys.read(entry_address, 8), byteorder='little')

def write_entry(pbase: int, entry: int, paddr: int, flags: PteFlags) -> None:
    """Write a page table entry to physical memory."""
    if entry < 0 or entry >= 512:
        raise ValueError(f"Entry index {entry} is out of bounds (0-511)")
    entry_address = pbase + entry * 8
    pte_value = paddr | flags
    phys.write(entry_address, pte_value.to_bytes(8, byteorder='little'))

def read_l4_addr() -> int:
    """Read the physical address of the level 4 page table from the CR3 register."""
    cr3 = int(gdb.parse_and_eval("$cr3"))
    return pte_phys_addr(cr3)