//! Reserve a bounded main-thread stack in the Windows MSVC executable itself.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    // Use the compilation target, not the host executing this build script.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // The unoptimised CLI overflows the default 1 MiB Windows main stack.
        // Reserve 8 MiB without increasing the initial committed allocation.
        // Scope this to fyi-cli, not dependency libraries or test executables.
        println!("cargo::rustc-link-arg-bin=fyi-cli=/STACK:8388608");
    }
}
