#[path = "build_support/release_identity.rs"]
mod release_identity;

#[path = "build_support/play_features.rs"]
mod play_features;

#[path = "build_support/preference_bindings.rs"]
mod preference_bindings;

fn main() -> Result<(), String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let root = std::path::Path::new(&manifest);
    release_identity::emit(root)?;
    println!("cargo:rerun-if-changed=build_support/release_identity.rs");
    println!("cargo:rerun-if-changed=src/features");
    println!("cargo:rerun-if-changed=build_support/play_features.rs");
    println!("cargo:rerun-if-changed=../../crates/protocol-oteryn/capabilities");
    let (generated, tests) =
        play_features::generate(&root.join("src/features"), &root.join("../.."))?;
    println!("cargo:rerun-if-changed=../../crates/client-ui/src/features");
    let bindings = preference_bindings::generate(&root.join("../.."))?;
    let out = std::env::var("OUT_DIR").map_err(|e| e.to_string())?;
    std::fs::write(
        std::path::Path::new(&out).join("preference_bindings.rs"),
        bindings,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        std::path::Path::new(&out).join("play_features.rs"),
        generated,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        std::path::Path::new(&out).join("play_feature_tests.rs"),
        tests,
    )
    .map_err(|e| e.to_string())
}
