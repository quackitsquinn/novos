import gdb
import phys
import table
from itertools import batched


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
        entry_value = table.read_entry(address, index)
        flags = table.pte_flags(entry_value)
        flags = flags.format_with("|{P}{W}{X}|{U}{H}|{G}|")
        addr = table.pte_phys_addr(entry_value)
        return f"{addr:#014x} {flags}"
        


    def read_table_entry(self, address, index) -> int:
        entry_address = address + index * 8
        return int.from_bytes(phys.read(entry_address, 8), byteorder='little')



    def extract_address(self, arg):
        if arg == "cr3":
            try:
                return table.read_l4_addr(), False
            except Exception as e:
                print(f"Error parsing CR3 register: {e}")
                return None
            
        if arg.startswith("pte:"):
            pte_str = arg[4:]
            try:
                pte = gdb.parse_and_eval(pte_str)
                return table.pte_phys_addr(int(pte)), False
                
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