#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

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

fn documents(revision: &str) -> CanonicalProjectDocuments {
    CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: revision.to_owned(),
            package_key: "oteryn:content.publication-proof".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "license:project-owned-v1".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-07-28".to_owned(),
            records: vec![ProjectReferenceRecord::Item {
                identity: DefinitionIdentityDocument {
                    family: "Item".to_owned(),
                    key: "oteryn:reference.item.publication-proof".to_owned(),
                    revision: "definition-r1".to_owned(),
                },
                client_projection: ProjectionDocument::ClientSafe,
                materializable: true,
                stack_class: ItemStackDocument::StackCapable,
            }],
            imports: Vec::new(),
            metadata: Vec::new(),
        },
        project_limits(),
    )
    .expect("canonical publication project")
}

struct Fixture {
    base: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "oteryn-project-publication-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&base).expect("create isolated publication parent");
        Self { base }
    }

    fn publish(&self, documents: &CanonicalProjectDocuments) -> ProjectPublicationOutcome {
        publish_world_project(
            &self.base,
            OsStr::new("project-root"),
            documents,
            filesystem_limits(),
        )
        .expect("publish canonical project")
    }

    fn capture(&self) -> WorldProject {
        capture_world_project(&self.base, OsStr::new("project-root"), filesystem_limits())
            .expect("capture published project")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn initial_and_replacement_publication_remain_whole_and_recoverable() {
    let fixture = Fixture::new();
    let initial = documents("project-r1");
    let replacement = documents("project-r2");

    assert_eq!(
        fixture.publish(&initial),
        ProjectPublicationOutcome::InitialRevisionPublished
    );
    assert_eq!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        )
        .expect("verify initial publication"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
    assert_eq!(
        fixture.publish(&replacement),
        ProjectPublicationOutcome::ReplacementRevisionPublished
    );

    let expected = replacement
        .clone()
        .into_snapshot(project_limits())
        .expect("replacement snapshot")
        .parse(project_limits())
        .expect("replacement project");
    assert_eq!(fixture.capture(), expected);
    assert_eq!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        )
        .expect("repeat committed recovery"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
}

#[test]
fn same_content_replacement_uses_identity_roles_instead_of_digest_guessing() {
    let fixture = Fixture::new();
    let revision = documents("project-same-content");
    assert_eq!(
        fixture.publish(&revision),
        ProjectPublicationOutcome::InitialRevisionPublished
    );
    assert_eq!(
        fixture.publish(&revision),
        ProjectPublicationOutcome::ReplacementRevisionPublished
    );
    assert_eq!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        )
        .expect("same-content recovery"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
    let expected = revision
        .into_snapshot(project_limits())
        .expect("same-content snapshot")
        .parse(project_limits())
        .expect("same-content project");
    assert_eq!(fixture.capture(), expected);
}

#[test]
fn invalid_limits_fail_before_parent_access() {
    let mut limits = filesystem_limits();
    limits.max_total_directory_entries_scanned = 0;
    let error = publish_world_project(
        PathBuf::from("/definitely/not/an/oteryn/project/parent").as_path(),
        OsStr::new("project-root"),
        &documents("project-r1"),
        limits,
    )
    .expect_err("zero limit must fail before parent access");
    assert!(matches!(error, ProjectFilesystemError::InvalidLimit(_)));
}
