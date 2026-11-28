//! BUNDLE-BUILD-1: the committed World pin's real artifact, built by `pin-check`, passes the node's
//! boot checks (`map::boot::check`) with the pins a preproduction `[world_bundle]` carries.
//! Booting it needs the generation's Item definitions, so that stage is reported, not asserted.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::BundlePins;
use oteryn_game_server::map::boot::{BootPins, check};
use oteryn_game_server::map::overlay::TilePos;
use serde_json::Value;

#[test]
fn the_pinned_artifact_passes_the_node_boot_checks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("wb-artifact-boot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let mut compiler = if let Some(binary) = std::env::var_os("OTERYN_WORLD_COMPILER_BIN") {
        let binary = PathBuf::from(binary);
        assert_eq!(
            binary,
            binary.canonicalize().expect("prebuilt compiler exists"),
            "prebuilt compiler path must be absolute and contain no symlinks"
        );
        assert_eq!(
            binary.file_name().and_then(|name| name.to_str()),
            Some("oteryn-world-bundle-compiler")
        );
        let directory = binary.parent().expect("prebuilt compiler directory");
        assert_eq!(
            directory.file_name().and_then(|name| name.to_str()),
            Some("world-compiler")
        );
        // Reuse the producer's source/workspace/variant/SHA256 verifier immediately
        // before pin-check. A supplied stale artifact must fail, never rebuild silently.
        let verified = Command::new("python3")
            .current_dir(&root)
            .arg(root.join("tools/qualification/node_boot/prebuilt.py"))
            .args(["verify", "world-compiler"])
            .arg(directory.parent().expect("prebuilt bundle root"))
            .status()
            .expect("verify prebuilt World compiler");
        assert!(
            verified.success(),
            "prebuilt World compiler source and checksum"
        );
        Command::new(binary)
    } else {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let mut command = Command::new(cargo);
        command.args([
            "run",
            "--locked",
            "--release",
            "-p",
            "oteryn-world-bundle-compiler",
            "--",
        ]);
        command
    };
    let status = compiler
        .current_dir(&root)
        .args(["pin-check", "."])
        .arg(&out)
        .status()
        .expect("pin-check runs");
    assert!(status.success(), "pin-check");
    let pin: Value = serde_json::from_slice(
        &std::fs::read(root.join("content/world/pins/oteryn.json")).unwrap(),
    )
    .unwrap();
    let hex = pin["digest"].as_str().unwrap();
    let mut digest = [0u8; 32];
    for (i, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    let data = std::fs::read(out.join(format!("oteryn-{hex}.otwb"))).unwrap();
    let text = |key: &str| pin[key].as_str().unwrap().to_owned();
    let start = &pin["entry_start"];
    let pins = BootPins {
        bundle: BundlePins {
            digest,
            project_format_version: text("project_format_version"),
            world_schema_version: text("world_schema_version"),
            content_revision: text("content_revision"),
            production: false,
        },
        map_revision: format!("sha256:{hex}"),
        start: TilePos {
            x: start["x"].as_u64().unwrap() as u16,
            y: start["y"].as_u64().unwrap() as u16,
            floor: -(start["floor"].as_i64().unwrap() as i8).abs(),
        },
    };
    // `check` compares `map_revision` with the bundle's own `sha256:<digest>` and the start tile.
    let checked = check(data, pins).expect("the pinned artifact passes the boot checks");
    let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).unwrap();
    let channel =
        ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2]).unwrap();
    // With no served Item definitions the boot result names the precondition that is missing.
    eprintln!(
        "boot without Item definitions: {:?}",
        checked.boot(world, channel, |_| None).map(|_| ())
    );
    let _ = std::fs::remove_dir_all(&out);
}
