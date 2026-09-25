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
        376,
        "cf1071b0026ed6417cf33ee892c54ceaa55b8739bde8c252cd93f72bc5c98620",
    ),
    (
        "definitions/declarations.json",
        63,
        "b9e6f5eaae263f0575e848fd5fd6a368d04e3d2d54b1d87042787abdf8fde278",
    ),
    (
        "definitions/reference.json",
        7_458_197,
        "6bd267cf3c272f36ec20f4eecdf0048eb0a5439074a513ff6060b2dfe6693309",
    ),
    (
        "editor/author.json",
        35_113,
        "f3cfb2e4c04abe2e38d11cbc514908655fd7c0fb9996fe9763fe9311aa93c4b6",
    ),
    (
        "manifest.json",
        1_930,
        "acddd89b388fddec1c54d5389de6d8ab8ae618b267a48da28104299f605dce91",
    ),
    (
        "presentations/bindings.json",
        73,
        "b7d27b2d7fde6370b7ae56c2c60412f2b213a4a5e92e7db82dd4b59a64359cd7",
    ),
    (
        "project.json",
        394,
        "269b09102b4b9e7d458bb18a35f6107ecbce8cf271289a903603fd833d662898",
    ),
    (
        "provenance/imports.json",
        1_386,
        "0f63720ce6763f540240d70e4b05e0aafa07721a6b3614160a3f5f39cb03d16c",
    ),
    (
        "provenance/sources.json",
        54_302,
        "81f55d61ae47a6032f906e3a189b88ebf4319ca2eac48fbecf9bf0125ee3619d",
    ),
    (
        "worlds/world.json",
        72,
        "0dcc223c4a904834a58b3ad2b1c7882636cc66c9123aafdb25bb69a1d4670dfa",
    ),
];
const TREE_SHA256: &str = "2649b6f2b6cfff35014f078f32530c66508266e5bf6e3ebea01491b596209c06";
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_120_000;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 43_000_000;

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
        max_import_records: 2,
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
    assert_eq!(project.project_revision(), "g4-item-exact-165-r1");
    assert_eq!(project.imports().len(), 2);
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

    let v2 = project.v2().expect("WorldProject/v2 state");
    assert!(v2.declarations.is_empty());
    assert!(v2.item_authoring.is_empty());
    assert!(v2.authoring_profiles.is_empty());
    assert!(v2.worlds.is_empty());
    assert!(v2.placements.is_empty());
    assert!(v2.appearance_bindings.is_empty());
    assert!(v2.assets.is_empty());
    assert_eq!(v2.sources.len(), 2);
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
    assert_eq!(v2.source_identity_bindings.len(), 165);
    assert_eq!(v2.editor.len(), 165);
    for binding in &v2.source_identity_bindings {
        assert_eq!(binding.source_key, v2.sources[1].key);
        assert_eq!(binding.source_revision, v2.sources[1].revision);
        assert_eq!(binding.identity_namespace, "mediawiki/page_id");
        assert_eq!(binding.target.family, ProjectV2Family::Item);
        assert_eq!(
            binding.disposition,
            ProjectV2SourceIdentityDisposition::Exact
        );
    }
    for entry in &v2.editor {
        assert!(!entry.display_name.is_empty());
        assert_eq!(entry.tags, ["oteryn:editor.item"]);
        assert!(entry.aliases.is_empty());
    }

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
}
