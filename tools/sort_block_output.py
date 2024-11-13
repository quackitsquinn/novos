import csv


with open('alloc_out.csv', 'r') as f:
    reader = csv.reader(f)
    data = list(reader)

data.pop(0)  # Remove the header row

# Convert the hex representation of the block number to an integer
data = [[i, int(x[0], 16), int(x[1]), x[2] == "true", x[3] == "true"] for i,x in enumerate(data)]

class Block:
    def __init__(self, id, block_num, size, is_allocated, is_free):
        self.id = id
        self.address = block_num
        self.size = size
        self.is_allocated = is_allocated
        self.is_free = is_free

    def is_next_to(self, other) -> bool:
        self_end = self.address + self.size
        return self_end == other.address or self.address == other.address + other.size
    def contains_address(self, address) -> bool:
        return self.address <= address and address < self.address + self.size
    def __repr__(self):
        return f"Block({hex(self.address)} ({self.address}), {self.size}, {self.is_allocated}, {self.is_free}): {self.id}"

data = [Block(*x) for x in data]

sorted_data = data.copy()
sorted_data.sort(key=lambda x: x.address)



print(f"Does sorted data match original data? {sorted_data == data}")

for i in range(len(sorted_data)):
    last_block = sorted_data[i - 1] if i > 0 else None
    block = sorted_data[i]
    next_block = sorted_data[i + 1] if i < len(sorted_data) - 1 else None

    if last_block and last_block.is_next_to(block):
        print(f"Block {block} is next to block {last_block}")
        continue
    if next_block and block.is_next_to(next_block):
        print(f"Block {block} is next to block {next_block}")
        continue
    print(f"Block {block} is not next to any other block")
    

    