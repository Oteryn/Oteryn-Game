fn main() {
    // Shared promotion adapters need this harness's private artifact types in their tests.
    println!("cargo::rustc-check-cfg=cfg(oteryn_gameplay_content_unit_tests)");
    println!("cargo::rustc-cfg=oteryn_gameplay_content_unit_tests");
}
