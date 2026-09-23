#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const B4_EVIDENCE: &str = include_str!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json"
);
const EXACT_SCAN_ENTRIES: usize = 6;
const EXACT_TOTAL_SCANNED: usize = 40;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn project_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 16_384,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 16,
        max_import_records: 8,
        max_reimport_states: 32,
    }
}

fn filesystem_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: project_limits(),
        max_entries_per_directory_scan: 64,
        max_total_directory_entries_scanned: 1_024,
    }
}

fn exact_filesystem_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: project_limits(),
        max_entries_per_directory_scan: EXACT_SCAN_ENTRIES,
        max_total_directory_entries_scanned: EXACT_TOTAL_SCANNED,
    }
}

fn identity(family: &str, key: &str) -> DefinitionIdentityDocument {
    DefinitionIdentityDocument {
        family: family.to_owned(),
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn reference(family: &str, key: &str) -> DefinitionReferenceDocument {
    DefinitionReferenceDocument {
        family: family.to_owned(),
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn project_records() -> Vec<ProjectReferenceRecord> {
    vec![
        ProjectReferenceRecord::Creature {
            identity: identity(
                "Creature",
                "oteryn:reference.creature.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ClientSafe,
            presentation: reference(
                "Presentation",
                "oteryn:reference.presentation.project-owned-courier",
            ),
            behavior: reference(
                "Behavior",
                "oteryn:reference.behavior.project-owned-courier",
            ),
            loot: None,
        },
        ProjectReferenceRecord::Item {
            identity: identity("Item", "oteryn:reference.item.project-owned-token"),
            client_projection: ProjectionDocument::ClientSafe,
            materializable: true,
            stack_class: ItemStackDocument::StackCapable,
            semantics: Default::default(),
        },
        ProjectReferenceRecord::Generic {
            identity: identity(
                "Presentation",
                "oteryn:reference.presentation.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ClientSafe,
        },
        ProjectReferenceRecord::Generic {
            identity: identity(
                "Behavior",
                "oteryn:reference.behavior.project-owned-courier",
            ),
            client_projection: ProjectionDocument::ServerOnly,
        },
    ]
}

fn b4_batch() -> ImportBatch {
    let evidence: Value = serde_json::from_str(B4_EVIDENCE).expect("protected B4 evidence parses");
    let candidates = evidence["ability_effect_formula_candidates"]
        .as_array()
        .expect("B4 candidates")
        .iter()
        .map(|candidate| {
            let operation = match candidate["ability_to_effect"]["candidate_family"]
                .as_str()
                .expect("candidate family")
            {
                "DAMAGE" => ImportCandidateOperation::Damage,
                "HEAL" => ImportCandidateOperation::Heal,
                unexpected => panic!("unexpected protected B4 family: {unexpected}"),
            };
            assert!(candidate["native_ability_identity"]["content_key"].is_null());
            assert_eq!(candidate["executable_promotion"]["disposition"], "BLOCKED");
            ImportCandidate {
                source_candidate_id: candidate["source_candidate_id"]
                    .as_str()
                    .expect("source candidate id")
                    .to_owned(),
                source_label: candidate["display_name"]
                    .as_str()
                    .expect("display name")
                    .to_owned(),
                source_numeric_id: None,
                candidate_family: ImportCandidateFamily::AbilityEffectFormula,
                candidate_operation: operation,
                candidate_target: candidate["ability_to_effect"]["candidate_shape"]
                    .as_str()
                    .expect("candidate target")
                    .to_owned(),
                candidate_formula: candidate["effect_to_formula"]["formula_state"]
                    .as_str()
                    .expect("formula state")
                    .to_owned(),
                evidence_class: candidate["target_evidence"]
                    .as_str()
                    .expect("target evidence")
                    .to_owned(),
                closure_disposition: CandidateDisposition::Blocked,
                disposition_reason: candidate["executable_promotion"]["reason_codes"]
                    .as_array()
                    .expect("reason codes")
                    .iter()
                    .map(|reason| reason.as_str().expect("reason"))
                    .collect::<Vec<_>>()
                    .join(","),
                normalized_fields: vec![
                    NamedCandidateField {
                        field_path: "candidate.formula-state".to_owned(),
                        value: CandidateValue::Text("UNKNOWN".to_owned()),
                    },
                    NamedCandidateField {
                        field_path: "candidate.target-evidence".to_owned(),
                        value: CandidateValue::Text("UNKNOWN".to_owned()),
                    },
                ],
            }
        })
        .collect();

    ImportBatch {
        batch_id: "cw2-b4-ability-effect-formula".to_owned(),
        source_repository: evidence["source_snapshot"]["repository"]
            .as_str()
            .expect("repository")
            .to_owned(),
        source_revision: evidence["source_snapshot"]["revision"]
            .as_str()
            .expect("revision")
            .to_owned(),
        source_artifact_sha256: "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491"
            .to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: evidence["schema"].as_str().expect("schema").to_owned(),
        importer: "repository-protected-cw2-b4-evidence".to_owned(),
        mapper: evidence["mapper_profile"]
            .as_str()
            .expect("mapper profile")
            .to_owned(),
        mapper_revision: evidence["mapper_revision"]["git_blob"]
            .as_str()
            .expect("mapper git blob")
            .to_owned(),
        mapper_sha256: evidence["mapper_revision"]["canonical_sha256"]
            .as_str()
            .expect("mapper sha256")
            .to_owned(),
        candidates,
        reimport_states: vec![ReimportFieldState {
            stable_identity: "reference-source:ability:ice_strike".to_owned(),
            field_path: "candidate.formula-state".to_owned(),
            baseline: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            upstream: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            local: Some(CandidateValue::Text("UNKNOWN".to_owned())),
            decision: ReimportDecision::Unchanged,
        }],
    }
}

fn draft() -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".to_owned(),
        package_key: "oteryn:content.world-project".to_owned(),
        semantic_schema_version: "reference-schema-v1".to_owned(),
        licensing_metadata: "license:project-owned-v1".to_owned(),
        world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
        coordinate_frame: "global-target-2026-07-28".to_owned(),
        records: project_records(),
        imports: vec![b4_batch()],
        metadata: vec![AuthorMetadataEntry {
            stable_identity: "oteryn:reference.item.project-owned-token".to_owned(),
            display_name: "Project Token".to_owned(),
            description: "Author-facing description".to_owned(),
            categories: vec!["editor-category".to_owned()],
            notes: vec!["Does not select gameplay behavior".to_owned()],
        }],
    }
}

fn canonical_documents() -> BTreeMap<String, Vec<u8>> {
    CanonicalProjectDocuments::from_draft(draft(), project_limits())
        .expect("canonical B4-containing project")
        .documents()
        .clone()
}

#[test]
fn v1_to_v2_migration_publishes_and_recaptures_all_project_roles() {
    let fixture = Fixture::new();
    let existing = fixture.capture().expect("admit v1 before migration");
    let documents = CanonicalProjectDocuments::from_v2_draft(
        existing.migrate_to_v2(), project_limits(),
    ).expect("canonical v2 migration");
    assert_eq!(documents.documents().len(), 11);
    publish_world_project(&fixture.base, OsStr::new("project-root"), &documents, filesystem_limits())
        .expect("coherent v2 publication");
    let captured = fixture.capture().expect("admit published v2");
    assert_eq!(captured.v2(), Some(&ProjectV2State::default()));
    assert_eq!(captured.canonical_documents(project_limits()).expect("rewrite v2").documents(), documents.documents());
}

struct Fixture {
    base: PathBuf,
    root: PathBuf,
    documents: BTreeMap<String, Vec<u8>>,
}

impl Fixture {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "oteryn-project-fs-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&base).expect("create isolated fixture parent");
        let root = base.join("project-root");
        fs::create_dir(&root).expect("create fixture root");
        let documents = canonical_documents();
        for (locator, bytes) in &documents {
            let path = root.join(locator);
            fs::create_dir_all(path.parent().expect("document parent"))
                .expect("create document directory");
            fs::write(path, bytes).expect("write canonical document");
        }
        Self {
            base,
            root,
            documents,
        }
    }

    fn capture(&self) -> Result<WorldProject, ProjectFilesystemError> {
        capture_world_project(&self.base, OsStr::new("project-root"), filesystem_limits())
    }

    fn path(&self, locator: &str) -> PathBuf {
        self.root.join(locator)
    }

    fn replace_with_symlink(&self, locator: &str) {
        let path = self.path(locator);
        let outside = self.base.join("outside-file");
        fs::write(&outside, b"outside").expect("write outside target");
        fs::remove_file(&path).expect("remove canonical file");
        symlink(outside, path).expect("create source symlink");
    }

    fn rewrite_manifest(&mut self, change: impl FnOnce(&mut Value)) {
        let mut manifest: Value = serde_json::from_slice(&self.documents["manifest.json"])
            .expect("parse canonical manifest");
        change(&mut manifest);
        let manifest_bytes = canonical_value(&manifest);
        let manifest_digest = world_project_sha256(&manifest_bytes);
        let package = PackageManifestBinding::new(
            ProductionKey::new(
                manifest["package_key"]
                    .as_str()
                    .expect("manifest package key"),
            )
            .expect("valid manifest package key"),
            ProductionAtom::new(
                "project package revision",
                manifest["package_revision"]
                    .as_str()
                    .expect("manifest package revision"),
            )
            .expect("valid package revision"),
            ProductionAtom::new(
                "project semantic schema",
                manifest["semantic_schema_version"]
                    .as_str()
                    .expect("manifest semantic schema"),
            )
            .expect("valid semantic schema"),
            ProductionAtom::new(
                "project licensing metadata",
                manifest["licensing_metadata"]
                    .as_str()
                    .expect("manifest licensing metadata"),
            )
            .expect("valid licensing metadata"),
            Sha256HexDigest::new(&manifest_digest).expect("valid manifest digest"),
        );
        let mut lock: Value = serde_json::from_slice(&self.documents["content.lock.json"])
            .expect("parse canonical lock");
        lock["entries"][0]["package_provenance_digest"] = Value::String(
            package
                .package_provenance_digest()
                .expect("derive package provenance")
                .as_str()
                .to_owned(),
        );
        let lock_bytes = canonical_value(&lock);
        let mut root: Value =
            serde_json::from_slice(&self.documents["project.json"]).expect("parse canonical root");
        root["manifest_sha256"] = Value::String(manifest_digest);
        root["content_lock_sha256"] = Value::String(world_project_sha256(&lock_bytes));
        let root_bytes = canonical_value(&root);
        self.documents
            .insert("manifest.json".to_owned(), manifest_bytes.clone());
        self.documents
            .insert("project.json".to_owned(), root_bytes.clone());
        self.documents
            .insert("content.lock.json".to_owned(), lock_bytes.clone());
        fs::write(self.path("manifest.json"), manifest_bytes).expect("rewrite manifest");
        fs::write(self.path("content.lock.json"), lock_bytes).expect("rewrite lock");
        fs::write(self.path("project.json"), root_bytes).expect("rebind project root");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn canonical_value(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("serialize JSON mutation");
    bytes.push(b'\n');
    bytes
}

fn assert_unsafe(result: Result<WorldProject, ProjectFilesystemError>) {
    assert!(
        matches!(
            result,
            Err(ProjectFilesystemError::UnsafeEntry { .. })
                | Err(ProjectFilesystemError::Io { .. })
        ),
        "expected unsafe filesystem rejection: {result:?}"
    );
}

#[test]
fn real_b4_project_capture_equals_the_existing_snapshot_parser() {
    let fixture = Fixture::new();
    let expected = ProjectSnapshot::new(fixture.documents.clone(), project_limits())
        .expect("in-memory snapshot")
        .parse(project_limits())
        .expect("in-memory parse");
    let captured = capture_world_project(
        &fixture.base,
        OsStr::new("project-root"),
        exact_filesystem_limits(),
    )
    .expect("secure filesystem capture");
    assert_eq!(captured, expected);
    assert_eq!(captured.imports()[0].candidates.len(), 2);
    assert!(
        captured.imports()[0]
            .candidates
            .iter()
            .all(|candidate| candidate.closure_disposition == CandidateDisposition::Blocked)
    );
}

#[test]
fn root_directory_and_document_links_fail_closed() {
    let root_link = Fixture::new();
    let real_root = root_link.base.join("real-root");
    fs::rename(&root_link.root, &real_root).expect("move real root");
    symlink(&real_root, &root_link.root).expect("create root symlink");
    assert_unsafe(root_link.capture());

    let nested_link = Fixture::new();
    let real_records = nested_link.base.join("real-records");
    fs::rename(nested_link.path("records"), &real_records).expect("move records directory");
    symlink(real_records, nested_link.path("records")).expect("create nested directory symlink");
    assert_unsafe(nested_link.capture());

    for locator in [
        "project.json",
        "content.lock.json",
        "records/reference.json",
    ] {
        let fixture = Fixture::new();
        fixture.replace_with_symlink(locator);
        assert_unsafe(fixture.capture());
    }
}

#[test]
fn special_files_directories_and_hardlinks_fail_closed() {
    let directory = Fixture::new();
    fs::remove_file(directory.path("records/reference.json")).expect("remove source file");
    fs::create_dir(directory.path("records/reference.json")).expect("create directory as source");
    assert_unsafe(directory.capture());

    let socket = Fixture::new();
    fs::remove_file(socket.path("records/reference.json")).expect("remove source file");
    let records = fs::File::open(socket.path("records")).expect("open records directory");
    rustix::fs::mknodat(
        &records,
        "reference.json",
        rustix::fs::FileType::Socket,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .expect("create socket node as source");
    assert_unsafe(socket.capture());

    let fifo = Fixture::new();
    fs::remove_file(fifo.path("records/reference.json")).expect("remove source file");
    let records = fs::File::open(fifo.path("records")).expect("open records directory");
    rustix::fs::mkfifoat(
        &records,
        "reference.json",
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .expect("create FIFO as source");
    assert_unsafe(fifo.capture());

    let outside_hardlink = Fixture::new();
    let source = outside_hardlink.path("records/reference.json");
    let outside = outside_hardlink.base.join("outside-hardlink");
    fs::hard_link(&source, &outside).expect("create outside hardlink");
    assert_unsafe(outside_hardlink.capture());

    let shared_identity = Fixture::new();
    let records = shared_identity.path("records/reference.json");
    let imports = shared_identity.path("imports/candidates.json");
    fs::remove_file(&imports).expect("remove import source");
    fs::hard_link(records, imports).expect("alias two locators to one inode");
    assert_unsafe(shared_identity.capture());
}

#[test]
fn exact_spelling_and_root_basename_are_enforced_before_path_resolution() {
    let fixture = Fixture::new();
    fs::rename(fixture.path("records"), fixture.path("Records")).expect("change actual entry case");
    assert_unsafe(fixture.capture());

    let nonexistent = Path::new("/definitely/not/an/oteryn/project/parent");
    let error = capture_world_project(nonexistent, OsStr::new("nested/root"), filesystem_limits())
        .expect_err("multi-component root basename rejected before parent access");
    assert!(matches!(error, ProjectFilesystemError::UnsafeEntry { .. }));
}

#[test]
fn malformed_manifest_locators_fail_before_source_filesystem_access() {
    for invalid in ["../outside.json", "/absolute.json", "records\\escape.json"] {
        let mut fixture = Fixture::new();
        fixture.rewrite_manifest(|manifest| {
            manifest["documents"][0]["locator"] = Value::String(invalid.to_owned());
        });
        fs::remove_dir_all(fixture.path("records")).expect("remove source tree");
        fs::remove_dir_all(fixture.path("imports")).expect("remove source tree");
        fs::remove_dir_all(fixture.path("metadata")).expect("remove source tree");
        assert!(matches!(
            fixture.capture(),
            Err(ProjectFilesystemError::Project(ProjectError::InvalidLocator(value)))
                if value == invalid
        ));
    }
}

#[test]
fn manifest_length_digest_and_control_coherence_fail_closed() {
    let mut length = Fixture::new();
    length.rewrite_manifest(|manifest| {
        let current = manifest["documents"][0]["byte_length"]
            .as_u64()
            .expect("manifest length");
        manifest["documents"][0]["byte_length"] = json!(current + 1);
    });
    assert_unsafe(length.capture());

    let digest = Fixture::new();
    let source = digest.path("records/reference.json");
    let mut bytes = fs::read(&source).expect("read source");
    bytes[0] ^= 1;
    fs::write(source, bytes).expect("mutate same-length source");
    assert!(matches!(
        digest.capture(),
        Err(ProjectFilesystemError::Project(
            ProjectError::DigestMismatch(_)
        ))
    ));

    for control in ["manifest.json", "content.lock.json"] {
        let fixture = Fixture::new();
        let path = fixture.path(control);
        let mut bytes = fs::read(&path).expect("read control");
        bytes[0] ^= 1;
        fs::write(path, bytes).expect("mutate bound control");
        assert!(matches!(
            fixture.capture(),
            Err(ProjectFilesystemError::Project(
                ProjectError::DigestMismatch(_)
            ))
        ));
    }
}

#[test]
fn sparse_oversize_is_rejected_from_metadata_before_allocation() {
    let fixture = Fixture::new();
    let source = fs::OpenOptions::new()
        .write(true)
        .open(fixture.path("records/reference.json"))
        .expect("open sparse source");
    source
        .set_len(u64::try_from(project_limits().max_document_bytes).expect("limit fits") + 1)
        .expect("extend sparse source");
    assert!(matches!(
        fixture.capture(),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "project document bytes",
            ..
        })
    ));
}

#[test]
fn measured_scan_and_byte_limits_accept_exact_and_reject_max_plus_one() {
    let fixture = Fixture::new();
    capture_world_project(
        &fixture.base,
        OsStr::new("project-root"),
        exact_filesystem_limits(),
    )
    .expect("exact measured scan budgets");

    let mut below_scan = exact_filesystem_limits();
    below_scan.max_entries_per_directory_scan = EXACT_SCAN_ENTRIES - 1;
    assert!(matches!(
        capture_world_project(&fixture.base, OsStr::new("project-root"), below_scan),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "directory entries per scan",
            actual: EXACT_SCAN_ENTRIES,
            limit,
        }) if limit == EXACT_SCAN_ENTRIES - 1
    ));

    let mut below_total_scan = exact_filesystem_limits();
    below_total_scan.max_total_directory_entries_scanned = EXACT_TOTAL_SCANNED - 1;
    assert!(matches!(
        capture_world_project(
            &fixture.base,
            OsStr::new("project-root"),
            below_total_scan,
        ),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "total directory entries scanned",
            actual: EXACT_TOTAL_SCANNED,
            limit,
        }) if limit == EXACT_TOTAL_SCANNED - 1
    ));

    let largest = fixture
        .documents
        .values()
        .map(Vec::len)
        .max()
        .expect("documents");
    let total: usize = fixture.documents.values().map(Vec::len).sum();
    let mut exact_bytes = exact_filesystem_limits();
    exact_bytes.project.max_document_bytes = largest;
    exact_bytes.project.max_total_bytes = total;
    capture_world_project(&fixture.base, OsStr::new("project-root"), exact_bytes)
        .expect("exact measured byte limits");

    let mut below_document = exact_bytes;
    below_document.project.max_document_bytes = largest - 1;
    assert!(matches!(
        capture_world_project(
            &fixture.base,
            OsStr::new("project-root"),
            below_document,
        ),
        Err(ProjectFilesystemError::Project(ProjectError::LimitExceeded {
            resource: "project document bytes",
            actual,
            limit,
        })) if actual == largest && limit == largest - 1
    ));

    let mut below_total = exact_bytes;
    below_total.project.max_total_bytes = total - 1;
    assert!(matches!(
        capture_world_project(&fixture.base, OsStr::new("project-root"), below_total),
        Err(ProjectFilesystemError::Project(ProjectError::LimitExceeded {
            resource: "project total bytes",
            actual,
            limit,
        })) if actual == total && limit == total - 1
    ));
}

#[test]
fn zero_filesystem_limits_fail_before_opening_the_parent() {
    for limit in 0..2 {
        let mut limits = filesystem_limits();
        if limit == 0 {
            limits.max_entries_per_directory_scan = 0;
        } else {
            limits.max_total_directory_entries_scanned = 0;
        }
        assert!(matches!(
            capture_world_project(
                Path::new("/definitely/not/an/oteryn/project/parent"),
                OsStr::new("project-root"),
                limits,
            ),
            Err(ProjectFilesystemError::InvalidLimit(_))
        ));
    }
}
