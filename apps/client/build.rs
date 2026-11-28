#[path = "build_support/launcher_version.rs"]
mod launcher_version;

#[path = "build_support/play_features.rs"]
mod play_features;

#[path = "build_support/preference_bindings.rs"]
mod preference_bindings;

#[path = "build_support/prey_catalogue.rs"]
mod prey_catalogue;

#[path = "build_support/release_identity.rs"]
mod release_identity;

#[path = "build_support/spell_parameters.rs"]
mod spell_parameters;

#[path = "build_support/update_trust.rs"]
mod update_trust;

/// `SetWindowSubclass`, `DefSubclassProc` and `RemoveWindowSubclass` are exported by name only
/// from Common Controls 6, which Windows loads only for an executable whose manifest asks for it;
/// without it the client fails to start with STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139).
fn emit_common_controls_manifest() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest =
            std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("resources/windows.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed={}", manifest.display());
    }
}

fn emit_launcher_version() {
    println!("cargo:rerun-if-changed=build_support/launcher_version.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let (major, minor) = launcher_version::PE_IMAGE_VERSION;
    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => {
            println!("cargo:rustc-link-arg-bin=oteryn-launcher=/VERSION:{major}.{minor}");
        }
        Ok("gnu") => {
            println!(
                "cargo:rustc-link-arg-bin=oteryn-launcher=-Wl,--major-image-version,{major},--minor-image-version,{minor}"
            );
        }
        _ => {}
    }
}

/// The Bevy host is the Windows client, so `cfg(bevy_shell)` is on for every Windows build;
/// elsewhere the `bevy-shell` feature opts in.
fn emit_bevy_shell_cfg() -> bool {
    println!("cargo:rustc-check-cfg=cfg(bevy_shell)");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let bevy_shell =
        target_os == "windows" || std::env::var_os("CARGO_FEATURE_BEVY_SHELL").is_some();
    if bevy_shell {
        println!("cargo:rustc-cfg=bevy_shell");
    }
    bevy_shell
}

fn main() -> Result<(), String> {
    emit_common_controls_manifest();
    emit_launcher_version();
    let bevy_shell = emit_bevy_shell_cfg();
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
    if bevy_shell {
        println!("cargo:rerun-if-changed=../../content/world/placements");
        let towns = oteryn_client_assets::world::depot_town_ids(&root.join("../.."))?
            .into_iter()
            .collect::<Vec<_>>();
        std::fs::write(
            std::path::Path::new(&out).join("depot_town_ids.rs"),
            format!("{towns:?}"),
        )
        .map_err(|error| error.to_string())?;
    }
    prey_catalogue::emit(&root.join("../.."), std::path::Path::new(&out))?;
    println!("cargo:rerun-if-changed=build_support/spell_parameters.rs");
    spell_parameters::generate(root, std::path::Path::new(&out))?;
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
