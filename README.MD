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