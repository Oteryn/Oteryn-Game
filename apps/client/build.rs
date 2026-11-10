#[path = "build_support/release_identity.rs"]
mod release_identity;

#[path = "build_support/play_features.rs"]
mod play_features;

#[path = "build_support/preference_bindings.rs"]
mod preference_bindings;

#[path = "build_support/update_trust.rs"]
mod update_trust;

/// `SetWindowSubclass`, `DefSubclassProc` and `RemoveWindowSubclass` are exported by name only
/// from Common Controls 6, which Windows loads only for an executable whose manifest asks for it;
/// without it the client fails to start with STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139).
fn emit_common_controls_manifest() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}

fn main() -> Result<(), String> {
    emit_common_controls_manifest();
    update_trust::emit()?;
    println!("cargo:rerun-if-changed=build_support/update_trust.rs");
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
