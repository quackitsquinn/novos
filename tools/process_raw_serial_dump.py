

data = list(open("COM2.out", "rb").read())
index = 0

TEST_ALLOCATOR_SIGN = int.from_bytes(b"TEST", "little") << 16

def read_bytes(size: int) -> list[int]:
    global data, index
    result = data[index:index + size]
    index += size
    return result

def read_byte() -> int:
    return read_bytes(1)[0]

def err(msg):
    print(f"\033[91m{msg}\033[0m")

command = read_byte()

print(f"Command: {command}")

if command != 0x01:
    print("Invalid command")
    exit(-1)

filename_length = read_byte()

print(f"Filename length: {filename_length}")

filename = read_bytes(filename_length)

print(f"Filename: raw({filename}) actual({bytes(filename).decode('utf-8')})")

file_size = int.from_bytes(read_bytes(4), "little")

print(f"File size: {hex(file_size)}")

if file_size >= len(data):
    err("File size is too small")

class Block:
    def __init__(self, address, size, is_reuseable, is_free):
        self.size = size
        self.is_free = is_free
        self.address = address
        self.is_reuseable = is_reuseable
    def __repr__(self):
        return f"Block({hex(self.address)} ({self.address}), {self.size}, {self.is_reuseable}, {self.is_free})"

def read_block() -> Block:
    size = int.from_bytes(read_bytes(8), "little")
    is_free = read_byte() == 1
    address = int.from_bytes(read_bytes(8), "little")
    is_reuseable = read_byte() == 1
    if not address & 0xFFFF000000000000 == TEST_ALLOCATOR_SIGN:
        err(f"Invalid address: {hex(address)} (Expected {hex(TEST_ALLOCATOR_SIGN)})")
    return Block(address, size, is_reuseable, is_free)

blocks = []

while index+32 < len(data):
    blocks.append(read_block());

print(f"Read {len(blocks)} blocks")

print("Blocks:")
for block in blocks:
    print(block)