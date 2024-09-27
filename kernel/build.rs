/// Tell rust-lld to use the linker script
const LINK_SCRIPT_PATH: &str = "kernel/link.ld";

fn main() {
    println!("cargo:rustc-link-arg=-T{}", LINK_SCRIPT_PATH);
    println!("cargo:rerun-if-changed={}", LINK_SCRIPT_PATH);
}
