import gdb
import phys
import table

class Translate(gdb.Command):
    """Translate a virtual address to a physical address using the current page tables."""

    def __init__(self):
        super().__init__("translate", gdb.COMMAND_DATA)

    def invoke(self, arg, from_tty):
        if not arg:
            print("Usage: translate <virtual_address>")
            return

        try:
            va = int(gdb.parse_and_eval(arg))
        except Exception as e:
            print(f"Error parsing virtual address: {e}")
            return

        translation = translate_address(va)
        if translation is None:
            print(f"Failed to translate virtual address {hex(va)}")
            return

        phys_addr = translation.phys_addr()
        flags = translation.flags()
        level = translation.level()
        print(f"Virtual address {hex(va)} translates to physical address {hex(phys_addr)} at level {level} with flags: {flags.format_with('{P}{W}{X}|{U}{H}|{G}')}")

class Translation:
    def __init__(self, pte: int, level: int):
        self._pte = pte
        self._level = level

    def pte(self) -> int:
        return self._pte

    def level(self) -> int:
        return self._level

    def phys_addr(self) -> int:
        return table.pte_phys_addr(self._pte)

    def flags(self) -> table.PteFlags:
        return table.pte_flags(self._pte)

def translate_address(va: int) -> Translation | None:
    """Translate a virtual address to a physical address using the current page tables."""
    # Read the CR3 register to get the base of the page table
    root = table.read_l4_addr()
    l4_index, l3_index, l2_index, l1_index = vaddr_table_indicies(va)
    root_entry = table.read_entry(root, l4_index)
    if not table.pte_flags(root_entry) & table.PteFlags.PRESENT:
        print(f"Level 4 entry not present for virtual address {hex(va)}")
        return None
    
    l3_base = table.pte_phys_addr(root_entry)
    l3_entry = table.read_entry(l3_base, l3_index)
    if not table.pte_flags(l3_entry) & table.PteFlags.PRESENT:
        print(f"Level 3 entry not present for virtual address {hex(va)}")
        return None
    if table.pte_flags(l3_entry) & table.PteFlags.HUGE:
        # 1 GiB page
        return Translation(l3_entry, 3)
    
    l2_base = table.pte_phys_addr(l3_entry)
    l2_entry = table.read_entry(l2_base, l2_index)
    if not table.pte_flags(l2_entry) & table.PteFlags.PRESENT:
        print(f"Level 2 entry not present for virtual address {hex(va)}")
        return None
    if table.pte_flags(l2_entry) & table.PteFlags.HUGE:
        # 2 MiB page
        return Translation(l2_entry, 2)

    l1_base = table.pte_phys_addr(l2_entry)
    l1_entry = table.read_entry(l1_base, l1_index)
    if not table.pte_flags(l1_entry) & table.PteFlags.PRESENT:
        print(f"Level 1 entry not present for virtual address {hex(va)}")
        return None
    
    return Translation(l1_entry, 1)


TABLE_INDEX_BITS = 9
ENTRY_OFFSET_BITS = 12

def vaddr_table_indicies(va: int) -> tuple[int, int, int, int]:
    """Extract the page table indices from a virtual address."""
    mask = (1 << TABLE_INDEX_BITS) - 1
    l4 = (va >> (TABLE_INDEX_BITS * 3 + ENTRY_OFFSET_BITS)) & mask
    l3 = (va >> (TABLE_INDEX_BITS * 2 + ENTRY_OFFSET_BITS)) & mask
    l2 = (va >> (TABLE_INDEX_BITS + ENTRY_OFFSET_BITS)) & mask
    l1 = (va >> ENTRY_OFFSET_BITS) & mask
    return l4, l3, l2, l1


if __name__ == "__main__":
    Translate()