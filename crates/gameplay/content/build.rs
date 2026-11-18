fn main() {
    // Shared promotion adapters need this harness's private artifact types in their tests.
    println!("cargo::rustc-check-cfg=cfg(oteryn_gameplay_content_unit_tests)");
    println!("cargo::rustc-cfg=oteryn_gameplay_content_unit_tests");
    let content =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/services");
    let routes = content.join("conditional-npc-travel.json");
    let included = if routes.is_file() {
        println!("cargo::rerun-if-changed={}", routes.display());
        format!("include_str!({:?})", routes)
    } else {
        println!("cargo::rerun-if-changed={}", content.display());
        "\"[]\"".into()
    };
    let generated = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output"));
    std::fs::write(
        generated.join("conditional_npc_travel.rs"),
        format!("const CONDITIONAL_ROUTES: &str = {included};\n"),
    )
    .expect("write conditional NPC travel registry");
}
