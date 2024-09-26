#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::hint::black_box;

use kernel::{
    display::{self, color::Color, terminal},
    interrupts::hardware::timer,
    memory::{self, allocator::get_block_allocator},
    println, sprintln, terminal,
};
use log::{error, log_enabled, trace};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    if kernel::display_init() {
        println!("uh oh, the code {}", _info);
    }
    kernel::hlt_loop();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use kernel::memory::allocator;

    kernel::init_kernel();

    sprintln!("Initialized kernel");
    x86_64::instructions::interrupts::enable();
    sprintln!("Enabled interrupts");
    loop {
        create_arr_check_free();
        assert!(allocator::get_allocation_balance() == 0);
    }
    kernel::hlt_loop();
    memory::allocator::output_blocks();
}

#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    test_main();
    kernel::hlt_loop();
}

static mut COUNTER: u32 = 0;

fn create_arr_check_free() {
    // Make sure this doesn't get optimized out
    let mut t: alloc::vec::Vec<u32> = alloc::vec![0];
    for i in 0..10 {
        t.push(unsafe { COUNTER });
        sprintln!("Pushed {}", unsafe { COUNTER });
        unsafe {
            COUNTER += 1;
        }
    }
    let blocks = get_block_allocator();
    let bt = blocks.get_block_table();
    bt.iter().enumerate().for_each(|(i, block)| {
        if block == blocks.get_table_block() {
            trace!("Table Block {:#X}: {:?}", i, block);
        } else {
            trace!("Block {:#X}: {:?}", i, block);
        }
    });

    drop(bt);
    drop(blocks);
    drop(black_box(t));

    let blocks = get_block_allocator();
    let bt = blocks.get_block_table();

    let mut failed = false;
    // Assert that all blocks are marked as free. (This should be true because we free the vector)
    bt.iter().enumerate().for_each(|(i, block)| {
        if block.is_free {
            trace!("Block {:#X}: {:?}", i, block);
        } else if block == blocks.get_table_block() {
            trace!("Table Block {:#X}: {:?}", i, block);
        } else {
            error!("Block {:#X}: {:?}", i, block);
            failed = true;
        }
    });

    if failed {
        panic!("Failed to free all blocks");
    }
}
