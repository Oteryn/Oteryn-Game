#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, symlink};
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
                semantics: Default::default(),
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

    fn internal(&self, suffix: &str) -> PathBuf {
        let matches: Vec<_> = fs::read_dir(&self.base)
            .expect("enumerate publication parent")
            .map(|entry| entry.expect("publication parent entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.ends_with(suffix))
            })
            .collect();
        assert_eq!(matches.len(), 1, "one internal {suffix} entry");
        matches.into_iter().next().expect("internal path")
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
fn committed_recovery_revalidates_every_canonical_document() {
    let fixture = Fixture::new();
    fixture.publish(&documents("mutated-child-r1"));
    let child = fixture.base.join("project-root/records/reference.json");
    let before = fs::metadata(&child).expect("inspect canonical child before mutation");
    let mut bytes = fs::read(&child).expect("read canonical child");
    let marker = b"publication-proof";
    let offset = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("canonical child contains proof identity");
    bytes[offset] = b'P';
    fs::write(&child, &bytes).expect("mutate canonical child in place");
    let after = fs::metadata(&child).expect("inspect canonical child after mutation");
    assert_eq!(before.ino(), after.ino(), "mutation retains child identity");
    assert_eq!(before.len(), after.len(), "mutation retains child length");

    assert!(matches!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::Project(
            ProjectError::DigestMismatch(_)
        ))
    ));
    assert!(matches!(
        capture_world_project(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::Project(
            ProjectError::DigestMismatch(_)
        ))
    ));
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

#[test]
fn partially_removed_backup_restarts_only_from_the_durable_subset() {
    let fixture = Fixture::new();
    fixture.publish(&documents("partial-cleanup-r1"));
    fixture.publish(&documents("partial-cleanup-r2"));
    let backup = fixture.internal(".previous");
    fs::remove_file(backup.join("metadata/author.json")).expect("simulate one durable deletion");

    assert_eq!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        )
        .expect("verify partial backup subset"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
    assert_eq!(
        fixture.publish(&documents("partial-cleanup-r3")),
        ProjectPublicationOutcome::ReplacementRevisionPublished
    );
}

#[test]
fn unplanned_backup_addition_and_root_substitution_fail_without_changing_live() {
    let addition = Fixture::new();
    let live = documents("addition-live-r2");
    addition.publish(&documents("addition-old-r1"));
    addition.publish(&live);
    let backup = addition.internal(".previous");
    fs::remove_file(backup.join("metadata/author.json"))
        .expect("simulate partial authorized cleanup before addition");
    fs::write(backup.join("unmanaged.txt"), b"must survive").expect("add unplanned backup entry");
    assert!(matches!(
        recover_world_project_publication(
            &addition.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::PublicationConflict(_))
    ));
    assert_eq!(
        addition.capture(),
        live.clone()
            .into_snapshot(project_limits())
            .expect("live snapshot")
            .parse(project_limits())
            .expect("live project")
    );
    assert_eq!(
        fs::read(backup.join("unmanaged.txt")).expect("unplanned entry retained"),
        b"must survive"
    );

    let substitution = Fixture::new();
    substitution.publish(&documents("root-substitution-r1"));
    substitution.publish(&documents("root-substitution-r2"));
    let backup = substitution.internal(".previous");
    let displaced = substitution.base.join("displaced-previous");
    fs::rename(&backup, &displaced).expect("move authorized backup identity");
    fs::create_dir(&backup).expect("substitute backup root identity");
    assert!(matches!(
        recover_world_project_publication(
            &substitution.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::PublicationConflict(_))
    ));
}

#[test]
fn cleanup_rejects_child_identity_links_and_special_files() {
    for case in ["identity", "symlink", "hardlink", "fifo"] {
        let fixture = Fixture::new();
        fixture.publish(&documents(&format!("{case}-old-r1")));
        fixture.publish(&documents(&format!("{case}-live-r2")));
        let backup = fixture.internal(".previous");
        let target = backup.join("records/reference.json");
        let original = fs::read(&target).expect("read planned backup file");
        match case {
            "identity" => {
                let replacement = backup.join("records/replacement.tmp");
                fs::write(&replacement, original).expect("write replacement identity");
                fs::rename(replacement, &target)
                    .expect("substitute same bytes with a different identity");
            }
            "symlink" => {
                let outside = fixture.base.join("outside-symlink-target");
                fs::write(&outside, original).expect("write outside symlink target");
                fs::remove_file(&target).expect("remove planned file");
                symlink(outside, &target).expect("substitute source symlink");
            }
            "hardlink" => {
                let outside = fixture.base.join("outside-hardlink");
                fs::hard_link(&target, outside).expect("increase planned file link count");
            }
            "fifo" => {
                fs::remove_file(&target).expect("remove planned file");
                let records =
                    fs::File::open(backup.join("records")).expect("open backup records directory");
                rustix::fs::mkfifoat(
                    &records,
                    "reference.json",
                    rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
                )
                .expect("substitute FIFO");
            }
            _ => panic!("unexpected case"),
        }
        let result = recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        );
        assert!(
            matches!(result, Err(ProjectFilesystemError::PublicationConflict(_))),
            "{case} substitution must conflict: {result:?}"
        );
    }
}

#[test]
fn journal_tamper_conflicts_but_a_torn_final_record_is_ignored() {
    let tamper = Fixture::new();
    let live = documents("journal-tamper-r1");
    tamper.publish(&live);
    let journal = tamper.internal(".journal");
    let mut bytes = fs::read(&journal).expect("read journal");
    bytes[0] ^= 1;
    fs::write(&journal, bytes).expect("tamper journal in place");
    assert!(matches!(
        recover_world_project_publication(
            &tamper.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::PublicationConflict(_))
    ));
    assert_eq!(
        tamper.capture(),
        live.into_snapshot(project_limits())
            .expect("tamper live snapshot")
            .parse(project_limits())
            .expect("tamper live project")
    );

    let torn = Fixture::new();
    torn.publish(&documents("journal-torn-r1"));
    let journal = torn.internal(".journal");
    fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .expect("open journal for torn suffix")
        .write_all(b"{\"partial\"")
        .expect("append torn suffix");
    assert_eq!(
        recover_world_project_publication(
            &torn.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        )
        .expect("recover torn journal suffix"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
}

#[test]
fn publication_limits_reject_max_plus_one_overflow_and_oversize_before_parent_access() {
    let missing = PathBuf::from("/definitely/not/an/oteryn/project/parent");
    let candidate = documents("resource-r1");

    let mut overflow = filesystem_limits();
    overflow.max_total_directory_entries_scanned = usize::MAX;
    assert!(matches!(
        publish_world_project(&missing, OsStr::new("project-root"), &candidate, overflow,),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "publication journal bytes",
            actual: usize::MAX,
            ..
        })
    ));

    let largest = candidate
        .documents()
        .values()
        .map(Vec::len)
        .max()
        .expect("canonical documents");
    let mut oversize = filesystem_limits();
    oversize.project.max_document_bytes = largest - 1;
    assert!(matches!(
        publish_world_project(&missing, OsStr::new("project-root"), &candidate, oversize,),
        Err(ProjectFilesystemError::Project(
            ProjectError::LimitExceeded {
                resource: "project document bytes",
                ..
            }
        ))
    ));

    let fixture = Fixture::new();
    let mut max_plus_one = filesystem_limits();
    max_plus_one.max_entries_per_directory_scan = 5;
    assert!(matches!(
        publish_world_project(
            &fixture.base,
            OsStr::new("project-root"),
            &candidate,
            max_plus_one,
        ),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "publication directory entries",
            actual: 6,
            limit: 5,
        })
    ));

    let exact = Fixture::new();
    let mut exact_scan = filesystem_limits();
    exact_scan.max_entries_per_directory_scan = 6;
    exact_scan.max_total_directory_entries_scanned = 41;
    publish_world_project(
        &exact.base,
        OsStr::new("project-root"),
        &candidate,
        exact_scan,
    )
    .expect("exact measured publication scan limits");

    let below = Fixture::new();
    let mut below_scan = exact_scan;
    below_scan.max_total_directory_entries_scanned = 40;
    assert!(matches!(
        publish_world_project(
            &below.base,
            OsStr::new("project-root"),
            &candidate,
            below_scan,
        ),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "total directory entries scanned",
            actual: 41,
            limit: 40,
        })
    ));
}

#[test]
fn oversized_sparse_journal_is_rejected_from_metadata_before_read_allocation() {
    let fixture = Fixture::new();
    fixture.publish(&documents("sparse-journal-r1"));
    let journal = fixture.internal(".journal");
    fs::OpenOptions::new()
        .write(true)
        .open(journal)
        .expect("open journal for sparse extension")
        .set_len(10_000_000)
        .expect("extend sparse journal");
    assert!(matches!(
        recover_world_project_publication(
            &fixture.base,
            OsStr::new("project-root"),
            filesystem_limits(),
        ),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "publication journal bytes",
            ..
        })
    ));
}
