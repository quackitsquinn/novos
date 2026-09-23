import gdb
import phys

class Xp(gdb.Command):
    def __init__(self):
        super().__init__("xp", gdb.COMMAND_DATA)

    def invoke(self, arg, from_tty):
        phys.enable_phys_access()
        try:
            gdb.execute("x " + arg)
        finally:
            phys.disable_phys_access()

Xp()