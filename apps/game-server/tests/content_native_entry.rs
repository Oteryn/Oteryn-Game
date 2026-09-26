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
    assert_eq!(source.cells.len(), 3);
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
