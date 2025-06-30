const LINK_SCRIPT_PATH: &str = "trampoline/link.ld";

fn main() {
    println!("cargo:rustc-link-arg=-T{}", LINK_SCRIPT_PATH);
    println!("cargo:rerun-if-changed={}", LINK_SCRIPT_PATH);
}
