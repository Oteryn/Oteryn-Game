#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const DOCUMENTS: [(&str, usize, &str); 11] = [
    (
        "assets/catalog.json",
        56,
        "fa61df1076ef66e8c69b6182db1b13fe2ccb92653ab16a148d1fbaf773adb344",
    ),
    (
        "content.lock.json",
        364,
        "fac024fdcdb5a9997b30f4aa7864b3d3ca803fb852d5a35b303d3982ace6a746",
    ),
    (
        "definitions/declarations.json",
        15_148_142,
        "01c03a2c2c73ad28d82756a5aebac0ad75bfbf1bd263f1488accf21319917722",
    ),
    (
        "definitions/reference.json",
        24_890_347,
        "9fc8082a0e7bf2bdec963cc1127cfd91e12339f75dce822130504295729f67c4",
    ),
    (
        "editor/author.json",
        120_557,
        "14c6e3163baf17096545daa897a866df0b5ee23721c26312867ffa73447bbfdb",
    ),
    (
        "manifest.json",
        1_937,
        "146b3b29b93488f67748fad29e9920865b677f1fae0f97ceb8c606c3e4f9a836",
    ),
    (
        "presentations/bindings.json",
        73,
        "b7d27b2d7fde6370b7ae56c2c60412f2b213a4a5e92e7db82dd4b59a64359cd7",
    ),
    (
        "project.json",
        390,
        "4b7494c986a82208e78010fd6b33e6aaa6d7b1b8e49977756f7bb96390adbe9b",
    ),
    (
        "provenance/imports.json",
        33_077,
        "a6865cf27269c6d3cabb80b9b7895aab202d11d829675e7fda17b1c2cda89763",
    ),
    (
        "provenance/sources.json",
        1_317_786,
        "8319dc455ec284809447e1d50b0dba984ff451c45ad305fa117269def60f1bb1",
    ),
    (
        "worlds/world.json",
        72,
        "0dcc223c4a904834a58b3ad2b1c7882636cc66c9123aafdb25bb69a1d4670dfa",
    ),
];
/// Successor authoring-tree directory markers that coexist under the legacy package root.
///
/// They are not WorldProject manifest locators: capture opens only control and manifest-listed
/// documents, so these files never enter the legacy package, its digests or runtime content.
const SUCCESSOR_TREE_MARKERS: [&str; 10] = [
    "areas/cities/index.json",
    "areas/hunting-places/index.json",
    "areas/islands/index.json",
    "areas/regions/index.json",
    "areas/streets/index.json",
    "objects/index.json",
    "placements/index.json",
    "terrain/index.json",
    "transitions/index.json",
    "worlds/index.json",
];
/// The one successor shard that sits beside a legacy locator: the `World` family record in
/// `worlds/`. Legacy lookups of `worlds/world.json` scan that directory, so this exception is
/// bounded to exactly one shard and is counted in the scan budget of `filesystem_limits`.
const SHARED_DIRECTORY_SUCCESSOR_SHARDS: [&str; 1] = ["worlds/worlds-00000-00000.json"];
/// Successor family shards (written by `tools/content-schema/world-authoring`) are likewise
/// never WorldProject locators. They sit only in a successor directory that holds no legacy
/// locator, so legacy locator lookups scan no additional directory entries, except for the
/// single explicit `SHARED_DIRECTORY_SUCCESSOR_SHARDS` entry.
fn is_successor_shard(locator: &str) -> bool {
    SHARED_DIRECTORY_SUCCESSOR_SHARDS.contains(&locator) || is_unshared_successor_shard(locator)
}
fn is_unshared_successor_shard(locator: &str) -> bool {
    SUCCESSOR_TREE_MARKERS.iter().any(|marker| {
        let directory = marker.trim_end_matches("index.json");
        locator.strip_prefix(directory).is_some_and(|name| {
            !name.contains('/') && (name.ends_with(".json") || name.ends_with(".b3"))
        }) && !DOCUMENTS
            .iter()
            .any(|(document, _, _)| document.starts_with(directory))
    })
}
/// WO-2 and Area catalogue shards beside the legacy package: `(directory, shard prefix)`.
/// Their bytes are pinned by `build_catalogue.py --check` and `build_areas.py build --check`,
/// not by this package inventory.
const WORLD_CATALOGUE_SHARDS: [(&str, &str); 4] = [
    ("terrain/", "terrain-"),
    ("objects/", "objects-"),
    ("areas/cities/", "areas-"),
    ("areas/regions/", "areas-"),
];
const TREE_CONTRACT: &str =
    "docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json";
const TREE_DIRECTORY_NODES: usize = 97;
const TREE_SHA256: &str = "2ead8ad32038fb4bb7534700590658d428e1bb57d8ef25a55aabdd70fe01240d";
/// A12 (ITEM-ID-1b): the protected Item family less the 4,590 D149 records, on Tibia keys,
/// plus the 404 donor epoch-2 records and the 60 appearance-only records (ITEM-ADD-1).
const ITEMS: usize = 34_031;
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_120_000;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 43_000_000;
/// Canary creature admission pilot (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §7 slice 3).
const CREATURES: usize = 1503;
const CREATURE_RECORDS: usize = 21069;
const CREATURE_PROFILES: usize = 20097;
/// NPC admission wave A (OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1 §7 slice 4).
const NPCS: usize = 1110;
const NPC_RECORDS: usize = 2220;
const NPC_DECLARATIONS: usize = 2184;
const NPC_DIALOGUES: usize = 694;
const NPC_BINDINGS: usize = 2376;
/// Encounter admission (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 §5 slice 4).
const ENCOUNTERS: usize = 61;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn project_root() -> PathBuf {
    repository_root().join("content/world")
}

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: DOCUMENTS.len(),
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: FULL_FAMILY_MAX_DECODED_FIELDS,
        max_string_bytes: FULL_FAMILY_MAX_STRING_BYTES,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT + CREATURE_RECORDS + NPC_RECORDS,
        max_import_records: 12,
        max_reimport_states: ENCOUNTERS,
    }
}

fn filesystem_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: limits(),
        max_entries_per_directory_scan: 32,
        // The successor authoring tree adds 16 legitimate siblings under content/
        // (22 entries versus the 6-entry legacy baseline). Root lookup scans that
        // ambient parent, so retain the original 128-entry evidence budget plus
        // exactly that bounded sibling delta without relaxing the per-scan limit.
        // The 10 successor world markers add 5 siblings to the package root (seen by
        // each of the 11 locator lookups) plus 1 entry in worlds/: exactly 56 more. The
        // World family adds one shard beside worlds/world.json, which the single
        // `worlds/world.json` lookup scans once: exactly 1 more (144 + 56 + 1).
        max_total_directory_entries_scanned: 144 + 56 + 1,
    }
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("read canonical package directory") {
        let entry = entry.expect("canonical package directory entry");
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).expect("canonical package metadata");
        assert!(!metadata.file_type().is_symlink(), "{}", path.display());
        if metadata.is_dir() {
            collect_files(root, &path, files);
        } else {
            assert!(metadata.is_file(), "{}", path.display());
            files.push(
                path.strip_prefix(root)
                    .expect("file remains below package root")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn tree_digest(root: &Path) -> String {
    let mut tree = Sha256::new();
    for (locator, _, _) in DOCUMENTS {
        let bytes = fs::read(root.join(locator)).expect("read canonical package document");
        tree.update((locator.len() as u64).to_be_bytes());
        tree.update(locator.as_bytes());
        tree.update((bytes.len() as u64).to_be_bytes());
        tree.update(bytes);
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in tree.finalize() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn promoted_atom_count(semantics: &ReferenceItemSemantics) -> usize {
    use ReferenceItemField::Known;

    let mut count = 0_usize;
    if let Known(value) = &semantics.presentation {
        count += usize::from(matches!(&value.name, Known(_)));
    }
    if let Known(value) = &semantics.weapon {
        count += usize::from(matches!(&value.attack, Known(_)));
        count += usize::from(matches!(&value.defense, Known(_)));
        count += usize::from(matches!(&value.extra_defense, Known(_)));
        count += usize::from(matches!(&value.range, Known(_)));
        count += usize::from(matches!(&value.hit_chance, Known(_)));
    }
    if let Known(value) = &semantics.protection {
        count += usize::from(matches!(&value.armor, Known(_)));
    }
    if let Known(value) = &semantics.charges {
        count += usize::from(matches!(&value.count, Known(_)));
    }
    if let Known(value) = &semantics.container {
        count += usize::from(matches!(&value.capacity, Known(_)));
    }
    count
}

/// Typed Item facts promoted by Item enrichment Wave 1.
fn wave1_atom_count(semantics: &ReferenceItemSemantics) -> usize {
    use ReferenceItemField::Known;

    let mut count = 0_usize;
    if let Known(value) = &semantics.weapon {
        count += usize::from(matches!(&value.weapon_type, Known(_)));
    }
    if let Known(value) = &semantics.imbuement {
        count += usize::from(matches!(&value.slot_count, Known(_)));
    }
    if let Known(value) = &semantics.stack {
        count += usize::from(matches!(&value.stackable, Known(false)));
        if matches!(value.stackable, Known(true)) {
            assert_eq!(value.stack_max, Known(100));
        } else {
            assert!(matches!(&value.stack_max, ReferenceItemField::Unknown));
        }
    }
    if let Known(value) = &semantics.trade_restrictions {
        count += usize::from(matches!(&value.marketable, Known(_)));
    }
    count
}

#[test]
fn tracked_package_has_exact_inventory_digests_and_no_runtime_identity_layer() {
    let root = project_root();
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    let files_seen = files.clone();
    let (successors, mut actual): (Vec<_>, Vec<_>) = files.into_iter().partition(|locator| {
        SUCCESSOR_TREE_MARKERS.contains(&locator.as_str()) || is_successor_shard(locator)
    });
    let mut markers: Vec<_> = successors
        .into_iter()
        .filter(|locator| SUCCESSOR_TREE_MARKERS.contains(&locator.as_str()))
        .collect();
    for (directory, prefix) in WORLD_CATALOGUE_SHARDS {
        assert!(
            files_seen
                .iter()
                .any(|locator| locator.starts_with(&format!("{directory}{prefix}"))),
            "{directory} catalogue is populated"
        );
    }
    markers.sort();
    actual.sort();
    assert_eq!(markers, SUCCESSOR_TREE_MARKERS);
    assert_eq!(
        actual,
        DOCUMENTS
            .iter()
            .map(|(locator, _, _)| (*locator).to_owned())
            .collect::<Vec<_>>()
    );

    let forbidden_keys = [
        "\"runtime_id\"",
        "\"runtime_ids\"",
        "\"wire_id\"",
        "\"wire_ids\"",
        "\"client_id\"",
        "\"client_ids\"",
        "\"legacy_id\"",
        "\"crystal_id\"",
    ];
    for (locator, byte_length, sha256) in DOCUMENTS {
        let bytes = fs::read(root.join(locator)).expect("read canonical package document");
        assert_eq!(bytes.len(), byte_length, "{locator}");
        assert_eq!(world_project_sha256(&bytes), sha256, "{locator}");
        let text = std::str::from_utf8(&bytes).expect("canonical JSON is UTF-8");
        for forbidden in forbidden_keys {
            assert!(!text.contains(forbidden), "{locator}: {forbidden}");
        }
    }
    assert_eq!(tree_digest(&root), TREE_SHA256);
}

#[test]
fn repository_package_recaptures_and_rewrites_without_identity_or_layer_drift() {
    let root = project_root();
    let project = capture_world_project(
        root.parent().expect("package has content parent"),
        OsStr::new("world"),
        filesystem_limits(),
    )
    .expect("capture tracked canonical package");
    assert_eq!(project.project_revision(), "g4-npc-wave-a-r9");
    assert_eq!(project.imports().len(), 12);
    let provenance = &project.imports()[1];
    assert_eq!(provenance.batch_id, "cw2-b1-full-item-family-registry-r1");
    assert_eq!(provenance.source_repository, "zimbadev/crystalserver");
    assert_eq!(
        provenance.source_revision,
        "ff7ede593c69d4c658b382c97443e8155926924a"
    );
    assert_eq!(
        provenance.source_artifact_sha256,
        "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7"
    );
    assert!(provenance.candidates.is_empty());
    assert!(provenance.reimport_states.is_empty());
    // ITEM-ADD-1: the donor identity epoch 2 batch, from its pinned donor items.xml.
    let donor_provenance = &project.imports()[0];
    assert_eq!(
        donor_provenance.batch_id,
        "cw2-b1-donor-identity-epoch-2-r1"
    );
    assert_eq!(donor_provenance.source_repository, "zimbadev/crystalserver");
    assert_eq!(
        donor_provenance.source_revision,
        CW2_B1_DONOR_EPOCH2_SOURCE_REVISION
    );
    assert_eq!(
        donor_provenance.source_artifact_sha256,
        CW2_B1_DONOR_EPOCH2_ITEMS_XML_SHA256
    );
    assert_eq!(
        donor_provenance.mapper_sha256,
        CW2_B1_DONOR_EPOCH2_CROSSWALK_SHA256
    );
    assert!(donor_provenance.candidates.is_empty());
    assert!(donor_provenance.reimport_states.is_empty());
    let creature_import = &project.imports()[2];
    assert_eq!(creature_import.batch_id, "g4-creature-canary-wave-a-r1");
    assert_eq!(creature_import.source_repository, "opentibiabr/canary");
    assert_eq!(
        creature_import.source_revision,
        "47dfd51f45280a59a1d3e50ba7edd573d7234446"
    );
    assert!(creature_import.candidates.is_empty());
    // E5: each admitted encounter keeps its manifest digest as an unchanged reimport baseline.
    assert_eq!(creature_import.reimport_states.len(), ENCOUNTERS);
    assert!(creature_import.reimport_states.iter().all(|state| {
        state.stable_identity.starts_with("oteryn:encounter.")
            && state.field_path == "encounter_manifest_sha256"
            && matches!(&state.baseline, Some(CandidateValue::Text(digest)) if digest.len() == 64)
            && state.upstream == state.baseline
            && state.local == state.baseline
            && state.decision == ReimportDecision::Unchanged
    }));
    let creature_crystal_import = &project.imports()[3];
    assert_eq!(
        creature_crystal_import.batch_id,
        "g4-creature-crystal-1530-r1"
    );
    assert_eq!(
        creature_crystal_import.source_repository,
        "zimbadev/crystalserver"
    );
    assert_eq!(
        creature_crystal_import.source_revision,
        "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
    );
    assert!(creature_crystal_import.candidates.is_empty());
    let wiki = &project.imports()[4];
    assert_eq!(wiki.batch_id, "g4-item-exact-165-tibiawiki-r1");
    assert_eq!(
        wiki.source_artifact_sha256,
        "583a0b0080f3e08633c8d6cde11d9fd073b47088d84774bfdf851382569dd675"
    );
    assert!(wiki.candidates.is_empty());
    let wave1_import = &project.imports()[5];
    assert_eq!(wave1_import.batch_id, "g4-item-wave1-tibiawiki-r1");
    assert_eq!(
        wave1_import.source_artifact_sha256,
        "5d8b84eee85e226e99d516beb7b40b8dc201c923e9b63b5ef18313085c3cbdf5"
    );
    assert_eq!(
        wave1_import.source_revision,
        "tibiawiki-item-wave1-snapshot:5d8b84eee85e226e99d516beb7b40b8dc201c923e9b63b5ef18313085c3cbdf5"
    );
    assert!(wave1_import.candidates.is_empty());
    let mount_import = &project.imports()[6];
    assert_eq!(mount_import.batch_id, "g4-mount-252-tibiawiki-r1");
    assert_eq!(
        mount_import.source_revision,
        "tibiawiki-nonitem-g1-snapshot:0b7caf98940305a91c5384dfb828c0afcf016f087f71572ec71d7c936c6387df"
    );
    assert_eq!(
        mount_import.source_artifact_sha256,
        "f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713"
    );
    assert!(mount_import.candidates.is_empty());
    assert!(mount_import.reimport_states.is_empty());

    let v2 = project.v2().expect("WorldProject/v2 state");
    assert_eq!(v2.declarations.len(), 385 + NPC_DECLARATIONS + ENCOUNTERS);
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Encounter { .. }))
            .count(),
        ENCOUNTERS
    );
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Npc { .. }))
            .count(),
        NPCS
    );
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Outfit { .. }))
            .count(),
        133
    );
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Dialogue { .. }))
            .count(),
        NPC_DIALOGUES
    );
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(
                declaration,
                ProjectV2Declaration::Npc {
                    dialogue: Some(_),
                    ..
                }
            ))
            .count(),
        NPC_DIALOGUES
    );
    assert_eq!(
        v2.declarations
            .iter()
            .filter(|declaration| matches!(declaration, ProjectV2Declaration::Mount { .. }))
            .count(),
        252
    );
    assert!(v2.declarations.iter().all(|declaration| match declaration {
        ProjectV2Declaration::Outfit {
            presentations,
            premium: None,
            acquisition_interactions,
            fields,
            ..
        } => presentations.is_empty() && acquisition_interactions.is_empty() && fields.is_empty(),
        ProjectV2Declaration::Mount {
            presentation: None,
            speed_bonus: None,
            premium: None,
            taming_item: None,
            acquisition_interactions,
            fields,
            ..
        } => acquisition_interactions.is_empty() && fields.is_empty(),
        ProjectV2Declaration::Npc {
            presentation: Some(_),
            behavior: Some(_),
            ..
        } => true,
        ProjectV2Declaration::Dialogue { fields, .. } => fields.is_empty(),
        ProjectV2Declaration::Service {
            recipes, fields, ..
        } => recipes.is_empty() && fields.is_empty(),
        ProjectV2Declaration::Encounter { fields, .. } => fields.is_empty(),
        _ => false,
    }));
    assert_eq!(v2.item_authoring.len(), 164);
    assert!(v2.item_authoring.iter().all(|entry| {
        entry.item.family == ProjectV2Family::Item
            && entry
                .taxonomy
                .as_ref()
                .is_some_and(|taxonomy| !taxonomy.primary.is_empty())
            && entry
                .forge
                .is_none_or(|forge| forge.classification > 0 && forge.max_tier > 0)
            && entry.proficiency.is_none()
            && entry.augments.is_empty()
            && entry.consumable.is_none()
    }));
    assert_eq!(
        v2.item_authoring
            .iter()
            .filter(|entry| entry.forge.is_some())
            .count(),
        146
    );
    assert_eq!(v2.authoring_profiles.len(), CREATURE_PROFILES + NPC_RECORDS);
    assert!(v2.authoring_profiles.iter().all(|profile| matches!(
        profile.target.family,
        ProjectV2Family::Creature
            | ProjectV2Family::Presentation
            | ProjectV2Family::Behavior
            | ProjectV2Family::Loot
            | ProjectV2Family::Ability
            | ProjectV2Family::Effect
            | ProjectV2Family::Formula
            | ProjectV2Family::Encounter
    )));
    assert_eq!(
        v2.authoring_profiles
            .iter()
            .filter(|profile| profile.target.family == ProjectV2Family::Encounter)
            .count(),
        ENCOUNTERS
    );
    assert!(v2.worlds.is_empty());
    assert!(v2.placements.is_empty());
    assert!(v2.appearance_bindings.is_empty());
    assert!(v2.assets.is_empty());
    assert_eq!(v2.sources.len(), 11);
    assert_eq!(v2.sources[0].key, "oteryn:source.canary");
    assert_eq!(v2.sources[0].import_batch_id, creature_import.batch_id);
    assert_eq!(v2.sources[0].revision, creature_import.source_revision);
    assert_eq!(v2.sources[0].sha256, creature_import.source_artifact_sha256);
    assert_eq!(
        v2.sources[0].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    // The creature Crystal batch has its own source revision, which keeps the commit.
    assert_eq!(v2.sources[2].key, "oteryn:source.crystalserver");
    assert_eq!(
        v2.sources[2].import_batch_id,
        creature_crystal_import.batch_id
    );
    assert_eq!(
        v2.sources[2].revision,
        "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
    );
    assert_eq!(
        v2.sources[2].revision,
        creature_crystal_import.source_revision
    );
    assert_eq!(
        v2.sources[2].sha256,
        creature_crystal_import.source_artifact_sha256
    );
    assert_eq!(
        v2.sources[2].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert_eq!(v2.sources[3].key, "oteryn:source.crystalserver");
    assert_eq!(v2.sources[3].import_batch_id, provenance.batch_id);
    assert_eq!(v2.sources[3].revision, provenance.source_revision);
    assert_eq!(v2.sources[3].sha256, provenance.source_artifact_sha256);
    assert_eq!(
        v2.sources[3].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert_eq!(v2.sources[5].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[5].import_batch_id, wiki.batch_id);
    assert_eq!(v2.sources[5].revision, wiki.source_revision);
    assert_eq!(v2.sources[5].sha256, wiki.source_artifact_sha256);
    assert_eq!(v2.sources[5].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[6].key, v2.sources[5].key);
    assert_eq!(v2.sources[6].import_batch_id, wave1_import.batch_id);
    assert_eq!(v2.sources[6].revision, wave1_import.source_revision);
    assert_eq!(v2.sources[6].sha256, wave1_import.source_artifact_sha256);
    assert_eq!(v2.sources[6].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[7].key, v2.sources[5].key);
    assert_eq!(v2.sources[7].import_batch_id, mount_import.batch_id);
    assert_eq!(v2.sources[7].revision, mount_import.source_revision);
    assert_eq!(v2.sources[7].sha256, mount_import.source_artifact_sha256);
    assert_eq!(v2.sources[7].evidence, ProjectV2EvidenceClass::Derived);
    // D12: offer prices both wikis agree on also come from the committed TibiaWiki BR facts.
    let npc_br_import = &project.imports()[8];
    assert_eq!(npc_br_import.batch_id, "g4-npc-prices-tibiawiki-br-r1");
    assert_eq!(npc_br_import.source_repository, "tibiawiki.com.br");
    assert_eq!(
        npc_br_import.source_artifact_sha256,
        "0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a"
    );
    assert!(npc_br_import.candidates.is_empty());
    assert_eq!(v2.sources[4].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[4].import_batch_id, npc_br_import.batch_id);
    assert_eq!(v2.sources[4].revision, npc_br_import.source_revision);
    assert_eq!(v2.sources[4].sha256, npc_br_import.source_artifact_sha256);
    assert_eq!(v2.sources[4].evidence, ProjectV2EvidenceClass::Derived);
    // D14: NPC files Crystal added after its pinned revision come from the pinned summer-update commit.
    let npc_supplement_import = &project.imports()[7];
    assert_eq!(
        npc_supplement_import.batch_id,
        "g4-npc-crystal-summer-supplement-r1"
    );
    assert_eq!(
        npc_supplement_import.source_repository,
        "zimbadev/crystalserver"
    );
    assert_eq!(
        npc_supplement_import.source_revision,
        "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
    );
    assert!(npc_supplement_import.candidates.is_empty());
    assert_eq!(v2.sources[1].key, "oteryn:source.crystalserver");
    assert_eq!(
        v2.sources[1].import_batch_id,
        npc_supplement_import.batch_id
    );
    assert_eq!(
        v2.sources[1].revision,
        npc_supplement_import.source_revision
    );
    assert_eq!(
        v2.sources[1].sha256,
        npc_supplement_import.source_artifact_sha256
    );
    // D13: offer prices two of three wikis agree on also come from the committed Tibiopedia facts.
    let npc_tibiopedia_import = &project.imports()[9];
    assert_eq!(
        npc_tibiopedia_import.batch_id,
        "g4-npc-prices-tibiopedia-r1"
    );
    assert_eq!(npc_tibiopedia_import.source_repository, "tibiopedia.pl");
    assert_eq!(
        npc_tibiopedia_import.source_generation_profile,
        "OTERYN_NPC_TIBIOPEDIA_FACTS/v1"
    );
    assert!(npc_tibiopedia_import.candidates.is_empty());
    assert_eq!(v2.sources[10].key, "oteryn:source.tibiawiki");
    assert_eq!(
        v2.sources[10].import_batch_id,
        npc_tibiopedia_import.batch_id
    );
    assert_eq!(
        v2.sources[10].revision,
        npc_tibiopedia_import.source_revision
    );
    assert_eq!(
        v2.sources[10].sha256,
        npc_tibiopedia_import.source_artifact_sha256
    );
    assert_eq!(v2.sources[10].evidence, ProjectV2EvidenceClass::Derived);
    let npc_import = &project.imports()[10];
    assert_eq!(npc_import.batch_id, "g4-npc-wave-a-tibiawiki-r9");
    assert!(npc_import.candidates.is_empty());
    assert_eq!(v2.sources[8].key, v2.sources[5].key);
    assert_eq!(v2.sources[8].import_batch_id, npc_import.batch_id);
    assert_eq!(v2.sources[8].revision, npc_import.source_revision);
    assert_eq!(v2.sources[8].evidence, ProjectV2EvidenceClass::Derived);
    // D44: creatures Tibia has at the target and Canary lacks, authored from TibiaWiki.
    let wiki_creature_import = &project.imports()[11];
    assert_eq!(
        wiki_creature_import.batch_id,
        "g4-wiki-authored-creature-d44-r1"
    );
    assert_eq!(wiki_creature_import.source_repository, "tibia.fandom.com");
    assert!(wiki_creature_import.candidates.is_empty());
    assert_eq!(v2.sources[9].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[9].import_batch_id, wiki_creature_import.batch_id);
    assert_eq!(v2.sources[9].revision, wiki_creature_import.source_revision);
    assert_eq!(v2.sources[9].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(
        v2.source_identity_bindings
            .iter()
            .filter(|binding| binding.source_revision == wiki_creature_import.source_revision)
            .map(|binding| (
                binding.identity_namespace.as_str(),
                binding.external_id.as_str(),
                binding.target.key.as_str(),
                binding.disposition
            ))
            .collect::<Vec<_>>(),
        [(
            "mediawiki/page_id",
            "108320",
            "oteryn:creature.dark_merudri",
            ProjectV2SourceIdentityDisposition::Exact
        )]
    );
    assert_eq!(
        v2.source_identity_bindings.len(),
        550 + CREATURES + ENCOUNTERS + NPC_BINDINGS
    );
    assert_eq!(v2.editor.len(), 550);
    let mut outfit_ids = BTreeSet::new();
    let mut mount_ids = BTreeSet::new();
    let mut item_ids = BTreeSet::new();
    let mut creature_files = BTreeSet::new();
    let mut npc_bindings = 0;
    let mut crystal_creatures = 0;
    let mut encounter_bindings = 0;
    for binding in &v2.source_identity_bindings {
        assert_eq!(
            binding.disposition,
            ProjectV2SourceIdentityDisposition::Exact
        );
        if binding.target.family == ProjectV2Family::Creature {
            if binding.source_key == "oteryn:source.tibiawiki" {
                // D44 wiki-authored creature, checked above.
                assert_eq!(binding.source_revision, v2.sources[9].revision);
                assert!(binding.target.key.starts_with("oteryn:creature."));
                continue;
            }
            if binding.source_key == "oteryn:source.crystalserver" {
                assert_eq!(binding.source_revision, v2.sources[2].revision);
                assert_eq!(binding.identity_namespace, "crystalserver/monster-file");
                assert!(binding.external_id.starts_with("data-global/monster/"));
                assert!(binding.target.key.starts_with("oteryn:creature."));
                assert!(creature_files.insert(&binding.external_id));
                crystal_creatures += 1;
                continue;
            }
            assert_eq!(binding.source_key, "oteryn:source.canary");
            assert_eq!(binding.source_revision, v2.sources[0].revision);
            assert_eq!(binding.identity_namespace, "canary/monster-file");
            assert!(binding.target.key.starts_with("oteryn:creature."));
            assert!(creature_files.insert(&binding.external_id));
            continue;
        }
        if binding.target.family == ProjectV2Family::Encounter {
            assert_eq!(binding.source_key, "oteryn:source.canary");
            assert_eq!(binding.source_revision, v2.sources[0].revision);
            assert_eq!(binding.identity_namespace, "canary/encounter");
            assert!(binding.target.key.starts_with("oteryn:encounter."));
            encounter_bindings += 1;
            continue;
        }
        if binding.target.family == ProjectV2Family::Npc {
            assert!(binding.target.key.starts_with("oteryn:npc."));
            assert!(matches!(
                (
                    binding.source_key.as_str(),
                    binding.identity_namespace.as_str()
                ),
                ("oteryn:source.canary", "canary/npc-file")
                    | ("oteryn:source.crystalserver", "crystalserver/npc-file")
                    | ("oteryn:source.tibiawiki", "mediawiki/page_id")
            ));
            npc_bindings += 1;
            continue;
        }
        assert_eq!(binding.source_key, "oteryn:source.tibiawiki");
        assert_eq!(binding.identity_namespace, "mediawiki/page_id");
        assert_eq!(
            binding.disposition,
            ProjectV2SourceIdentityDisposition::Exact
        );
        match binding.target.family {
            ProjectV2Family::Item => {
                assert_eq!(binding.source_revision, v2.sources[5].revision);
                assert!(item_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Outfit => {
                assert_eq!(binding.source_revision, v2.sources[7].revision);
                assert!(binding.target.key.starts_with("oteryn:content.outfit."));
                assert!(outfit_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Mount => {
                assert_eq!(binding.source_revision, v2.sources[7].revision);
                assert!(binding.target.key.starts_with("oteryn:content.mount."));
                assert!(mount_ids.insert(&binding.external_id));
            }
            _ => panic!("only Item, Outfit, Mount and Creature source bindings are populated"),
        }
    }
    assert_eq!(creature_files.len(), CREATURES - 1);
    assert_eq!(npc_bindings, NPC_BINDINGS);
    assert_eq!(encounter_bindings, ENCOUNTERS);
    assert_eq!(crystal_creatures, 37);
    assert_eq!(item_ids.len(), 165);
    assert_eq!(outfit_ids.len(), 133);
    assert!(outfit_ids.iter().all(|id| id.as_str() != "68724"));
    assert_eq!(mount_ids.len(), 252);
    assert!(item_ids.is_disjoint(&outfit_ids));
    assert!(item_ids.is_disjoint(&mount_ids));
    assert!(outfit_ids.is_disjoint(&mount_ids));
    let outfit_keys = v2
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            ProjectV2Declaration::Outfit { identity, .. } => Some(identity.key.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mount_keys = v2
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            ProjectV2Declaration::Mount { identity, .. } => Some(identity.key.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(outfit_keys.len(), 133);
    assert_eq!(mount_keys.len(), 252);
    assert!(
        v2.source_identity_bindings
            .iter()
            .filter(|binding| binding.target.family == ProjectV2Family::Outfit)
            .all(|binding| outfit_keys.contains(binding.target.key.as_str()))
    );
    assert!(
        v2.source_identity_bindings
            .iter()
            .filter(|binding| binding.target.family == ProjectV2Family::Mount)
            .all(|binding| mount_keys.contains(binding.target.key.as_str()))
    );
    let mut outfit_editor = 0;
    let mut mount_editor = 0;
    let mut item_editor = 0;
    for entry in &v2.editor {
        assert!(!entry.display_name.is_empty());
        assert!(entry.aliases.is_empty());
        match entry.target.family {
            ProjectV2Family::Item => {
                assert_eq!(entry.tags, ["oteryn:editor.item"]);
                item_editor += 1;
            }
            ProjectV2Family::Outfit => {
                assert_eq!(entry.tags, ["oteryn:editor.outfit"]);
                assert!(outfit_keys.contains(entry.target.key.as_str()));
                outfit_editor += 1;
            }
            ProjectV2Family::Mount => {
                assert_eq!(entry.tags, ["oteryn:editor.mount"]);
                assert!(mount_keys.contains(entry.target.key.as_str()));
                mount_editor += 1;
            }
            _ => panic!("only Item, Outfit and Mount editor entries are populated"),
        }
    }
    assert_eq!(item_editor, 165);
    assert_eq!(outfit_editor, 133);
    assert_eq!(mount_editor, 252);

    let rewritten = project
        .canonical_documents(limits())
        .expect("canonical deterministic rewrite");
    assert_eq!(rewritten.documents().len(), DOCUMENTS.len());
    for (locator, _, _) in DOCUMENTS {
        assert_eq!(
            rewritten.documents().get(locator).map(Vec::as_slice),
            Some(
                fs::read(root.join(locator))
                    .expect("read tracked canonical document")
                    .as_slice()
            ),
            "{locator}"
        );
    }

    let linked = project
        .link()
        .expect("link protected Item family and admitted creatures");
    assert_eq!(
        linked.definitions.len(),
        ITEMS + CREATURE_RECORDS + NPC_RECORDS
    );
    let keys = linked
        .definitions
        .iter()
        .map(|definition| definition.definition.key().as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), ITEMS + CREATURE_RECORDS + NPC_RECORDS);
    // Both historical gold-coin keys are retired; the Tibia key is the only identity.
    assert!(keys.contains(item_identity::semantic::CURRENCY_GOLD_COIN));
    assert!(!keys.contains(R7_P04_GOLD_COIN_KEY));
    assert!(!keys.contains(R7_P04_GOLD_COIN_OLD_KEY));
    assert!(!keys.contains(R7_P04_UNRELATED_REGISTRY_KEY));
    assert!(keys.contains("oteryn:item.tibia.i3147"));
    // ITEM-ADD-1: a donor epoch-2 id and an appearance-only id are Items; 48296 (no admitted
    // CipSoft appearance) stays out.
    assert!(keys.contains("oteryn:item.tibia.i54335"));
    assert!(keys.contains("oteryn:item.tibia.i21887"));
    assert!(!keys.contains(concat!("oteryn:item.tibia.", "i48296")));
    let gold_coin = linked
        .definitions
        .iter()
        .find(|definition| {
            definition.definition.key().as_str() == item_identity::semantic::CURRENCY_GOLD_COIN
        })
        .expect("repository Gold Coin definition");
    let ReferenceDefinitionKind::Item(gold_coin) = &gold_coin.kind else {
        panic!("Gold Coin is an Item");
    };
    assert!(gold_coin.materializable);
    assert_eq!(gold_coin.stack_class, ReferenceItemStackClass::StackCapable);
    // The #1048 lowering v1 packet admits exactly one typed field for source 3031
    // (`presentation.name`, lowercase per items.xml) once it resolves to the renamed
    // native key; every other semantics group remains Unknown.
    let ReferenceItemField::Known(gold_coin_presentation) = &gold_coin.semantics.presentation
    else {
        panic!("R7 P04 Gold Coin presentation unset by the lowering v1 promotion");
    };
    assert_eq!(
        gold_coin_presentation.name,
        ReferenceItemField::Known("gold coin".to_owned())
    );
    assert!(matches!(
        gold_coin_presentation.description,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(
        gold_coin.semantics.weapon,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(
        gold_coin.semantics.protection,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(
        gold_coin.semantics.charges,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(
        gold_coin.semantics.container,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(
        gold_coin.semantics.stack,
        ReferenceItemField::Unknown
    ));
    assert_eq!(
        linked
            .definitions
            .iter()
            .filter(|definition| matches!(definition.kind, ReferenceDefinitionKind::Creature(_)))
            .count(),
        CREATURES
    );
    let (promoted_items, promoted_fields) = linked
        .definitions
        .iter()
        .filter_map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Item(item) => Some(item),
            _ => None,
        })
        .fold((0_usize, 0_usize), |(items, fields), item| {
            let atoms = promoted_atom_count(&item.semantics);
            (items + usize::from(atoms > 0), fields + atoms)
        });
    // The 201 D149 records carried 204 promoted atoms; they left content with their records
    // (ITEM-ID-1b) and are kept in the tombstone archive.
    // ITEM-ADD-1: 23 donor epoch-2 Items carry 39 TibiaWiki atoms on these paths.
    assert_eq!(promoted_items, 12_301 - 201 + 23);
    // ITEM-SEM-2b adds 328 TibiaWiki atoms on these v1 paths where v1 had none; it replaces,
    // never removes, the others. Capacity adds 17 unknown atoms; declared charges add one.
    assert_eq!(
        promoted_fields,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT + 12 - 204 + 328 + 39 + 17 + 1
    );
    let (wave1_items, wave1_fields) = linked
        .definitions
        .iter()
        .filter_map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Item(item) => Some(item),
            _ => None,
        })
        .fold((0_usize, 0_usize), |(items, fields), item| {
            let atoms = wave1_atom_count(&item.semantics);
            (items + usize::from(atoms > 0), fields + atoms)
        });
    // Wave 1 promoted 290 atoms on 164 items; ITEM-SEM-2b adds TibiaWiki weapon types and
    // imbuement slot counts on the same paths for 1,269 more atoms (995 more items), and
    // ITEM-ADD-1 49 more on 27 donor epoch-2 Items. STARTER-CONTENT-1 adds `stackable: false`
    // on the backpack, which already has a slot count.
    // Explicit wiki non-stackability adds 2,345 atoms; 557 Items already had an atom
    // on these paths, so only 1,788 additional Items enter this atom census.
    // Physical fact initialization exposes one additional explicit wiki negative (i20129).
    // Documented-default No adds 1,487 leaves; 500 overlap this older atom census.
    // Seven genuine historical defaults add seven leaves; three overlap this census.
    // Affirmative official marketability adds 4,891 leaves without altering admission.
    assert_eq!(
        wave1_items,
        164 + 995 + 27 + 1_788 + 1 + 11 + 987 + 4 + 2519
    );
    assert_eq!(
        wave1_fields,
        290 + 1_269 + 49 + 1 + 2_345 + 1 + 11 + 1_487 + 7 + 4_891
    );
    // The declared timer has its own census: it is not one of the older v1/Wave 1 atoms.
    let (charge_fields, duration_fields) = linked
        .definitions
        .iter()
        .filter_map(|definition| {
            if let ReferenceDefinitionKind::Item(item) = &definition.kind {
                Some(item)
            } else {
                None
            }
        })
        .fold((0_usize, 0_usize), |(charges, durations), item| {
            let charge = matches!(
                &item.semantics.charges,
                ReferenceItemField::Known(ReferenceItemCharges {
                    count: ReferenceItemField::Known(_)
                })
            );
            let duration = matches!(
                &item.semantics.temporal,
                ReferenceItemField::Known(ReferenceItemTemporal {
                    duration: ReferenceItemField::Known(_),
                    ..
                })
            );
            (
                charges + usize::from(charge),
                durations + usize::from(duration),
            )
        });
    assert_eq!(charge_fields, 125 + 1);
    assert_eq!(duration_fields, 138);
    // Resistance vectors were entirely unknown in the predecessor. Count their typed
    // percentages as atoms so a missing list member cannot hide behind the vector count.
    let (mut resistance_vectors, mut resistance_atoms, mut equipment_patterns) = (0, 0, 0);
    let (mut pickup_fields, mut positive_stacks, mut weight_fields) = (0, 0, 0);
    let (mut movable_true, mut movable_false) = (0, 0);
    let (mut modifier_vectors, mut modifier_atoms) = (0, 0);
    for definition in &linked.definitions {
        let ReferenceDefinitionKind::Item(item) = &definition.kind else {
            continue;
        };
        if let ReferenceItemField::Known(physical) = &item.semantics.physical {
            pickup_fields += usize::from(matches!(
                physical.pickupable,
                ReferenceItemField::Known(true)
            ));
            weight_fields += usize::from(matches!(physical.weight, ReferenceItemField::Known(_)));
            movable_true += usize::from(physical.movable == ReferenceItemField::Known(true));
            movable_false += usize::from(physical.movable == ReferenceItemField::Known(false));
        }
        if let ReferenceItemField::Known(stack) = &item.semantics.stack
            && matches!(stack.stackable, ReferenceItemField::Known(true))
        {
            positive_stacks += 1;
            assert_eq!(stack.stack_max, ReferenceItemField::Known(100));
            let admitted = definition.definition.key().as_str() == "oteryn:item.tibia.i3155";
            assert_eq!(item.materializable, admitted);
            assert_eq!(
                (item.physical_class, item.stack_class),
                if admitted {
                    (
                        ReferenceItemPhysicalClass::Physical,
                        ReferenceItemStackClass::StackCapable,
                    )
                } else {
                    (
                        ReferenceItemPhysicalClass::Unknown,
                        ReferenceItemStackClass::Unknown,
                    )
                }
            );
        }
        if definition.definition.key().as_str() == "oteryn:item.tibia.i5801" {
            let ReferenceItemField::Known(physical) = &item.semantics.physical else {
                panic!("5801 physical facts must be known");
            };
            let ReferenceItemField::Known(container) = &item.semantics.container else {
                panic!("5801 container facts must remain known");
            };
            assert_eq!(physical.weight, ReferenceItemField::Known(1700));
            assert_eq!(container.capacity, ReferenceItemField::Known(22));
            assert!(!item.materializable);
            assert!(item.legal_destinations.is_empty());
        }
        if definition.definition.key().as_str() == "oteryn:item.tibia.i50275" {
            assert!(matches!(
                &item.semantics.protection,
                ReferenceItemField::Known(ReferenceItemProtection {
                    armor: ReferenceItemField::Known(ReferenceSignedPoints(8)),
                    resistances: ReferenceItemField::Unknown,
                })
            ));
        }
        if let ReferenceItemField::Known(protection) = &item.semantics.protection
            && let ReferenceItemField::Known(entries) = &protection.resistances
        {
            resistance_vectors += 1;
            resistance_atoms += entries.len();
            assert!(
                entries
                    .iter()
                    .all(|entry| matches!(entry.percent, ReferenceItemField::Known(_)))
            );
        }
        if definition.definition.key().as_str() == "oteryn:item.tibia.i34086" {
            assert_eq!(item.semantics.skill_modifiers, ReferenceItemField::Unknown);
        }
        if let ReferenceItemField::Known(group) = &item.semantics.skill_modifiers
            && let ReferenceItemField::Known(entries) = &group.modifiers
        {
            modifier_vectors += 1;
            modifier_atoms += entries.len();
            for entry in entries {
                assert_eq!(entry.target_domain, ReferenceItemField::Unknown);
                assert_eq!(entry.evaluation_phase, ReferenceItemField::Unknown);
                assert_eq!(entry.priority, ReferenceItemField::Unknown);
                assert!(matches!(entry.parameter, ReferenceItemField::Known(_)));
            }
        }
        if let ReferenceItemField::Known(equipment) = &item.semantics.equipment
            && let ReferenceItemField::Known(patterns) = &equipment.patterns
        {
            equipment_patterns += patterns.len();
        }
    }
    assert_eq!(
        (pickup_fields, positive_stacks, weight_fields),
        (6756, 39, 6513 + 4)
    );
    assert_eq!((movable_true, movable_false), (5692, 0));
    assert_eq!((modifier_vectors, modifier_atoms), (419, 619));
    assert_eq!(resistance_vectors, 391);
    assert_eq!(resistance_atoms, 625);
    // The independent predecessor census includes the separately admitted starter pattern.
    assert_eq!(equipment_patterns, 1_785 + 2);
}

#[test]
fn full_game_tree_contract_nodes_are_materialized_without_entering_legacy_package() {
    let repository = repository_root();
    let contract: serde_json::Value = serde_json::from_slice(
        &fs::read(repository.join(TREE_CONTRACT)).expect("read full game tree contract"),
    )
    .expect("full game tree contract is JSON");
    let nodes = contract["target_tree_nodes"]
        .as_array()
        .expect("contract target tree nodes");
    let directories = nodes
        .iter()
        .filter(|node| {
            node["path"]
                .as_str()
                .is_some_and(|path| path.ends_with('/'))
        })
        .collect::<Vec<_>>();
    assert_eq!(directories.len(), TREE_DIRECTORY_NODES);

    let mut world_markers = Vec::new();
    for node in &directories {
        let path = node["path"].as_str().expect("contract node path");
        assert!(!path.contains("..") && !path.starts_with('/'), "{path}");
        let marker = repository.join(path).join("index.json");
        let metadata = fs::symlink_metadata(&marker).expect("tree node index is materialized");
        assert!(metadata.is_file(), "{path}");
        let Some(locator) = path.strip_prefix("content/world/") else {
            continue;
        };
        let payload: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker).expect("read world tree marker"))
                .expect("world tree marker is JSON");
        world_markers.push(format!("{locator}index.json"));
        if payload["schema"] == "OTERYN_FAMILY_INDEX/v1" {
            // A populated successor family: shards stay in this directory, which holds no
            // legacy locator except `worlds/` (see `is_successor_shard`).
            assert_eq!(payload["population_state"], "POPULATED", "{path}");
            let shards = payload["shards"].as_array().expect("family index shards");
            assert!(!shards.is_empty(), "{path}");
            for shard in shards {
                let shard = shard.as_str().expect("family shard path");
                let shard_locator = shard
                    .strip_prefix("content/world/")
                    .unwrap_or_else(|| panic!("{shard}"));
                assert!(shard_locator.starts_with(locator), "{shard}");
                assert!(is_successor_shard(shard_locator), "{shard}");
            }
            continue;
        }
        assert_eq!(payload["schema"], "OTERYN_GAME_TREE_DIRECTORY/v1", "{path}");
        assert_eq!(payload["path"], path);
        assert_eq!(payload["kind"], node["kind"], "{path}");
        assert_eq!(payload["owner"], node["owner"], "{path}");
        let populated_catalogue = WORLD_CATALOGUE_SHARDS
            .iter()
            .any(|(directory, _)| *directory == locator);
        assert!(
            match payload["population_state"].as_str() {
                Some("READY_UNPOPULATED" | "LEGACY_COMPAT_PRESENT") => true,
                Some("POPULATED") => populated_catalogue,
                _ => false,
            },
            "{path}"
        );
    }
    world_markers.sort();
    assert_eq!(world_markers, SUCCESSOR_TREE_MARKERS);

    let legacy_locators = DOCUMENTS
        .iter()
        .map(|(locator, _, _)| *locator)
        .collect::<BTreeSet<_>>();
    assert!(
        SUCCESSOR_TREE_MARKERS
            .iter()
            .all(|marker| !legacy_locators.contains(marker))
    );

    let root = project_root();
    let project = capture_world_project(
        root.parent().expect("package has content parent"),
        OsStr::new("world"),
        filesystem_limits(),
    )
    .expect("capture tracked canonical package beside successor markers");
    let v2 = project.v2().expect("WorldProject/v2 state");
    assert!(v2.worlds.is_empty());
    assert!(v2.placements.is_empty());
    let rewritten = project
        .canonical_documents(limits())
        .expect("canonical deterministic rewrite");
    assert_eq!(
        rewritten
            .documents()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        legacy_locators
    );
    assert_eq!(tree_digest(&root), TREE_SHA256);
}
