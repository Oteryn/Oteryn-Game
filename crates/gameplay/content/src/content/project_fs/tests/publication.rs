use super::*;
use crate::content::{
    DefinitionIdentityDocument, ItemStackDocument, ProjectDraft, ProjectReferenceRecord,
    ProjectionDocument,
};
use rustix::fs::{Mode, mkfifoat};
use std::fs;
use std::os::unix::fs::MetadataExt as StdMetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn evidence_limits() -> crate::content::ProjectEvidenceLimits {
    crate::content::ProjectEvidenceLimits {
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

fn publication_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: evidence_limits(),
        max_entries_per_directory_scan: 64,
        max_total_directory_entries_scanned: 1_024,
    }
}

fn canonical_documents(revision: &str) -> CanonicalProjectDocuments {
    CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: revision.to_owned(),
            package_key: "oteryn:content.publication-unit-proof".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "license:project-owned-v1".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-09-27".to_owned(),
            records: vec![ProjectReferenceRecord::Item {
                identity: DefinitionIdentityDocument {
                    family: "Item".to_owned(),
                    key: "oteryn:reference.item.publication-unit-proof".to_owned(),
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
        evidence_limits(),
    )
    .expect("canonical unit publication project")
}

struct PublicationFixture {
    base: std::path::PathBuf,
}

impl PublicationFixture {
    fn new() -> Self {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "oteryn-project-publication-unit-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&base).expect("create publication unit parent");
        Self { base }
    }

    fn publish(&self, documents: &CanonicalProjectDocuments) {
        publish(
            &self.base,
            OsStr::new("project-root"),
            documents,
            publication_limits(),
        )
        .expect("publish unit project");
    }

    fn interrupt(
        &self,
        documents: &CanonicalProjectDocuments,
        target: PublicationCheckpoint,
    ) {
        let mut fired = false;
        let result = publish_with_observer(
            &self.base,
            OsStr::new("project-root"),
            documents,
            publication_limits(),
            &mut |checkpoint| {
                if checkpoint == target && !fired {
                    fired = true;
                    Err(conflict("injected publication interruption"))
                } else {
                    Ok(())
                }
            },
        );
        assert!(fired, "requested checkpoint was reached");
        assert!(matches!(
            result,
            Err(ProjectFilesystemError::PublicationConflict(
                "injected publication interruption"
            ))
        ));
    }

    fn recover(&self) -> Result<ProjectRecoveryOutcome, ProjectFilesystemError> {
        recover(&self.base, OsStr::new("project-root"), publication_limits())
    }

    fn captured(&self) -> WorldProject {
        capture(&self.base, OsStr::new("project-root"), publication_limits())
            .expect("capture unit project")
    }
}

impl Drop for PublicationFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn expected(documents: CanonicalProjectDocuments) -> WorldProject {
    documents
        .into_snapshot(evidence_limits())
        .expect("unit snapshot")
        .parse(evidence_limits())
        .expect("unit project")
}

fn corrupt_reference_document_in_place(root: &Path) {
    let child = root.join("records/reference.json");
    let before = fs::metadata(&child).expect("inspect reference document before mutation");
    let mut bytes = fs::read(&child).expect("read reference document");
    let marker = b"publication-unit-proof";
    let offset = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("reference document contains unit proof identity");
    bytes[offset] = b'P';
    fs::write(&child, &bytes).expect("mutate reference document in place");
    let after = fs::metadata(&child).expect("inspect reference document after mutation");
    assert_eq!(
        StdMetadataExt::ino(&before),
        StdMetadataExt::ino(&after),
        "mutation retains the planned file identity"
    );
    assert_eq!(before.len(), after.len(), "mutation retains byte length");
}

#[test]
fn initial_install_crash_is_completed_from_identity_topology() {
    let fixture = PublicationFixture::new();
    let replacement = canonical_documents("initial-crash-r1");
    fixture.interrupt(
        &replacement,
        PublicationCheckpoint::RootInstalledBeforePhase,
    );
    assert_eq!(
        fixture.recover().expect("recover initial install"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(fixture.captured(), expected(replacement));
    assert_eq!(
        fixture.recover().expect("repeat initial recovery"),
        ProjectRecoveryOutcome::CommittedRevisionVerified
    );
}

#[test]
fn replacement_exchange_crash_completes_new_root_without_mixing() {
    let fixture = PublicationFixture::new();
    fixture.publish(&canonical_documents("exchange-old-r1"));
    let replacement = canonical_documents("exchange-new-r2");
    fixture.interrupt(
        &replacement,
        PublicationCheckpoint::RootInstalledBeforePhase,
    );
    assert_eq!(
        fixture.recover().expect("recover exchanged root"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(fixture.captured(), expected(replacement));
}

#[test]
fn same_content_backup_install_crash_is_committed_by_identity() {
    let fixture = PublicationFixture::new();
    let same = canonical_documents("same-content-crash");
    fixture.publish(&same);
    fixture.interrupt(&same, PublicationCheckpoint::BackupInstalledBeforePhase);
    assert_eq!(
        fixture.recover().expect("recover same-content backup"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(fixture.captured(), expected(same));
}

#[test]
fn durable_stage_ready_without_install_rolls_back_to_previous() {
    let fixture = PublicationFixture::new();
    let previous = canonical_documents("stage-ready-old-r1");
    fixture.publish(&previous);
    fixture.interrupt(
        &canonical_documents("stage-ready-new-r2"),
        PublicationCheckpoint::StageReadyDurable,
    );
    assert_eq!(
        fixture.recover().expect("roll back durable stage"),
        ProjectRecoveryOutcome::PreviousRevisionPreserved
    );
    assert_eq!(fixture.captured(), expected(previous));
}

#[test]
fn creation_before_durable_plan_extension_is_an_explicit_conflict() {
    for checkpoint in [
        PublicationCheckpoint::StageRootCreatedBeforePlan,
        PublicationCheckpoint::StageEntryCreatedBeforePlan,
    ] {
        let fixture = PublicationFixture::new();
        let previous = canonical_documents("unplanned-old-r1");
        fixture.publish(&previous);
        fixture.interrupt(&canonical_documents("unplanned-new-r2"), checkpoint);
        assert!(matches!(
            fixture.recover(),
            Err(ProjectFilesystemError::PublicationConflict(_))
        ));
        assert_eq!(fixture.captured(), expected(previous));
    }
}

#[test]
fn recovery_syncs_observed_namespace_before_advancing_the_journal() {
    let initial = PublicationFixture::new();
    let initial_replacement = canonical_documents("unsynced-initial-r1");
    initial.interrupt(
        &initial_replacement,
        PublicationCheckpoint::RootInstalledBeforeParentSync,
    );
    assert_eq!(
        initial.recover().expect("recover unsynced initial install"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(initial.captured(), expected(initial_replacement));

    let exchange = PublicationFixture::new();
    exchange.publish(&canonical_documents("unsynced-exchange-old-r1"));
    let exchange_replacement = canonical_documents("unsynced-exchange-new-r2");
    exchange.interrupt(
        &exchange_replacement,
        PublicationCheckpoint::RootInstalledBeforeParentSync,
    );
    assert_eq!(
        exchange
            .recover()
            .expect("recover unsynced replacement exchange"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(exchange.captured(), expected(exchange_replacement));

    let backup = PublicationFixture::new();
    backup.publish(&canonical_documents("unsynced-backup-old-r1"));
    let backup_replacement = canonical_documents("unsynced-backup-new-r2");
    backup.interrupt(
        &backup_replacement,
        PublicationCheckpoint::BackupInstalledBeforeParentSync,
    );
    assert_eq!(
        backup
            .recover()
            .expect("recover unsynced previous-backup install"),
        ProjectRecoveryOutcome::ReplacementRevisionRecovered
    );
    assert_eq!(backup.captured(), expected(backup_replacement));
}

#[test]
fn canonical_document_limits_are_checked_before_the_clone_boundary() {
    let documents = canonical_documents("preclone-resource-r1");
    let largest = documents
        .documents()
        .values()
        .map(Vec::len)
        .max()
        .expect("canonical documents");
    let total: usize = documents.documents().values().map(Vec::len).sum();
    let mut count_limited = publication_limits();
    count_limited.project.max_documents = documents.documents().len() - 1;
    let mut document_limited = publication_limits();
    document_limited.project.max_document_bytes = largest - 1;
    let mut total_limited = publication_limits();
    total_limited.project.max_total_bytes = total - 1;

    for limits in [count_limited, document_limited, total_limited] {
        let mut clone_boundary_reached = false;
        let result = publish_with_observer(
            Path::new("/ambient/parent/must/not/be/opened"),
            OsStr::new("project-root"),
            &documents,
            limits,
            &mut |checkpoint| {
                if checkpoint == PublicationCheckpoint::DocumentsAdmittedBeforeClone {
                    clone_boundary_reached = true;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(ProjectFilesystemError::Project(
                crate::content::ProjectError::LimitExceeded { .. }
            ))
        ));
        assert!(
            !clone_boundary_reached,
            "oversized canonical documents reached the clone boundary"
        );
    }
}

#[test]
fn concurrent_publication_fails_before_interpreting_recovery_state() {
    let fixture = PublicationFixture::new();
    fixture.publish(&canonical_documents("locked-r1"));
    let parent = Dir::open_ambient_dir(&fixture.base, ambient_authority())
        .expect("open parent for competing lock");
    let _held = lock_parent(&parent).expect("hold publication lock");
    assert!(matches!(
        fixture.recover(),
        Err(ProjectFilesystemError::Io {
            operation: "acquire exclusive project publication lock",
            ..
        })
    ));
}

#[test]
fn publication_validation_and_commit_stay_on_the_pinned_parent() {
    let fixture = PublicationFixture::new();
    fixture.publish(&canonical_documents("pinned-parent-old-r1"));
    let replacement = canonical_documents("pinned-parent-new-r2");
    let substitute = canonical_documents("ambient-substitute-r1");
    let displaced = fixture.base.with_file_name(format!(
        "{}-displaced",
        fixture
            .base
            .file_name()
            .expect("publication fixture basename")
            .to_string_lossy()
    ));
    let mut parent_replaced = false;

    let outcome = publish_with_observer(
        &fixture.base,
        OsStr::new("project-root"),
        &replacement,
        publication_limits(),
        &mut |checkpoint| {
            if checkpoint == PublicationCheckpoint::ParentPinned && !parent_replaced {
                parent_replaced = true;
                fs::rename(&fixture.base, &displaced)
                    .expect("move the pinned parent away from its ambient path");
                fs::create_dir(&fixture.base).expect("create ambient parent substitute");
                publish(
                    &fixture.base,
                    OsStr::new("project-root"),
                    &substitute,
                    publication_limits(),
                )
                .expect("publish an unrelated project in the substitute parent");
            }
            Ok(())
        },
    )
    .expect("publish through the originally pinned parent");

    assert!(parent_replaced, "parent replacement checkpoint was reached");
    assert_eq!(
        outcome,
        ProjectPublicationOutcome::ReplacementRevisionPublished
    );
    assert_eq!(
        capture(&displaced, OsStr::new("project-root"), publication_limits(),)
            .expect("capture replacement from the pinned parent"),
        expected(replacement)
    );
    assert_eq!(
        capture(
            &fixture.base,
            OsStr::new("project-root"),
            publication_limits(),
        )
        .expect("capture unrelated ambient substitute"),
        expected(substitute)
    );
    fs::remove_dir_all(displaced).expect("remove displaced publication parent");
}

#[test]
fn committed_recovery_binds_captured_children_across_exchange_boundaries() {
    for checkpoint in [
        RoleVerificationCheckpoint::IdentityPlanVerified,
        RoleVerificationCheckpoint::CanonicalCaptureComplete,
    ] {
        let fixture = PublicationFixture::new();
        let donor = PublicationFixture::new();
        let documents = canonical_documents("role-exchange-r1");
        fixture.publish(&documents);
        donor.publish(&documents);

        if checkpoint == RoleVerificationCheckpoint::IdentityPlanVerified {
            corrupt_reference_document_in_place(&fixture.base.join("project-root"));
        } else {
            corrupt_reference_document_in_place(&donor.base.join("project-root"));
        }

        let parent = Dir::open_ambient_dir(&fixture.base, ambient_authority())
            .expect("open role-exchange parent");
        let _held = lock_parent(&parent).expect("lock role-exchange parent");
        let root = parent
            .open_dir_nofollow("project-root")
            .expect("open planned project root");
        let records = root
            .open_dir_nofollow("records")
            .expect("open planned records directory");
        let donor_parent = Dir::open_ambient_dir(&donor.base, ambient_authority())
            .expect("open alternate project parent");
        let donor_root = donor_parent
            .open_dir_nofollow("project-root")
            .expect("open alternate project root");
        let donor_records = donor_root
            .open_dir_nofollow("records")
            .expect("open alternate records directory");
        let names = internal_names(OsStr::new("project-root"))
            .expect("derive role-exchange internal names");
        let max_journal =
            journal_byte_limit(publication_limits()).expect("bound recovery journal");
        let mut exchanged = false;
        let result = recover_in_parent_with_observer(
            &parent,
            OsStr::new("project-root"),
            &names,
            publication_limits(),
            max_journal,
            &mut |observed| {
                if observed == checkpoint && !exchanged {
                    rustix::fs::renameat_with(
                        &records,
                        "reference.json",
                        &donor_records,
                        "reference.json",
                        RenameFlags::EXCHANGE,
                    )
                    .map_err(|source| {
                        io_error(
                            "exchange canonical child during role verification",
                            "records/reference.json",
                            source.into(),
                        )
                    })?;
                    exchanged = true;
                }
                Ok(())
            },
        );
        assert!(exchanged, "targeted role-verification boundary was reached");
        assert!(
            result.is_err(),
            "root exchange must not produce a committed recovery: {result:?}"
        );

        if checkpoint == RoleVerificationCheckpoint::IdentityPlanVerified {
            rustix::fs::renameat_with(
                &records,
                "reference.json",
                &donor_records,
                "reference.json",
                RenameFlags::EXCHANGE,
            )
            .expect("restore corrupted planned child to the live tree");
        }
        assert!(
            capture_in_parent(
                &parent,
                OsStr::new("project-root"),
                publication_limits(),
                ProjectAdmission::Ordinary,
            )
            .is_err(),
            "the corrupt live tree remains unverifiable"
        );
        assert!(
            recover_in_parent(
                &parent,
                OsStr::new("project-root"),
                &names,
                publication_limits(),
                max_journal,
            )
            .is_err(),
            "recovery must not certify the corrupt live tree"
        );
    }
}

#[test]
fn final_open_flags_do_not_wait_for_a_fifo_writer() {
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "oteryn-project-fifo-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&path).expect("create FIFO probe directory");
    let std_dir = fs::File::open(&path).expect("open FIFO probe directory");
    mkfifoat(&std_dir, "source.json", Mode::RUSR | Mode::WUSR)
        .expect("create unpaired FIFO");
    let dir = Dir::open_ambient_dir(&path, ambient_authority())
        .expect("open FIFO probe directory capability");
    let started = Instant::now();
    let file = dir
        .open_with("source.json", &safe_read_options())
        .expect("nonblocking nofollow FIFO open");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!file.metadata().expect("FIFO metadata").is_file());
    drop(file);
    drop(dir);
    fs::remove_dir_all(path).expect("remove FIFO probe directory");
}

#[test]
fn scan_budget_arithmetic_overflow_fails_closed() {
    let mut total_overflow = ScanBudget {
        per_scan: usize::MAX,
        total_limit: usize::MAX,
        total: usize::MAX,
    };
    let mut in_scan = 0;
    assert!(matches!(
        total_overflow.admit(&mut in_scan),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "total directory entries scanned",
            actual: usize::MAX,
            limit: usize::MAX,
        })
    ));

    let mut per_scan_overflow = ScanBudget {
        per_scan: usize::MAX,
        total_limit: usize::MAX,
        total: 0,
    };
    let mut in_scan = usize::MAX;
    assert!(matches!(
        per_scan_overflow.admit(&mut in_scan),
        Err(ProjectFilesystemError::LimitExceeded {
            resource: "directory entries per scan",
            actual: usize::MAX,
            limit: usize::MAX,
        })
    ));
}
