//! `NATIVE_ENTRY_ROOM_CI_ISSUED_WORLD_V1`: the committed native entry-room source carries no
//! WorldId and is bound only to a canonical WorldId issued by the owning Platform World Registry
//! (owner decision, #162). The ignored test consumes the receipt of the disposable issuer run by
//! `tools/qualification/native_entry_room/run.sh`; it fails closed when the receipt is absent.
#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use oteryn_game_server::foundation::WorldId;
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

const RECEIPT_ENV: &str = "OTERYN_NATIVE_ENTRY_TOPOLOGY_RECEIPT";

fn parse_uuid_v7(value: &str) -> WorldId {
    let canonical = value.len() == 36
        && value.char_indices().all(|(index, character)| match index {
            8 | 13 | 18 | 23 => character == '-',
            _ => matches!(character, '0'..='9' | 'a'..='f'),
        })
        && value.as_bytes()[14] == b'7'
        && matches!(value.as_bytes()[19], b'8' | b'9' | b'a' | b'b');
    assert!(canonical, "not a canonical lower-case UUIDv7: {value:?}");
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("hex");
    }
    WorldId::decode(&bytes).expect("UUIDv7 WorldId")
}

fn temp_parent(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "oteryn-native-entry-room-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp parent");
    path
}

/// Writes the bound room to disk, captures it through the explicit native admission twice and
/// compiles the deterministic ordinary-release pair.
fn qualify_bound_room(world_id: WorldId, label: &str) -> NativeEntryProject {
    let documents = native_entry_room_documents(world_id).expect("bound native entry room");
    let parent = temp_parent(label);
    for (locator, bytes) in documents.documents() {
        let path = parent.join("project").join(locator);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, bytes).expect("write");
    }
    let first = capture_native_entry_project(&parent, OsStr::new("project")).expect("capture");
    let second = capture_native_entry_project(&parent, OsStr::new("project")).expect("recapture");
    fs::remove_dir_all(parent).expect("cleanup");
    assert_eq!(first, second);
    assert_eq!(first.source().world_id, world_id);
    let pair = compile_first_production(
        first.source(),
        FirstProductionCompileTarget::OrdinaryRelease,
    )
    .expect("ordinary release pair");
    let again = compile_first_production(
        second.source(),
        FirstProductionCompileTarget::OrdinaryRelease,
    )
    .expect("recompiled pair");
    assert_eq!(pair.server_digest(), again.server_digest());
    assert_eq!(pair.client_digest(), again.client_digest());
    first
}

#[test]
fn committed_room_source_carries_no_world_id() {
    let source: Value =
        serde_json::from_slice(NATIVE_ENTRY_ROOM_SOURCE).expect("committed room source JSON");
    assert_eq!(source["schema"], NATIVE_ENTRY_ROOM_SOURCE_SCHEMA);
    let world = source["world"].as_object().expect("one world object");
    assert!(!world.contains_key("world_id"));
    assert!(!String::from_utf8_lossy(NATIVE_ENTRY_ROOM_SOURCE).contains("world_id"));
}

#[test]
fn committed_room_binds_any_world_id_deterministically() {
    // Mechanics only: a locally chosen UUIDv7 is never issuance proof (#937).
    let world_id = parse_uuid_v7("01890f4c-3b2a-7c01-8d11-9a321b7c0002");
    let first = native_entry_room_documents(world_id).expect("bound room");
    let second = native_entry_room_documents(world_id).expect("rebound room");
    assert_eq!(first.documents(), second.documents());
    assert_eq!(first.documents().len(), 11);
    let project = qualify_bound_room(world_id, "local");
    // #162 A4-a: the three room Terrain cells plus the door's own walkable Terrain cell.
    assert_eq!(
        project.source().cells.len(),
        NATIVE_ENTRY_CELLS + NATIVE_ENTRY_DOOR_CELLS
    );
    // DECISION_REQUIRED (r4120444680): `door()` stays genuinely, fully linked with no fabricated
    // placement — see `NativeEntryProject::door()`'s doc comment.
    assert!(project.door().placements.is_empty());
    let other = parse_uuid_v7("01890f4c-3b2a-7c01-8d11-9a321b7c0009");
    assert_ne!(
        native_entry_room_documents(other)
            .expect("other room")
            .documents(),
        first.documents()
    );
}

#[test]
#[ignore = "requires the Platform-issued receipt from tools/qualification/native_entry_room/run.sh"]
fn platform_issued_world_binds_and_qualifies_the_committed_room() {
    let raw = std::env::var(RECEIPT_ENV)
        .unwrap_or_else(|_| panic!("{RECEIPT_ENV} must hold the Platform issuer receipt"));
    let receipt: Value = serde_json::from_str(&raw).expect("receipt JSON");
    let fields = receipt.as_object().expect("receipt object");
    let mut keys: Vec<&str> = fields.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["channel_id", "issuer", "purpose", "version", "world_id"]
    );
    assert_eq!(receipt["version"], 1);
    assert_eq!(
        receipt["purpose"],
        "disposable-preproduction-native-topology"
    );
    assert_eq!(receipt["issuer"], "oteryn-platform-world-registry");
    let world = receipt["world_id"].as_str().expect("world_id");
    let channel = receipt["channel_id"].as_str().expect("channel_id");
    parse_uuid_v7(channel);
    assert_ne!(world, channel, "WorldId and ChannelId must stay distinct");
    let world_id = parse_uuid_v7(world);
    qualify_bound_room(world_id, "issued");
}
