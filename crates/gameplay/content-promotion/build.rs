fn main() {
    // CONTENT owns the shared adapters' tests; recognize its cfg without enabling it here.
    println!("cargo::rustc-check-cfg=cfg(oteryn_gameplay_content_unit_tests)");
}
