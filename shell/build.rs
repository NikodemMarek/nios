fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg-bins=-T{}/src/linker.ld", manifest_dir);
    println!("cargo:rerun-if-changed=src/linker.ld");
}
