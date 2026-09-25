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
        400,
        "b03a1d70f3f37edbe45bf69b97f514cacf54e361a95e413d8224410cd1fe58db",
    ),
    (
        "definitions/declarations.json",
        63,
        "b9e6f5eaae263f0575e848fd5fd6a368d04e3d2d54b1d87042787abdf8fde278",
    ),
    (
        "definitions/reference.json",
        7_309_278,
        "614ce98b2fd13e173772540702b5b05f7cd3ffc4cd2ab91d695ca7fa3470adaa",
    ),
    (
        "editor/author.json",
        77,
        "91836a3e71fc6b6ad0918deb05e4b5faeeeffc6da421e4bd0ee7e40a2aaeffe5",
    ),
    (
        "manifest.json",
        1_932,
        "639e9542f285e82cfe15486c6858a2a56b14b2536316f21a18bb512babb0259d",
    ),
    (
        "presentations/bindings.json",
        73,
        "b7d27b2d7fde6370b7ae56c2c60412f2b213a4a5e92e7db82dd4b59a64359cd7",
    ),
    (
        "project.json",
        402,
        "d866e2f2c10af713bac4de3ce0fac0ab927969fca715be5453a149649dbacbb3",
    ),
    (
        "provenance/imports.json",
        719,
        "9071e9d696bbeaa624ed0d13cc4bcddd0f972771dc8063f7aea0032ca1b7b7f3",
    ),
    (
        "provenance/sources.json",
        315,
        "0713724dbbaa6219018773a7a9e2265d24e307214d469fb2587df8bc0ff48bd9",
    ),
    (
        "worlds/world.json",
        72,
        "0dcc223c4a904834a58b3ad2b1c7882636cc66c9123aafdb25bb69a1d4670dfa",
    ),
];
const TREE_SHA256: &str = "eafdbb03dfc9bdbfe3570b4e31f0fa5a84d1e5a81a83ea6485f7e20fed19043c";
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_098_651;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 42_332_603;

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/world")
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
        max_import_records: 1,
        max_reimport_states: 1,
    }
}

fn filesystem_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: limits(),
        max_entries_per_directory_scan: 32,
        max_total_directory_entries_scanned: 128,
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

#[test]
fn tracked_package_has_exact_inventory_digests_and_no_runtime_identity_layer() {
    let root = project_root();
    let mut actual = Vec::new();
    collect_files(&root, &root, &mut actual);
    actual.sort();
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
    assert_eq!(project.project_revision(), "g4-canonical-worldproject-r1");
    assert_eq!(project.imports().len(), 1);
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

    let v2 = project.v2().expect("WorldProject/v2 state");
    assert!(v2.declarations.is_empty());
    assert!(v2.item_authoring.is_empty());
    assert!(v2.authoring_profiles.is_empty());
    assert!(v2.worlds.is_empty());
    assert!(v2.placements.is_empty());
    assert!(v2.appearance_bindings.is_empty());
    assert!(v2.assets.is_empty());
    assert_eq!(v2.sources.len(), 1);
    assert_eq!(v2.sources[0].key, "oteryn:source.crystalserver");
    assert_eq!(v2.sources[0].import_batch_id, provenance.batch_id);
    assert_eq!(v2.sources[0].revision, provenance.source_revision);
    assert_eq!(v2.sources[0].sha256, provenance.source_artifact_sha256);
    assert_eq!(
        v2.sources[0].evidence,
        ProjectV2EvidenceClass::OtsHypothesisOnly
    );
    assert!(v2.source_identity_bindings.is_empty());
    assert!(v2.editor.is_empty());

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
    assert_eq!(promoted_items, ITEM_SEMANTIC_PROMOTION_ITEM_COUNT);
    assert_eq!(promoted_fields, ITEM_SEMANTIC_PROMOTION_FIELD_COUNT);
}
