use std::{env::args, path::Path};

fn main() {
    if args().len() == 1 {
        println!("No arguments provided");
    } else {
        if args().nth(1).unwrap() == "parse_heap" {
            parse_heap();
        }
    }
}

pub fn parse_heap() {
    let main_heap = Path::new("output/heap.raw");
    if !main_heap.exists() {
        eprintln!("Main heap file does not exist");
    } else {
        let heap = std::fs::read("output/heap.raw").unwrap();
        assert!(
            heap.len() % std::mem::size_of::<Block>() == 0,
            "Heap file is not a multiple of Block size"
        );
        let blocks = unsafe {
            std::slice::from_raw_parts(
                heap.as_ptr() as *const Block,
                heap.len() / std::mem::size_of::<Block>(),
            )
        };
        for block in blocks {
            println!("{:?}", block);
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Block {
    // The size of the block
    pub size: usize,
    //  Is the block free or allocated
    pub is_free: bool,
    // The start address of the block
    pub address: *mut u8,
}
