import gdb


def read(address: int, size: int) -> memoryview[int]:
    """Read physical memory at the given address."""
    enable_phys_access()
    try:
        return gdb.selected_inferior().read_memory(address, size)
    finally:
        disable_phys_access()


def write(address: int, data: bytes) -> None:
    """Write physical memory at the given address."""
    enable_phys_access()
    try:
        gdb.selected_inferior().write_memory(address, data)
    finally:
        disable_phys_access()


def enable_phys_access() -> None:
    """Enable physical memory access in QEMU."""
    gdb.execute("maintenance packet Qqemu.PhyMemMode:1", to_string=True)

def disable_phys_access() -> None:
    """Disable physical memory access in QEMU."""
    gdb.execute("maintenance packet Qqemu.PhyMemMode:0", to_string=True)