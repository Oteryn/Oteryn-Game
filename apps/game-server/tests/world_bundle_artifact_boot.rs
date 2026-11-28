//! BUNDLE-BUILD-1: the committed World pin's real artifact, built by `pin-check`, passes the node's
//! boot checks (`map::boot::check`) with the pins a preproduction `[world_bundle]` carries.
//! Also compile the qualification town and load it with the production generation's Item index.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::BundlePins;
use oteryn_game_server::map::boot::{BootPins, check};
use oteryn_game_server::map::overlay::TilePos;
use serde_json::Value;

fn world_compiler(root: &Path) -> Command {
    if let Some(binary) = std::env::var_os("OTERYN_WORLD_COMPILER_BIN") {
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
            .current_dir(root)
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
    }
}

#[test]
fn the_pinned_artifact_passes_the_node_boot_checks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("wb-artifact-boot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let mut compiler = world_compiler(&root);
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

#[test]
fn compiled_town_bundle_loads_with_the_runtime_item_index() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use oteryn_game_server::content::{
                ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
                activate_native_entry_room_with_gameplay, native_gameplay::NativeGameplayInput,
                qualify_selected_native_gameplay_room,
            };
            use oteryn_game_server::map::boot::native_item_definition;

            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let out =
                std::env::temp_dir().join(format!("wb-town-boot-{}.otwb", std::process::id()));
            let regions = std::fs::read_to_string(
                root.join("tools/qualification/node_boot/town-regions.txt"),
            )
            .expect("town regions");
            let output = world_compiler(&root)
                .current_dir(&root)
                .args([
                    "compile-town-qualification",
                    ".",
                    "content/world/pins/oteryn.identity.json",
                ])
                .arg(&out)
                .arg(regions.trim())
                .output()
                .expect("run town compiler");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: Value = serde_json::from_slice(&output.stdout).expect("compiler report");
            let data = std::fs::read(&out).expect("compiled town");
            std::fs::remove_file(&out).expect("remove town artifact");
            let key = "oteryn:world-object.tibia.i20661";
            let visited = oteryn_world_bundle::bundle::visit(&data, |_, _| Ok(()))
                .expect("read town manifest");
            assert!(
                visited
                    .manifest
                    .palette
                    .iter()
                    .any(|entry| entry.key == key)
            );
            let pin: Value = serde_json::from_slice(
                &std::fs::read(root.join("content/world/pins/oteryn.json"))
                    .expect("read town pins"),
            )
            .expect("parse town pins");
            let text = |key: &str| pin[key].as_str().expect("committed town pin").to_owned();
            let hex = report["digest"].as_str().expect("town digest");
            let mut digest = [0u8; 32];
            assert_eq!(hex.len(), digest.len() * 2);
            for (index, byte) in digest.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                    .expect("hex town digest");
            }
            let start = TilePos {
                x: 32369,
                y: 32238,
                floor: -7,
            };
            let checked = check(
                data,
                BootPins {
                    bundle: BundlePins {
                        digest,
                        project_format_version: text("project_format_version"),
                        world_schema_version: text("world_schema_version"),
                        content_revision: text("content_revision"),
                        production: false,
                    },
                    map_revision: format!("sha256:{hex}"),
                    start,
                },
            )
            .expect("town pins and geometry");

            let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
                .expect("world id");
            let channel =
                ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
                    .expect("channel id");
            let input =
                NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
                    .expect("native gameplay manifest");
            let room = qualify_selected_native_gameplay_room(world, &input).expect("native room");
            let issuance = NativeEntryActivationIssuance {
                world_id: world,
                activation_sequence: 1,
                server_artifact_digest: room.compiled().server_digest(),
                client_artifact_digest: room.compiled().client_digest(),
                frame_binding_digest: room.frame_binding().digest(),
            };
            drop(room);
            let mut controller = ContentActivationController::new();
            activate_native_entry_room_with_gameplay(
                &mut controller,
                &NodeBootQuiescence::before_channel_runtime(),
                world,
                &issuance,
                &input,
            )
            .expect("activate the runtime generation");
            let native = controller
                .active()
                .and_then(|active| active.native_gameplay())
                .expect("runtime Item index");
            let town = checked
                .boot(world, channel, |key| {
                    native_item_definition(Some(native), key)
                })
                .expect("compiler palette must load through the runtime");
            assert!(town.enterable(start));
        })
        .expect("start town boot thread")
        .join()
        .expect("town boot thread passes");
}
