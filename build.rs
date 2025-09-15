fn main() {
    // Simple build script - no Windows resources needed
    println!("cargo:rerun-if-changed=build.rs");
}
