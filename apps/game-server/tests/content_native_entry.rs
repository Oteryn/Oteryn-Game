//! `NATIVE_ENTRY_SOURCE_CONSUMER_V1`: explicit native admission, typed overlay lowering and the
//! deterministic FirstProduction pair (#937 §5/§7, #940). Every negative case changes one
//! invariant of an otherwise valid project.
#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const WORLD_ID: &str = "0123456789ab70cd8ef0123456789abc";
const FRAME: &str = "oteryn:entry.frame";
const REV: &str = "oteryn:rev/entry-r1";

fn limits() -> ProjectEvidenceLimits {
    native_entry_first_slice_limits().project
}

fn def(family: &str, key: &str) -> Value {
    json!({"family": family, "key": key, "revision": REV})
}

fn records() -> Value {
    json!([
        {"kind": "Ability", "identity": def("Ability", "oteryn:ability/bite"),
         "effects": [def("Effect", "oteryn:effect/bite")]},
        {"kind": "Generic", "identity": def("Behavior", "oteryn:behavior/passive-idle"),
         "client_projection": "ServerOnly"},
        {"kind": "Creature", "identity": def("Creature", "oteryn:creature/rat"),
         "client_projection": "ClientSafe",
         "presentation": def("Presentation", "oteryn:presentation/rat"),
         "behavior": def("Behavior", "oteryn:behavior/passive-idle"), "loot": null},
        {"kind": "Effect", "identity": def("Effect", "oteryn:effect/bite"),
         "client_projection": "ServerOnly", "effect_family": "Damage",
         "formula": def("Formula", "oteryn:formula/entry-melee-r1")},
        {"kind": "Formula", "identity": def("Formula", "oteryn:formula/entry-melee-r1")},
        {"kind": "Item", "identity": def("Item", "oteryn:item/cheese"),
         "client_projection": "ClientSafe", "materializable": true,
         "stack_class": "NonStackable"},
        {"kind": "Generic", "identity": def("Presentation", "oteryn:presentation/bite"),
         "client_projection": "ClientSafe"},
        {"kind": "Generic", "identity": def("Presentation", "oteryn:presentation/cheese"),
         "client_projection": "ClientSafe"},
        {"kind": "Generic", "identity": def("Presentation", "oteryn:presentation/rat"),
         "client_projection": "ClientSafe"},
        {"kind": "Generic", "identity": def("Terrain", "oteryn:terrain/stone-floor"),
         "client_projection": "ServerOnly"}
    ])
}

fn placement(key: &str, x: i32, y: i32) -> Value {
    json!({
        "key": key, "world": "oteryn:entry.world", "map_revision": "oteryn:map/entry-r1",
        "definition": def("Terrain", "oteryn:terrain/stone-floor"),
        "area": def("Area", "oteryn:area/entry-room"),
        "coordinate_frame": FRAME, "x": x, "y": y, "floor": 0,
        "presentation_order": {"plane": 0, "order": 0}, "disposition": "CandidateOnly"
    })
}

fn state() -> Value {
    json!({
        "declarations": [{"kind": "Area",
            "identity": {"key": "oteryn:area/entry-room", "revision": REV}, "fields": []}],
        "worlds": [{"key": "oteryn:entry.world", "world_id": WORLD_ID, "coordinate_frame": FRAME,
            "bounds": {"min_x": 0, "min_y": -1, "max_x_exclusive": 2, "max_y_exclusive": 1},
            "floors": [0]}],
        "placements": [
            placement("oteryn:cell/entry-start", 0, 0),
            placement("oteryn:cell/entry-east", 1, 0),
            placement("oteryn:cell/entry-north", 0, -1)
        ]
    })
}

fn overlay() -> Value {
    json!({
        "world_key": "oteryn:entry.world",
        "frame": {"coordinate_profile": "oteryn-world-spatial-v1", "contract_revision": 1,
            "coordinate_frame": FRAME, "origin": {"x": 0, "y": 0, "floor": 0},
            "x_direction": "East", "y_direction": "South", "higher_floor": "Up"},
        "revisions": {"content": "oteryn:content/entry-r1", "map": "oteryn:map/entry-r1",
            "ruleset": "oteryn:ruleset/entry-r1", "world_policy": "oteryn:world-policy/entry-r1",
            "compiler": "oteryn:compiler/first-production-r1",
            "canonicalization": "oteryn:canonicalization/first-production-r1",
            "sim_profile": "oteryn:sim/entry-r1",
            "profile_revision": "FIRST_PRODUCTION_CONTENT_PROFILE/v1"},
        "region": {"key": "oteryn:region/entry"},
        "cells": [
            {"placement_key": "oteryn:cell/entry-start", "region_key": "oteryn:region/entry",
             "collision": "Walkable"},
            {"placement_key": "oteryn:cell/entry-east", "region_key": "oteryn:region/entry",
             "collision": "Walkable"},
            {"placement_key": "oteryn:cell/entry-north", "region_key": "oteryn:region/entry",
             "collision": "Blocked"}
        ],
        "relocation": {"key": "oteryn:relocation/entry-east-return",
            "from_cell": "oteryn:cell/entry-east", "to_cell": "oteryn:cell/entry-start"},
        "behavior": {"definition": def("Behavior", "oteryn:behavior/passive-idle"),
            "policy_revision": "oteryn:policy/passive-idle-r1"},
        "presentations": [
            {"definition": def("Presentation", "oteryn:presentation/rat"),
             "metadata_token": "oteryn:appearance/rat-r1"},
            {"definition": def("Presentation", "oteryn:presentation/bite"),
             "metadata_token": "oteryn:appearance/bite-r1"},
            {"definition": def("Presentation", "oteryn:presentation/cheese"),
             "metadata_token": "oteryn:appearance/cheese-r1"}
        ],
        "creature": {"definition": def("Creature", "oteryn:creature/rat"),
            "policy_revision": "oteryn:policy/creature-rat-r1"},
        "spawn": {"key": "oteryn:spawn/entry-rat", "creature": def("Creature", "oteryn:creature/rat"),
            "behavior": def("Behavior", "oteryn:behavior/passive-idle"),
            "cell_key": "oteryn:cell/entry-east", "population_limit": 1,
            "recovery": "EphemeralScopeReset", "multiplicity": "ChannelLocalRepeatable",
            "eligibility_scope": "CharacterWorld"},
        "ability": {"definition": def("Ability", "oteryn:ability/bite"),
            "presentation": def("Presentation", "oteryn:presentation/bite")},
        "item": {"definition": def("Item", "oteryn:item/cheese"),
            "presentation": def("Presentation", "oteryn:presentation/cheese")},
        "loot_table": {"key": "oteryn:loot/rat", "entry": {"key": "oteryn:loot-entry/rat-cheese",
            "item": def("Item", "oteryn:item/cheese"), "rng_purpose_key": "oteryn:rng/rat-loot"}},
        "xp": {"key": "oteryn:xp/rat", "formula": def("Formula", "oteryn:formula/entry-melee-r1")},
        "rng": {"profile_revision": "oteryn:rng-profile/entry-r1",
            "purpose_key": "oteryn:rng/rat-loot"}
    })
}

struct Parts {
    licensing: String,
    records: Value,
    state: Value,
    overlay: Value,
}

fn valid() -> Parts {
    Parts {
        licensing: NATIVE_ENTRY_LICENSING.to_owned(),
        records: records(),
        state: state(),
        overlay: overlay(),
    }
}

fn draft(parts: &Parts) -> ProjectV2Draft {
    let state = &parts.state;
    ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "oteryn:package-rev/entry-r1".into(),
            package_key: "oteryn:package/native-entry-room".into(),
            semantic_schema_version: "oteryn:schema/first-production-v1".into(),
            licensing_metadata: parts.licensing.clone(),
            world_id: WORLD_ID.into(),
            coordinate_frame: FRAME.into(),
            records: serde_json::from_value(parts.records.clone()).expect("records"),
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: serde_json::from_value(state["declarations"].clone()).expect("decls"),
            item_authoring: vec![],
            authoring_profiles: vec![],
            worlds: serde_json::from_value(state["worlds"].clone()).expect("worlds"),
            placements: serde_json::from_value(state["placements"].clone()).expect("placements"),
            appearance_bindings: vec![],
            assets: vec![],
            sources: vec![],
            source_identity_bindings: vec![],
            editor: vec![],
        },
    }
}

fn build(parts: &Parts) -> Result<CanonicalProjectDocuments, ProjectError> {
    let overlay: NativeFirstEntryDocument = serde_json::from_value(parts.overlay.clone())
        .map_err(|error| ProjectError::InvalidJson(error.to_string()))?;
    CanonicalProjectDocuments::from_native_entry_draft(draft(parts), overlay, limits())
}

fn temp_parent() -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "oteryn-native-entry-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp parent");
    path
}

fn write_root(parent: &Path, documents: &CanonicalProjectDocuments) {
    let root = parent.join("project");
    for (locator, bytes) in documents.documents() {
        let path = root.join(locator);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, bytes).expect("write");
    }
}

fn refuses(mutate: impl FnOnce(&mut Parts)) {
    let mut parts = valid();
    mutate(&mut parts);
    assert!(build(&parts).is_err(), "mutation must refuse");
}

#[test]
fn native_entry_captures_qualifies_and_compiles_a_deterministic_pair() {
    let documents = build(&valid()).expect("valid native entry project");
    assert_eq!(documents.documents().len(), 11);
    let parent = temp_parent();
    write_root(&parent, &documents);

    let first = capture_native_entry_project(&parent, OsStr::new("project")).expect("capture");
    let second = capture_native_entry_project(&parent, OsStr::new("project")).expect("recapture");
    assert_eq!(first, second);
    let source = first.source();
    assert_eq!(source.cells.len(), 3);
    assert_eq!(
        source
            .cells
            .iter()
            .filter(|cell| cell.collision == CollisionClass::Blocked)
            .count(),
        1
    );
    assert_eq!(
        source.package_manifest.licensing_metadata.as_str(),
        NATIVE_ENTRY_LICENSING
    );

    let pair = compile_first_production(source, FirstProductionCompileTarget::OrdinaryRelease)
        .expect("ordinary release pair");
    let again = compile_first_production(
        second.source(),
        FirstProductionCompileTarget::OrdinaryRelease,
    )
    .expect("recompiled pair");
    assert_eq!(pair.server_digest(), again.server_digest());
    assert_eq!(pair.client_digest(), again.client_digest());
    fs::remove_dir_all(parent).expect("cleanup");
}

#[test]
fn ordinary_and_native_admission_never_accept_each_other() {
    let parent = temp_parent();
    write_root(&parent, &build(&valid()).expect("native"));
    assert!(
        capture_world_project(
            &parent,
            OsStr::new("project"),
            native_entry_first_slice_limits()
        )
        .is_err(),
        "ordinary capture must refuse the native variant"
    );
    fs::remove_dir_all(&parent).expect("cleanup");

    let ordinary = CanonicalProjectDocuments::from_v2_draft(draft(&valid()), limits())
        .expect("ordinary v2 project");
    let parent = temp_parent();
    write_root(&parent, &ordinary);
    assert!(
        capture_native_entry_project(&parent, OsStr::new("project")).is_err(),
        "native capture must refuse an ordinary v2 project"
    );
    fs::remove_dir_all(parent).expect("cleanup");
}

#[test]
fn first_slice_limits_are_the_accepted_values() {
    let limits = native_entry_first_slice_limits();
    let project = limits.project;
    assert_eq!(
        (
            project.max_documents,
            project.max_document_bytes,
            project.max_total_bytes,
            project.max_json_depth,
            project.max_decoded_fields,
            project.max_string_bytes,
            project.max_locator_bytes,
            project.max_locator_segments,
            project.max_reference_records,
            project.max_import_records,
            project.max_reimport_states,
        ),
        (11, 65_536, 262_144, 16, 4_096, 16_384, 128, 4, 32, 1, 1)
    );
    assert_eq!(limits.max_entries_per_directory_scan, 32);
    assert_eq!(limits.max_total_directory_entries_scanned, 192);
    // max / max+1 at the document-byte bound.
    let at_max = vec![b' '; project.max_document_bytes];
    assert!(ProjectSnapshot::new([("a.json".to_owned(), at_max)], project).is_ok());
    let over = vec![b' '; project.max_document_bytes + 1];
    assert!(ProjectSnapshot::new([("a.json".to_owned(), over)], project).is_err());
}

#[test]
fn every_single_invariant_mutation_refuses() {
    refuses(|p| p.licensing = "license:project-owned-v1".into());
    refuses(|p| p.overlay["world_key"] = json!("oteryn:other.world"));
    refuses(|p| p.overlay["frame"]["coordinate_profile"] = json!("other-profile"));
    refuses(|p| p.overlay["frame"]["contract_revision"] = json!(2));
    refuses(|p| p.overlay["frame"]["coordinate_frame"] = json!("oteryn:other.frame"));
    refuses(|p| p.overlay["frame"]["origin"]["x"] = json!(5));
    refuses(|p| p.overlay["frame"]["origin"]["floor"] = json!(1));
    refuses(|p| p.overlay["frame"]["x_direction"] = json!("West"));
    refuses(|p| p.overlay["revisions"]["map"] = json!("oteryn:map/other"));
    refuses(|p| p.overlay["revisions"]["profile_revision"] = json!("oteryn:profile/other"));
    refuses(|p| p.overlay["revisions"]["ruleset"] = Value::Null);
    refuses(|p| {
        p.overlay["cells"]
            .as_array_mut()
            .expect("cells")
            .truncate(2)
    });
    refuses(|p| p.overlay["cells"][1]["region_key"] = json!("oteryn:region/other"));
    refuses(|p| p.overlay["cells"][1]["placement_key"] = json!("oteryn:cell/entry-start"));
    refuses(|p| p.overlay["cells"][2]["collision"] = json!("Water"));
    refuses(|p| p.overlay["relocation"]["to_cell"] = json!("oteryn:cell/entry-east"));
    refuses(|p| p.overlay["relocation"]["to_cell"] = json!("oteryn:cell/missing"));
    refuses(|p| p.overlay["behavior"]["definition"]["revision"] = json!("oteryn:rev/other"));
    refuses(|p| {
        p.overlay["presentations"]
            .as_array_mut()
            .expect("p")
            .truncate(2)
    });
    refuses(|p| {
        p.overlay["presentations"][2]["definition"] =
            def("Presentation", "oteryn:presentation/rat");
    });
    refuses(|p| p.overlay["presentations"][0]["metadata_token"] = Value::Null);
    refuses(|p| p.overlay["creature"]["policy_revision"] = json!("fixture:policy"));
    refuses(|p| p.overlay["spawn"]["cell_key"] = json!("oteryn:cell/missing"));
    refuses(|p| p.overlay["spawn"]["behavior"] = def("Creature", "oteryn:creature/rat"));
    refuses(|p| p.overlay["spawn"]["recovery"] = Value::Null);
    refuses(|p| {
        p.overlay["ability"]["presentation"] = def("Presentation", "oteryn:presentation/x")
    });
    refuses(|p| p.overlay["item"]["presentation"] = def("Presentation", "oteryn:presentation/rat"));
    refuses(|p| p.overlay["loot_table"]["entry"]["rng_purpose_key"] = json!("oteryn:rng/other"));
    refuses(|p| p.overlay["loot_table"]["entry"]["item"] = def("Item", "oteryn:item/other"));
    refuses(|p| p.overlay["xp"]["formula"] = def("Formula", "oteryn:formula/other"));
    refuses(|p| p.overlay["rng"]["purpose_key"] = json!("oteryn:rng/other"));
    refuses(|p| p.overlay["unexpected"] = json!(true));
    refuses(|p| p.records[5]["materializable"] = json!(false));
    refuses(|p| p.records[3]["effect_family"] = json!("Heal"));
    refuses(|p| {
        p.records[2]["behavior"] = def("Behavior", "oteryn:behavior/other");
    });
    refuses(|p| {
        p.records.as_array_mut().expect("records").push(json!({"kind": "Generic",
            "identity": def("Terrain", "oteryn:terrain/extra"), "client_projection": "ServerOnly"}));
    });
    refuses(|p| p.state["placements"][1]["map_revision"] = json!("oteryn:map/other"));
    refuses(|p| p.state["placements"][1]["x"] = json!(0));
    refuses(|p| p.state["placements"][1]["area"] = Value::Null);
    refuses(|p| {
        p.state["worlds"][0]["bounds"]["max_x_exclusive"] = json!(1);
    });
}
