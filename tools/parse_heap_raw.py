
class Block:
    def __init__(self, size: int,  is_free: bool, addr: int, is_reuseable: bool):
        self.size = size
        self.is_free = is_free
        self.addr = addr
        self.is_reuseable = is_reuseable

class HeapRepr:
    def __init__(self, filename: str):
        self.bytes = list(open(filename, 'rb').read())
        assert len(self.bytes) % 0x20 == 0
        self.parse()
    def parse(self):
        self.blocks = []
        for i in range(0, len(self.bytes), 0x20):
            buf = self.bytes[i:i+0x20]
            size = int.from_bytes(buf[0:8], byteorder='little')
            is_free = bool(buf[8])
            addr = int.from_bytes(buf[9:17], byteorder='little')
            is_reuseable = bool(buf[17])
            self.blocks.append(Block(size, is_free, addr, is_reuseable))
    def __str__(self):
        return '\n'.join([f"size: {block.size} free: {block.is_free} addr: {hex(block.addr)} can reuse: {block.is_reuseable}" for block in self.blocks])
            
            



if __name__ == '__main__':
    print("=== MAIN HEAP ===")
    print(HeapRepr("output/heap.raw"))
    print("=== TEST HEAP ===")
    print(HeapRepr("output/test_heap.raw"))
