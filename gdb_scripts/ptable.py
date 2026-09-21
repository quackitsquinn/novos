import gdb
from itertools import batched

def pte_phys_addr(pte, offset=0):
     res =  (pte & 0x000ffffffffff000) | (offset & 0xfff)
     return res

def pte_format_flags(pte):
    flags = "|"
    flags += "R" if pte & 0b1 else "*"
    flags += "W" if pte & 0b10 else "*"
    flags += "*" if pte & 0b1 << 63 else "X"
    flags += "|"
    flags += "U" if pte & 0b100 else "*"
    flags += "P" if pte & 0b1 << 7 else "*"
    flags += "|"
    flags += "G" if pte & 0b1 << 8 else "*"
    flags += "|"

    return flags

class PTable(gdb.Command):
    def __init__(self):
        super().__init__("ptable", gdb.COMMAND_DATA)

    def invoke(self, arg, from_tty):
        args = arg.split()
        if len(args) < 1:
            print("Usage: ptable <address> <optional: start_index> <optional: end_index>")
            print("Prefix the address with pte: to handle it as a page table entry")
            return

        if len(args) >= 2:
            start_index = int(gdb.parse_and_eval(args[1]))
        else:
            start_index = 512 - 16;

        if len(args) >= 3:
            end_index = int(gdb.parse_and_eval(args[2]))
        else:
            end_index = 512

        res = self.extract_address(args[0])
        if res is None:
            return
        address, address_explicit = res
        if not address_explicit:
            print(f"Using address {hex(address)} extracted from {args[0]}")
        print((" " * 25) + "|RWX|UP|G|")
        print((" " * 9) + "|Addr" + (" " * 11) + "|   |SS| |", end="")
        print()
        for index in batched(range(start_index, end_index), 4):
            print(f"[{index[0]:>3}-{index[-1]:>3}]|", end="")
            line = ""
            for i in index:
                entry_str = self.print_entry(address, i)
                line += f"{entry_str} | ".center(15)
            print(line[:-3])  # Remove the last " | "


    def print_entry(self, address, index):
        entry_value = self.read_table_entry(address, index)
        flags = pte_format_flags(entry_value)
        addr = pte_phys_addr(entry_value)
        return f"{addr:#014x} {flags}"
        


    def read_table_entry(self, address, index) -> int:
        entry_address = address + index * 8
        gdb.execute("maintenance packet Qqemu.PhyMemMode:1", to_string=True)
        entry_value = gdb.selected_inferior().read_memory(entry_address, 8)
        gdb.execute("maintenance packet Qqemu.PhyMemMode:0", to_string=True)
        return int.from_bytes(entry_value, byteorder='little')



    def extract_address(self, arg):
        if arg == "cr3":
            try:
                cr3 = gdb.parse_and_eval("$cr3")
                return pte_phys_addr(int(cr3)), False
            except Exception as e:
                print(f"Error parsing CR3 register: {e}")
                return None
            
        if arg.startswith("pte:"):
            pte_str = arg[4:]
            try:
                pte = gdb.parse_and_eval(pte_str)
                return pte_phys_addr(int(pte)), False
                
            except Exception as e:
                print(f"Error parsing page table entry: {e}")
                return None
        else:
            try:
                address = gdb.parse_and_eval(arg)
                return address, True
            except Exception as e:
                print(f"Error parsing address: {e}")
                return None


        

PTable()