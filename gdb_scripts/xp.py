import gdb

class Xp(gdb.Command):
    def __init__(self):
        super().__init__("xp", gdb.COMMAND_DATA)

    def invoke(self, arg, from_tty):
        gdb.execute("maintenance packet Qqemu.PhyMemMode:1", to_string=True)
        try:
            gdb.execute("x " + arg)
        finally:
            gdb.execute("maintenance packet Qqemu.PhyMemMode:0", to_string=True)

Xp()