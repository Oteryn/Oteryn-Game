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
        "90229d6d5a287d4598d2cb34da1046a2cc638f1b090f6db877e40bb01971ab9f",
    ),
    (
        "definitions/declarations.json",
        14_749_417,
        "76e76d5531c6337a41c805743b6289c8ab0ac46d1f15d1c4845c76e95f7e07cf",
    ),
    (
        "definitions/reference.json",
        22_561_121,
        "a28b4d553248ce184496a7c501bc54ec18682451b5fb0c2c85a8250496c2d8b8",
    ),
    (
        "editor/author.json",
        121_657,
        "a41044d85174f5c8f9a86c8433d880f43ea48a0c988d7772260a02eb78deea42",
    ),
    (
        "manifest.json",
        1937,
        "a03078dfa5868ae5adf4886e4eb86e4770f5e24493e34f06cef994c7458e6c85",
    ),
    (
        "presentations/bindings.json",
        73,
        "b7d27b2d7fde6370b7ae56c2c60412f2b213a4a5e92e7db82dd4b59a64359cd7",
    ),
    (
        "project.json",
        390,
        "44cdbbbd844ad895248005c87b28ceaf88fc11633486585feb07d8bfd5e6bbc7",
    ),
    (
        "provenance/imports.json",
        30_006,
        "9805797335b2f1823c4286fae0cd832b7dbc6243ce8f5b115d226d9019e53bff",
    ),
    (
        "provenance/sources.json",
        1_277_028,
        "b10e28a94b7062bb7d1b8126a3abc26e9ce91fac79ff6c2caeafafa11cf8318e",
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
const TREE_SHA256: &str = "7bc5b021efdca34369cbabd39f1cec8cfccf59d5eb0b9de4f937073fe1f66921";
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_120_000;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 43_000_000;
/// Canary creature admission pilot (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §7 slice 3).
const CREATURES: usize = 1450;
const CREATURE_RECORDS: usize = 20379;
const CREATURE_PROFILES: usize = 19435;
/// NPC admission wave A (OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1 §7 slice 4).
const NPCS: usize = 1088;
const NPC_RECORDS: usize = 2176;
const NPC_DECLARATIONS: usize = 2151;
const NPC_DIALOGUES: usize = 701;
const NPC_BINDINGS: usize = 2296;
/// Encounter admission (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 §5 slice 4).
const ENCOUNTERS: usize = 58;

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
        max_import_records: 9,
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
    assert_eq!(project.project_revision(), "g4-npc-wave-a-r5");
    assert_eq!(project.imports().len(), 9);
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
    let creature_import = &project.imports()[1];
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
    let wiki = &project.imports()[2];
    assert_eq!(wiki.batch_id, "g4-item-exact-165-tibiawiki-r1");
    assert_eq!(
        wiki.source_artifact_sha256,
        "583a0b0080f3e08633c8d6cde11d9fd073b47088d84774bfdf851382569dd675"
    );
    assert!(wiki.candidates.is_empty());
    let wave1_import = &project.imports()[3];
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
    let mount_import = &project.imports()[4];
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
    assert_eq!(v2.sources.len(), 9);
    assert_eq!(v2.sources[0].key, "oteryn:source.canary");
    assert_eq!(v2.sources[0].import_batch_id, creature_import.batch_id);
    assert_eq!(v2.sources[0].revision, creature_import.source_revision);
    assert_eq!(v2.sources[0].sha256, creature_import.source_artifact_sha256);
    assert_eq!(
        v2.sources[0].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert_eq!(v2.sources[1].key, "oteryn:source.crystalserver");
    assert_eq!(v2.sources[1].import_batch_id, provenance.batch_id);
    assert_eq!(v2.sources[1].revision, provenance.source_revision);
    assert_eq!(v2.sources[1].sha256, provenance.source_artifact_sha256);
    assert_eq!(
        v2.sources[1].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert_eq!(v2.sources[3].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[3].import_batch_id, wiki.batch_id);
    assert_eq!(v2.sources[3].revision, wiki.source_revision);
    assert_eq!(v2.sources[3].sha256, wiki.source_artifact_sha256);
    assert_eq!(v2.sources[3].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[4].key, v2.sources[3].key);
    assert_eq!(v2.sources[4].import_batch_id, wave1_import.batch_id);
    assert_eq!(v2.sources[4].revision, wave1_import.source_revision);
    assert_eq!(v2.sources[4].sha256, wave1_import.source_artifact_sha256);
    assert_eq!(v2.sources[4].evidence, ProjectV2EvidenceClass::Derived);
    assert_eq!(v2.sources[5].key, v2.sources[3].key);
    assert_eq!(v2.sources[5].import_batch_id, mount_import.batch_id);
    assert_eq!(v2.sources[5].revision, mount_import.source_revision);
    assert_eq!(v2.sources[5].sha256, mount_import.source_artifact_sha256);
    assert_eq!(v2.sources[5].evidence, ProjectV2EvidenceClass::Derived);
    // D12: offer prices both wikis agree on also come from the committed TibiaWiki BR facts.
    let npc_br_import = &project.imports()[5];
    assert_eq!(npc_br_import.batch_id, "g4-npc-prices-tibiawiki-br-r1");
    assert_eq!(npc_br_import.source_repository, "tibiawiki.com.br");
    assert_eq!(
        npc_br_import.source_artifact_sha256,
        "0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a"
    );
    assert!(npc_br_import.candidates.is_empty());
    assert_eq!(v2.sources[2].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[2].import_batch_id, npc_br_import.batch_id);
    assert_eq!(v2.sources[2].revision, npc_br_import.source_revision);
    assert_eq!(v2.sources[2].sha256, npc_br_import.source_artifact_sha256);
    assert_eq!(v2.sources[2].evidence, ProjectV2EvidenceClass::Derived);
    // D13: offer prices two of three wikis agree on also come from the committed Tibiopedia facts.
    let npc_tibiopedia_import = &project.imports()[6];
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
    assert_eq!(v2.sources[8].key, "oteryn:source.tibiawiki");
    assert_eq!(
        v2.sources[8].import_batch_id,
        npc_tibiopedia_import.batch_id
    );
    assert_eq!(
        v2.sources[8].revision,
        npc_tibiopedia_import.source_revision
    );
    assert_eq!(
        v2.sources[8].sha256,
        npc_tibiopedia_import.source_artifact_sha256
    );
    assert_eq!(v2.sources[8].evidence, ProjectV2EvidenceClass::Derived);
    let npc_import = &project.imports()[7];
    assert_eq!(npc_import.batch_id, "g4-npc-wave-a-tibiawiki-r5");
    assert!(npc_import.candidates.is_empty());
    assert_eq!(v2.sources[6].key, v2.sources[3].key);
    assert_eq!(v2.sources[6].import_batch_id, npc_import.batch_id);
    assert_eq!(v2.sources[6].revision, npc_import.source_revision);
    assert_eq!(v2.sources[6].evidence, ProjectV2EvidenceClass::Derived);
    // D44: creatures Tibia has at the target and Canary lacks, authored from TibiaWiki.
    let wiki_creature_import = &project.imports()[8];
    assert_eq!(
        wiki_creature_import.batch_id,
        "g4-wiki-authored-creature-d44-r1"
    );
    assert_eq!(wiki_creature_import.source_repository, "tibia.fandom.com");
    assert!(wiki_creature_import.candidates.is_empty());
    assert_eq!(v2.sources[7].key, "oteryn:source.tibiawiki");
    assert_eq!(v2.sources[7].import_batch_id, wiki_creature_import.batch_id);
    assert_eq!(v2.sources[7].revision, wiki_creature_import.source_revision);
    assert_eq!(v2.sources[7].evidence, ProjectV2EvidenceClass::Derived);
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
    let mut encounter_bindings = 0;
    for binding in &v2.source_identity_bindings {
        assert_eq!(
            binding.disposition,
            ProjectV2SourceIdentityDisposition::Exact
        );
        if binding.target.family == ProjectV2Family::Creature {
            if binding.source_key == "oteryn:source.tibiawiki" {
                // D44 wiki-authored creature, checked above.
                assert_eq!(binding.source_revision, v2.sources[7].revision);
                assert!(binding.target.key.starts_with("oteryn:creature."));
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
                assert_eq!(binding.source_revision, v2.sources[3].revision);
                assert!(item_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Outfit => {
                assert_eq!(binding.source_revision, v2.sources[5].revision);
                assert!(binding.target.key.starts_with("oteryn:content.outfit."));
                assert!(outfit_ids.insert(&binding.external_id));
            }
            ProjectV2Family::Mount => {
                assert_eq!(binding.source_revision, v2.sources[5].revision);
                assert!(binding.target.key.starts_with("oteryn:content.mount."));
                assert!(mount_ids.insert(&binding.external_id));
            }
            _ => panic!("only Item, Outfit, Mount and Creature source bindings are populated"),
        }
    }
    assert_eq!(creature_files.len(), CREATURES - 1);
    assert_eq!(npc_bindings, NPC_BINDINGS);
    assert_eq!(encounter_bindings, ENCOUNTERS);
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
        CW2_B1_FULL_ITEM_FAMILY_COUNT + CREATURE_RECORDS + NPC_RECORDS
    );
    let keys = linked
        .definitions
        .iter()
        .map(|definition| definition.definition.key().as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys.len(),
        CW2_B1_FULL_ITEM_FAMILY_COUNT + CREATURE_RECORDS + NPC_RECORDS
    );
    assert!(keys.contains(R7_P04_GOLD_COIN_KEY));
    assert!(!keys.contains(R7_P04_GOLD_COIN_OLD_KEY));
    assert!(keys.contains(R7_P04_UNRELATED_REGISTRY_KEY));
    let gold_coin = linked
        .definitions
        .iter()
        .find(|definition| definition.definition.key().as_str() == R7_P04_GOLD_COIN_KEY)
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
    assert_eq!(promoted_items, 12_301);
    assert_eq!(
        promoted_fields,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT + 12
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
