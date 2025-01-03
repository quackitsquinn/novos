# novos: A little kernel written in rust. (so I can learn osdev)


## Running the kernel

To run the kernel, you need to have the following dependencies installed:

- `qemu`
- `xorriso`
- `rustc` and `cargo`

Then, you can run the kernel with the following command from the root of the repository:

```sh
cargo run --bin bios_boot
```

(This should be right, but I also haven't had to set up a new machine in a while, so I might be missing something)


## Debugging the kernel

All debugging is done through environment variables. The following environment variables are supported:

TODO: There's a few more (look in kbuild/src/config.rs for most of the new ones) so update this list

- `ALLOC_DEBUG`: If set, will print out information about memory allocations and deallocations.
- `DEBUG`: If set, QEMU will wait for a debugger to attach on port 1234.
- `QEMU_FLAGS`: Additional flags to pass to QEMU.
- `VERBOSE`: Shows QEMU's invocation command.
- `NO_DISPLAY`: Disables the graphical display in QEMU.
- `KERNEL_MEM`: Sets the amount of memory to allocate for the kernel in QEMU.