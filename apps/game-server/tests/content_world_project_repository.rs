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
        "a89ba184397ee9d8eb48848d948dcf72b8a2110ab40d6a6e0b38a04dcc7b8001",
    ),
    (
        "definitions/declarations.json",
        81_474,
        "8c691f73472ad14675c8860fc50511e72e51fc1b9e5edf372a679d32d9de6a0d",
    ),
    (
        "definitions/reference.json",
        7_475_358,
        "240a5467d795d911669edc3724601fa469961963eea257cc83fbdcc0ec1a4555",
    ),
    (
        "editor/author.json",
        121_657,
        "a41044d85174f5c8f9a86c8433d880f43ea48a0c988d7772260a02eb78deea42",
    ),
    (
        "manifest.json",
        1931,
        "a72ef2c1c99d8dbb7bf6b0c7ef7884e11b0087a370640a847216e345184401b0",
    ),
    (
        "presentations/bindings.json",
        73,
        "b7d27b2d7fde6370b7ae56c2c60412f2b213a4a5e92e7db82dd4b59a64359cd7",
    ),
    (
        "project.json",
        390,
        "f9442137d0e73611b1fc4b5d859c70a8e3d5f0d3d6dad40438169dbc04e5dd3e",
    ),
    (
        "provenance/imports.json",
        2677,
        "ea968bc64f96c00efb1a2c876f413499a38c9915e21e2c294b5cd14534aa5aab",
    ),
    (
        "provenance/sources.json",
        186_536,
        "80033ba3bdf94fa25a3aaab0c25ce2ba4d5acd6a84f870f12e98dee667ec8977",
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
const TREE_CONTRACT: &str =
    "docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json";
const TREE_DIRECTORY_NODES: usize = 97;
const TREE_SHA256: &str = "e3ef5bdd37d92eb5b8c117beb375d16ca10b8114cdc5f55dcefc139030557186";
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_120_000;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 43_000_000;

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
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: 4,
        max_reimport_states: 1,
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
        // each of the 11 locator lookups) plus 1 entry in worlds/: exactly 56 more.
        max_total_directory_entries_scanned: 144 + 56,
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
        assert!(matches!(&value.stack_max, ReferenceItemField::Unknown));
    }
    if let Known(value) = &semantics.trade_restrictions {
        count += usize::from(matches!(&value.marketable, Known(_)));
    }
    if let Known(value) = &semantics.physical {
        assert!(matches!(&value.weight, ReferenceItemField::Unknown));
    }
    count
}

#[test]
fn tracked_package_has_exact_inventory_digests_and_no_runtime_identity_layer() {
    let root = project_root();
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    let (mut markers, mut actual): (Vec<_>, Vec<_>) = files
        .into_iter()
        .partition(|locator| SUCCESSOR_TREE_MARKERS.contains(&locator.as_str()));
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
    assert_eq!(project.project_revision(), "g4-item-wave1-r1");
    assert_eq!(project.imports().len(), 4);
    let provenance = &project.imports()[0];
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
    let wiki = &project.imports()[1];
    assert_eq!(wiki.batch_id, "g4-item-exact-165-tibiawiki-r1");
    assert_eq!(
        wiki.source_artifact_sha256,
        "583a0b0080f3e08633c8d6cde11d9fd073b47088d84774bfdf851382569dd675"
    );
    assert!(wiki.candidates.is_empty());
    let wave1_import = &project.imports()[2];
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
    let mount_import = &project.imports()[3];
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
    assert_eq!(v2.declarations.len(), 385);
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
    assert!(v2.authoring_profiles.is_empty());
    assert!(v2.worlds.is_empty());
    assert!(v2.placements.is_empty());
    assert!(v2.appearance_bindings.is_empty());
    assert!(v2.assets.is_empty());
    assert_eq!(v2.sources.len(), 4);
    assert_eq!(v2.sources[0].key, "oteryn:source.crystalserver");
    assert_eq!(v2.sources[0].import_batch_id, provenance.batch_id);
    assert_eq!(v2.sources[0].revision, provenance.source_revision);
    assert_eq!(v2.sources[0].sha256, provenance.source_artifact_sha256);
    assert_eq!(
        v2.sources[0].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert_eq!(v2.sources[1].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[1].import_batch_id, wiki.batch_id);
    assert_eq!(v2.sources[1].revision, wiki.source_revision);
    assert_eq!(v2.sources[1].sha256, wiki.source_artifact_sha256);
    assert_eq!(v2.sources[1].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[2].key, v2.sources[1].key);
    assert_eq!(v2.sources[2].import_batch_id, wave1_import.batch_id);
    assert_eq!(v2.sources[2].revision, wave1_import.source_revision);
    assert_eq!(v2.sources[2].sha256, wave1_import.source_artifact_sha256);
    assert_eq!(v2.sources[2].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[3].key, v2.sources[1].key);
    assert_eq!(v2.sources[3].import_batch_id, mount_import.batch_id);
    assert_eq!(v2.sources[3].revision, mount_import.source_revision);
    assert_eq!(v2.sources[3].sha256, mount_import.source_artifact_sha256);
    assert_eq!(v2.sources[3].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.source_identity_bindings.len(), 550);
    assert_eq!(v2.editor.len(), 550);
    let mut outfit_ids = BTreeSet::new();
    let mut mount_ids = BTreeSet::new();
    let mut item_ids = BTreeSet::new();
    for binding in &v2.source_identity_bindings {
        assert_eq!(binding.source_key, "oteryn:source.tibiawiki");
        assert_eq!(binding.identity_namespace, "mediawiki/page_id");
        assert_eq!(
            binding.disposition,
            ProjectV2SourceIdentityDisposition::Exact
        );
        match binding.target.family {
            ProjectV2Family::Item => {
                assert_eq!(binding.source_revision, v2.sources[1].revision);
                assert!(item_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Outfit => {
                assert_eq!(binding.source_revision, v2.sources[3].revision);
                assert!(binding.target.key.starts_with("oteryn:content.outfit."));
                assert!(outfit_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Mount => {
                assert_eq!(binding.source_revision, v2.sources[3].revision);
                assert!(binding.target.key.starts_with("oteryn:content.mount."));
                assert!(mount_ids.insert(&binding.external_id));
            }
            _ => panic!("only Item, Outfit and Mount source bindings are populated"),
        }
    }
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

    let linked = project.link().expect("link protected Item family");
    assert_eq!(linked.definitions.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    let keys = linked
        .definitions
        .iter()
        .map(|definition| definition.definition.key().as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    let (promoted_items, promoted_fields) =
        linked
            .definitions
            .iter()
            .fold((0_usize, 0_usize), |(items, fields), definition| {
                let ReferenceDefinitionKind::Item(item) = &definition.kind else {
                    panic!("repository seed contains a non-Item reference definition");
                };
                let atoms = promoted_atom_count(&item.semantics);
                (items + usize::from(atoms > 0), fields + atoms)
            });
    assert_eq!(promoted_items, 178);
    assert_eq!(promoted_fields, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT + 526);
    let (wave1_items, wave1_fields) =
        linked
            .definitions
            .iter()
            .fold((0_usize, 0_usize), |(items, fields), definition| {
                let ReferenceDefinitionKind::Item(item) = &definition.kind else {
                    panic!("repository seed contains a non-Item reference definition");
                };
                let atoms = wave1_atom_count(&item.semantics);
                (items + usize::from(atoms > 0), fields + atoms)
            });
    assert_eq!(wave1_items, 164);
    assert_eq!(wave1_fields, 290);
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
        assert_eq!(payload["schema"], "OTERYN_GAME_TREE_DIRECTORY/v1", "{path}");
        assert_eq!(payload["path"], path);
        assert_eq!(payload["kind"], node["kind"], "{path}");
        assert_eq!(payload["owner"], node["owner"], "{path}");
        assert!(
            matches!(
                payload["population_state"].as_str(),
                Some("READY_UNPOPULATED" | "LEGACY_COMPAT_PRESENT")
            ),
            "{path}"
        );
        world_markers.push(format!("{locator}index.json"));
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
