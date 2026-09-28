//! `NATIVE_ENTRY_SOURCE_CONSUMER_V1`: explicit native admission, typed overlay lowering and the
//! deterministic FirstProduction pair (#937 §5/§7, #940). Every negative case changes one
//! invariant of an otherwise valid project.
#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const WORLD_ID: &str = "0123456789ab70cd8ef0123456789abc";
const FRAME: &str = "oteryn:entry.frame";
const REV: &str = "oteryn:rev/entry-r1";
// Records are written family-then-key sorted: Ability, Behavior, Creature, Effect, Formula, Item,
// LocalObject (index 6), Presentation x3, Terrain.
const DOOR_RECORD_INDEX: usize = 6;
// Placements are written key-sorted, so "oteryn:cell/entry-door" (index 0) sorts before
// east/north/start.
const DOOR_PLACEMENT_INDEX: usize = 0;

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
         "client_projection": "ServerOnly"},
        {"kind": "LocalObject", "identity": def("LocalObject", "oteryn:local-object/entry-door"),
         "client_projection": "ClientSafe",
         "states": [
            {"key": "oteryn:reference.state.closed", "collision": "Present"},
            {"key": "oteryn:reference.state.open", "collision": "Absent"}
         ]}
    ])
}

fn door_cell() -> Value {
    json!({
        "key": "oteryn:cell/entry-door", "world": "oteryn:entry.world",
        "map_revision": "oteryn:map/entry-r1",
        "definition": def("Terrain", "oteryn:terrain/stone-floor"),
        "area": def("Area", "oteryn:area/entry-room"),
        "coordinate_frame": FRAME, "x": 1, "y": -1, "floor": 0,
        "presentation_order": {"plane": 0, "order": 0}, "disposition": "CandidateOnly"
    })
}

fn door_overlay() -> Value {
    json!({
        "cell": {"placement_key": "oteryn:cell/entry-door", "region_key": "oteryn:region/entry",
            "collision": "Walkable"},
        "definition": def("LocalObject", "oteryn:local-object/entry-door"),
        "closed_state": "oteryn:reference.state.closed",
        "open_state": "oteryn:reference.state.open",
        "open_transition": {"key": "oteryn:transition/entry-door-open"},
        "close_transition": {"key": "oteryn:transition/entry-door-close"}
    })
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
            placement("oteryn:cell/entry-north", 0, -1),
            door_cell()
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
        "doors": [door_overlay()],
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
    CanonicalProjectDocuments::from_native_entry_draft(draft(parts), overlay)
}

type Docs = BTreeMap<String, Vec<u8>>;

fn valid_docs() -> Docs {
    build(&valid())
        .expect("valid native entry project")
        .documents()
        .clone()
}

fn value(docs: &Docs, locator: &str) -> Value {
    serde_json::from_slice(&docs[locator]).expect("json document")
}

fn put(docs: &mut Docs, locator: &str, value: &Value) {
    docs.insert(
        locator.to_owned(),
        serde_json::to_vec(value).expect("encode"),
    );
}

/// Recompute the manifest inventory, package provenance, Content Lock and root digests so a
/// mutated document is admitted up to the invariant under test.
fn reseal(docs: &mut Docs) {
    let mut manifest = value(docs, "manifest.json");
    for entry in manifest["documents"].as_array_mut().expect("inventory") {
        let bytes = &docs[entry["locator"].as_str().expect("locator")];
        entry["byte_length"] = json!(bytes.len());
        entry["sha256"] = json!(world_project_sha256(bytes));
    }
    put(docs, "manifest.json", &manifest);
    let text = |field: &str| manifest[field].as_str().expect("manifest field").to_owned();
    let provenance = PackageManifestBinding::new(
        ProductionKey::new(&text("package_key")).expect("key"),
        ProductionAtom::new("revision", &text("package_revision")).expect("revision"),
        ProductionAtom::new("schema", &text("semantic_schema_version")).expect("schema"),
        ProductionAtom::new("licensing", &text("licensing_metadata")).expect("licensing"),
        Sha256HexDigest::new(&world_project_sha256(&docs["manifest.json"])).expect("digest"),
    )
    .package_provenance_digest()
    .expect("provenance");
    let mut lock = value(docs, "content.lock.json");
    lock["entries"][0]["package_provenance_digest"] = json!(provenance.as_str());
    put(docs, "content.lock.json", &lock);
    let mut root = value(docs, "project.json");
    root["manifest_sha256"] = json!(world_project_sha256(&docs["manifest.json"]));
    root["content_lock_sha256"] = json!(world_project_sha256(&docs["content.lock.json"]));
    put(docs, "project.json", &root);
}

fn edit(locator: &str, mutate: impl FnOnce(&mut Value)) -> Docs {
    let mut docs = valid_docs();
    let mut document = value(&docs, locator);
    mutate(&mut document);
    put(&mut docs, locator, &document);
    reseal(&mut docs);
    docs
}

fn overlay_edit(mutate: impl FnOnce(&mut Value)) -> Docs {
    edit("definitions/declarations.json", |document| {
        mutate(&mut document["native_first_entry"]);
    })
}

fn admit(docs: &Docs) -> Result<NativeEntryProject, ProjectError> {
    ProjectSnapshot::new(docs.clone(), limits())?.parse_native_entry()
}

#[track_caller]
fn refuses(docs: &Docs, reason: &str) {
    let error = admit(docs).expect_err("mutation must refuse");
    assert!(
        format!("{error:?}").contains(reason),
        "expected refusal containing {reason:?}, got {error:?}"
    );
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

fn write_root(parent: &Path, docs: &Docs) {
    let root = parent.join("project");
    for (locator, bytes) in docs {
        let path = root.join(locator);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, bytes).expect("write");
    }
}

fn capture(docs: &Docs) -> Result<NativeEntryProject, ProjectFilesystemError> {
    let parent = temp_parent();
    write_root(&parent, docs);
    let result = capture_native_entry_project(&parent, OsStr::new("project"));
    fs::remove_dir_all(parent).expect("cleanup");
    result
}

#[test]
fn native_entry_captures_qualifies_and_compiles_a_deterministic_pair() {
    let docs = valid_docs();
    assert_eq!(docs.len(), 11);
    let parent = temp_parent();
    write_root(&parent, &docs);
    let first = capture_native_entry_project(&parent, OsStr::new("project")).expect("capture");
    let second = capture_native_entry_project(&parent, OsStr::new("project")).expect("recapture");
    fs::remove_dir_all(parent).expect("cleanup");
    assert_eq!(first, second);
    let source = first.source();
    // #162 A4-a: the three room Terrain cells plus the door's own walkable Terrain cell.
    assert_eq!(source.cells.len(), 4);
    let door = first.door();
    // DECISION_REQUIRED (r4120444680): no placement is genuinely linker-validated today (the
    // accepted evidence manifest has no CONTENT_WORLD case bound to any target-sensitive claim),
    // so `door()` stays genuinely, fully linked with no fabricated placement.
    assert!(door.placements.is_empty());
    assert_eq!(door.transitions.len(), 2);
    assert_eq!(door.definitions.len(), 1);
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
    // The reseal helper is neutral: an unmodified project still admits.
    let mut resealed = valid_docs();
    reseal(&mut resealed);
    assert!(admit(&resealed).is_ok());
}

#[test]
fn ordinary_and_native_admission_never_accept_each_other() {
    let parent = temp_parent();
    write_root(&parent, &valid_docs());
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
        .expect("ordinary v2 project")
        .documents()
        .clone();
    assert!(
        capture(&ordinary).is_err(),
        "native capture must refuse ordinary v2"
    );
    refuses(
        &ordinary,
        "native entry admission requires the native source profile",
    );

    // Native root profile with the ordinary declarations schema (document and manifest).
    let mut ordinary_schema = edit("definitions/declarations.json", |document| {
        document["schema"] = json!("OTERYN_WORLD_PROJECT_DECLARATIONS/v2");
    });
    refuses(&ordinary_schema, "schema mismatch");
    let mut manifest = value(&ordinary_schema, "manifest.json");
    for entry in manifest["documents"].as_array_mut().expect("inventory") {
        if entry["role"] == "declarative-definitions" {
            entry["schema"] = json!("OTERYN_WORLD_PROJECT_DECLARATIONS/v2");
        }
    }
    put(&mut ordinary_schema, "manifest.json", &manifest);
    reseal(&mut ordinary_schema);
    refuses(&ordinary_schema, "role");

    // Missing overlay, extra feature declaration.
    refuses(
        &edit("definitions/declarations.json", |document| {
            document
                .as_object_mut()
                .expect("object")
                .remove("native_first_entry");
        }),
        "native_first_entry",
    );
    refuses(
        &edit("manifest.json", |manifest| {
            manifest["required_features"] = json!(["oteryn:feature/native"]);
        }),
        "unsupported project feature declaration",
    );
    // Ordinary parsing of the native bytes refuses.
    assert!(
        ProjectSnapshot::new(valid_docs(), limits())
            .expect("snapshot")
            .parse(limits())
            .is_err()
    );
}

#[test]
fn accepted_product_bindings_are_enforced() {
    let pin = "not the accepted";
    refuses(
        &overlay_edit(|o| o["creature"]["policy_revision"] = json!("oteryn:policy/unaccepted-r9")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["behavior"]["policy_revision"] = json!("oteryn:policy/other-r1")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["recovery"] = json!("DurableEventOccurrence")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["multiplicity"] = json!("WorldScopedUnique")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["eligibility_scope"] = json!("AccountWorld")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["cell_key"] = json!("oteryn:cell/entry-start")),
        pin,
    );
    refuses(
        &overlay_edit(|o| {
            o["cells"][0]["collision"] = json!("Blocked");
            o["cells"][2]["collision"] = json!("Walkable");
        }),
        pin,
    );
    refuses(&overlay_edit(|o| o["frame"]["origin"]["x"] = json!(1)), pin);
    refuses(
        &overlay_edit(|o| {
            o["region"]["key"] = json!("oteryn:region/other");
            o["doors"][0]["cell"]["region_key"] = json!("oteryn:region/other");
            for cell in o["cells"].as_array_mut().expect("cells") {
                cell["region_key"] = json!("oteryn:region/other");
            }
        }),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["relocation"]["to_cell"] = json!("oteryn:cell/entry-north")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["rng"]["profile_revision"] = json!("oteryn:rng-profile/other")),
        pin,
    );
    refuses(
        &overlay_edit(|o| {
            o["presentations"][0]["metadata_token"] = json!("oteryn:appearance/other")
        }),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["revisions"]["ruleset"] = json!("oteryn:ruleset/other")),
        pin,
    );
    refuses(
        &overlay_edit(|o| o["loot_table"]["key"] = json!("oteryn:loot/other")),
        pin,
    );
    refuses(
        &edit("worlds/world.json", |w| {
            w["worlds"][0]["bounds"]["max_x_exclusive"] = json!(100)
        }),
        pin,
    );
    refuses(
        &edit("worlds/world.json", |w| {
            w["worlds"][0]["floors"] = json!([0, 1])
        }),
        pin,
    );
    refuses(
        &edit("manifest.json", |m| {
            m["package_key"] = json!("oteryn:package/other")
        }),
        "Content Lock",
    );
}

#[test]
fn every_single_invariant_mutation_refuses_for_its_reason() {
    refuses(
        &edit("manifest.json", |m| {
            m["licensing_metadata"] = json!("license:project-owned-v1")
        }),
        "licensing metadata mismatch",
    );
    refuses(
        &overlay_edit(|o| o["world_key"] = json!("oteryn:other.world")),
        "World or frame binding",
    );
    refuses(
        &overlay_edit(|o| o["frame"]["coordinate_profile"] = json!("other")),
        "coordinate contract",
    );
    refuses(
        &overlay_edit(|o| o["frame"]["contract_revision"] = json!(2)),
        "coordinate contract",
    );
    refuses(
        &overlay_edit(|o| o["frame"]["coordinate_frame"] = json!("oteryn:x.frame")),
        "World or frame binding",
    );
    refuses(
        &overlay_edit(|o| o["frame"]["origin"]["floor"] = json!(1)),
        "origin",
    );
    refuses(
        &overlay_edit(|o| o["frame"]["x_direction"] = json!("West")),
        "unknown variant `West`",
    );
    refuses(
        &overlay_edit(|o| o["revisions"]["map"] = json!("oteryn:map/other")),
        "placement binding",
    );
    refuses(
        &overlay_edit(|o| o["revisions"]["profile_revision"] = json!("oteryn:profile/other")),
        "FirstProduction profile",
    );
    refuses(
        &overlay_edit(|o| o["revisions"]["ruleset"] = Value::Null),
        "invalid type: null",
    );
    refuses(
        &overlay_edit(|o| o["cells"].as_array_mut().expect("c").truncate(2)),
        "exactly three cells",
    );
    refuses(
        &overlay_edit(|o| o["cells"][1]["region_key"] = json!("oteryn:region/other")),
        "cell region",
    );
    refuses(
        &overlay_edit(|o| o["cells"][1]["placement_key"] = json!("oteryn:cell/entry-start")),
        "duplicated",
    );
    refuses(
        &overlay_edit(|o| o["cells"][2]["collision"] = json!("Water")),
        "unknown variant `Water`",
    );
    refuses(
        &overlay_edit(|o| o["relocation"]["to_cell"] = json!("oteryn:cell/entry-east")),
        "relocation endpoints",
    );
    refuses(
        &overlay_edit(|o| o["relocation"]["to_cell"] = json!("oteryn:cell/missing")),
        "relocation endpoints",
    );
    refuses(
        &overlay_edit(|o| o["behavior"]["definition"]["revision"] = json!("oteryn:rev/other")),
        "does not resolve exactly",
    );
    // Wrong family, same key.
    refuses(
        &overlay_edit(|o| o["behavior"]["definition"]["family"] = json!("Terrain")),
        "wrong family",
    );
    refuses(
        &overlay_edit(|o| o["presentations"].as_array_mut().expect("p").truncate(2)),
        "exactly three presentations",
    );
    refuses(
        &overlay_edit(|o| {
            o["presentations"][2]["definition"] = def("Presentation", "oteryn:presentation/rat");
            o["item"]["presentation"] = def("Presentation", "oteryn:presentation/rat");
        }),
        "distinct per Creature, Ability and Item",
    );
    refuses(
        &overlay_edit(|o| o["presentations"][0]["metadata_token"] = Value::Null),
        "invalid type: null",
    );
    refuses(
        &overlay_edit(|o| o["creature"]["policy_revision"] = json!("fixture:policy")),
        "InvalidString",
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["cell_key"] = json!("oteryn:cell/missing")),
        "spawn binding",
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["behavior"] = def("Creature", "oteryn:creature/rat")),
        "spawn binding",
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["population_limit"] = json!(0)),
        "spawn is not the accepted",
    );
    refuses(
        &overlay_edit(|o| o["spawn"]["recovery"] = Value::Null),
        "invalid type: null",
    );
    refuses(
        &overlay_edit(|o| {
            o["ability"]["presentation"] = def("Presentation", "oteryn:presentation/x")
        }),
        "Ability presentation",
    );
    refuses(
        &overlay_edit(|o| o["item"]["presentation"] = def("Presentation", "oteryn:presentation/x")),
        "Item presentation",
    );
    refuses(
        &overlay_edit(|o| o["loot_table"]["entry"]["rng_purpose_key"] = json!("oteryn:rng/other")),
        "loot entry binding",
    );
    refuses(
        &overlay_edit(|o| o["loot_table"]["entry"]["item"] = def("Item", "oteryn:item/other")),
        "loot entry binding",
    );
    refuses(
        &overlay_edit(|o| o["xp"]["formula"] = def("Formula", "oteryn:formula/other")),
        "single Effect formula",
    );
    refuses(
        &overlay_edit(|o| o["rng"]["purpose_key"] = json!("oteryn:rng/other")),
        "loot entry binding",
    );
    refuses(
        &overlay_edit(|o| o["unexpected"] = json!(true)),
        "unknown field `unexpected`",
    );
    refuses(
        &edit("definitions/reference.json", |r| {
            r["records"][5]["materializable"] = json!(false)
        }),
        "materializable",
    );
    refuses(
        &edit("definitions/reference.json", |r| {
            r["records"][3]["effect_family"] = json!("Heal")
        }),
        "Damage Effect",
    );
    refuses(
        &edit("definitions/reference.json", |r| {
            r["records"][2]["behavior"] = def("Behavior", "oteryn:behavior/other");
        }),
        "Creature behavior mismatch",
    );
    refuses(
        &edit("definitions/reference.json", |r| {
            r["records"]
                .as_array_mut()
                .expect("records")
                .push(json!({"kind": "Generic",
                "identity": def("Terrain", "oteryn:terrain/zzz-extra"),
                "client_projection": "ServerOnly"}));
        }),
        "outside the selected graph",
    );
    refuses(
        &edit("worlds/world.json", |w| {
            w["placements"][1]["map_revision"] = json!("oteryn:map/other")
        }),
        "placement binding",
    );
    refuses(
        &edit("worlds/world.json", |w| {
            w["placements"][1]["area"] = Value::Null
        }),
        "placement binding",
    );
}

/// #162 comment 5865792400 (owner decision A4-a): the entry room's one usable door. Every
/// negative case changes one invariant of the otherwise-valid door while the three room cells and
/// everything else stay valid.
#[test]
fn door_admission_refuses_every_invariant_mutation() {
    // No door.
    refuses(
        &overlay_edit(|o| o["doors"] = json!([])),
        "exactly one door",
    );
    // Two doors.
    refuses(
        &overlay_edit(|o| {
            let extra = o["doors"][0].clone();
            o["doors"].as_array_mut().expect("doors").push(extra);
        }),
        "exactly one door",
    );
    // Not adjacent / off the room frame: the accepted World envelope exactly fits the four placed
    // cells (`accepted::BOUNDS`, not grown to make refusal tests distinct — #162 A4-a cleanup), so
    // every other in-bounds coordinate is already occupied by a room cell. A door moved off that
    // frame is refused by the generic v2 placement/World bounds check before native-entry's own
    // door-adjacency check would even run; that adjacency check itself remains structurally
    // exercised by every passing admission (the accepted door cell must satisfy it).
    refuses(
        &edit("worlds/world.json", |w| {
            w["placements"][DOOR_PLACEMENT_INDEX]["x"] = json!(5);
        }),
        "world/frame/position mismatch",
    );
    // Unknown state: the overlay names a state the LocalObject record never declares.
    refuses(
        &overlay_edit(|o| {
            o["doors"][0]["closed_state"] = json!("oteryn:reference.state.other");
        }),
        "unknown state",
    );
    // Unknown state: the record's own vocabulary no longer matches (both states end up Absent).
    refuses(
        &edit("definitions/reference.json", |r| {
            r["records"][DOOR_RECORD_INDEX]["states"][0]["collision"] = json!("Absent");
        }),
        "unknown state",
    );
    // Bad transition: the door's own two transitions must keep their exactly-accepted, distinct
    // keys — collapsing them onto one key is refused as an unaccepted door transition binding.
    refuses(
        &overlay_edit(|o| {
            let close_key = o["doors"][0]["close_transition"]["key"].clone();
            o["doors"][0]["open_transition"]["key"] = close_key;
        }),
        "not the accepted binding",
    );
}

/// r4120444694: the resolved door definition key must match `accepted::DOOR_DEFINITION`, the same
/// way every other product binding is checked — not merely resolve to exactly one LocalObject
/// record. Changing both the record's own identity and `doors[0].definition.key` to the same new
/// key still refuses, because that key is not the accepted one.
#[test]
fn door_definition_key_must_match_the_accepted_binding() {
    let mut docs = valid_docs();
    let mut records = value(&docs, "definitions/reference.json");
    records["records"][DOOR_RECORD_INDEX]["identity"]["key"] =
        json!("oteryn:local-object/other-door");
    put(&mut docs, "definitions/reference.json", &records);
    let mut declarations = value(&docs, "definitions/declarations.json");
    declarations["native_first_entry"]["doors"][0]["definition"]["key"] =
        json!("oteryn:local-object/other-door");
    put(&mut docs, "definitions/declarations.json", &declarations);
    reseal(&mut docs);
    refuses(&docs, "not the accepted binding");
}

/// r4120672740: a malformed project can declare World bounds spanning the full accepted i32
/// coordinate range, reaching the door-adjacency distance calculation before the later
/// accepted-coordinate pin. With a room cell near `i32::MIN` and the door near `i32::MAX`, i32
/// subtraction/addition here would overflow (a debug-build panic); it must instead refuse with a
/// `ProjectError`.
#[test]
fn door_adjacency_near_i32_extremes_refuses_without_panicking() {
    let docs = edit("worlds/world.json", |w| {
        w["worlds"][0]["bounds"]["min_x"] = json!(i64::from(i32::MIN));
        w["worlds"][0]["bounds"]["max_x_exclusive"] = json!(i64::from(i32::MAX) + 1);
        // Placements are written key-sorted: door(0), east(1), north(2), start(3).
        w["placements"][0]["x"] = json!(i32::MAX);
        w["placements"][2]["x"] = json!(i32::MIN);
    });
    refuses(&docs, "must be adjacent");
}

/// r4120444668: zero placements must refuse with a `ProjectError`, not panic by indexing
/// `state.placements[0]` before cardinality is checked.
#[test]
fn zero_placements_refuses_without_panicking() {
    refuses(
        &edit("worlds/world.json", |w| {
            w["placements"] = json!([]);
        }),
        "requires exactly three cells and placements",
    );
}

/// DECISION_REQUIRED (r4120444680): reconstructs the door's own genuinely linked content plus a
/// placement built exactly like the one M2a's earlier revision fabricated, and re-runs that
/// source through the real `link_reference_playable`. It refuses — proving no placement, honest
/// or malformed, can pass `validate_placement`'s reference-promotion check today, because the
/// accepted evidence manifest has no `CONTENT_WORLD` case bound to any target-sensitive claim
/// (`REFERENCE_TARGET_CLAIM_CASE_BINDINGS` in `content/reference_playable.rs`). This is why
/// `NativeEntryProject::door()` carries no placement at all rather than an unlinked, fabricated
/// one labeled canonical.
#[test]
fn door_placement_is_refused_by_the_real_linker_path() {
    let door = admit(&valid_docs())
        .expect("valid native entry project")
        .door()
        .clone();
    let evidence = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r0").expect("atom"),
        ProductionKey::new("oteryn:cw4.native-entry-door-placement").expect("key"),
        EvidenceDisposition::Unknown,
    );
    let footprint = FootprintRelation::Qualified {
        members: vec![FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        }],
        evidence: evidence.clone(),
    };
    let placement = PlacementRef {
        key: PlacementKey::new("oteryn:cell/entry-door").expect("placement key"),
        map_revision: MapRevisionRef::new("oteryn:map/entry-r1").expect("map revision"),
        definition: door.definitions[0].definition.clone(),
        address: SpatialAddress {
            world_id: door.world_id,
            coordinate_frame: door.coordinate_frame.clone(),
            cell: LogicalCell { x: 1, y: -1, z: 0 },
            evidence: evidence.clone(),
        },
        presentation_footprint: footprint.clone(),
        collision_footprint: footprint,
        local_object_initial_state: Some(
            ProductionKey::new("oteryn:reference.state.closed").expect("state key"),
        ),
    };
    let source = ReferencePlayableContentSource {
        profile_revision: door.profile_revision,
        capability_profile: door.capability_profile,
        package_manifest: door.package_manifest,
        content_lock: door.content_lock,
        world_id: door.world_id,
        coordinate_frame: door.coordinate_frame,
        definitions: door.definitions,
        placements: vec![placement],
        ordered_placements: vec![],
        transitions: door.transitions,
    };
    let error =
        link_reference_playable(source).expect_err("no placement can be linker-validated today");
    let message = format!("{error:?}");
    assert!(
        message.contains("promotable")
            || message.contains("not bound to target-sensitive")
            || message.contains("evidence manifest revision")
            || message.contains("evidence case"),
        "{message}"
    );
}

#[test]
fn first_slice_limits_are_fixed_and_enforced_at_the_producer_boundary() {
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

    // Document bytes: pad editor/author.json with JSON whitespace to exactly max and max+1.
    let padded = |target: usize| {
        let mut docs = valid_docs();
        let mut bytes = docs["editor/author.json"].clone();
        bytes.resize(target, b' ');
        docs.insert("editor/author.json".to_owned(), bytes);
        reseal(&mut docs);
        docs
    };
    capture(&padded(project.max_document_bytes)).expect("document at max bytes");
    let over = capture(&padded(project.max_document_bytes + 1)).expect_err("max+1 bytes");
    assert!(format!("{over:?}").contains("LimitExceeded"), "{over:?}");

    // Parent-directory scan: the dedicated parent may hold at most 32 entries.
    let with_siblings = |siblings: usize| {
        let parent = temp_parent();
        write_root(&parent, &valid_docs());
        for index in 0..siblings {
            fs::create_dir(parent.join(format!("sibling-{index}"))).expect("sibling");
        }
        let result = capture_native_entry_project(&parent, OsStr::new("project"));
        fs::remove_dir_all(parent).expect("cleanup");
        result
    };
    with_siblings(limits.max_entries_per_directory_scan - 1).expect("parent at max entries");
    let over = with_siblings(limits.max_entries_per_directory_scan).expect_err("max+1 entries");
    assert!(format!("{over:?}").contains("LimitExceeded"), "{over:?}");
}
