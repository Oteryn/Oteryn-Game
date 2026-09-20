//! Secure Linux filesystem capture and publication for canonical editable World projects.
//!
//! Publication is a whole-root transaction. A durable, identity-bound journal authorizes every
//! entry which recovery may remove, and an atomic root rename/exchange keeps readers on either the
//! previous complete root or the replacement complete root. Non-Linux targets fail before
//! filesystem access until an equivalent opened-handle identity API is available there.

use super::{CanonicalProjectDocuments, ProjectError, ProjectEvidenceLimits, WorldProject};
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::Path;

/// Finite filesystem-capture limits for the first non-production evidence corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectFilesystemLimits {
    pub project: ProjectEvidenceLimits,
    pub max_entries_per_directory_scan: usize,
    pub max_total_directory_entries_scanned: usize,
}

impl ProjectFilesystemLimits {
    fn validate(self) -> Result<Self, ProjectFilesystemError> {
        self.project.validate()?;
        for (name, value) in [
            (
                "directory entries per scan",
                self.max_entries_per_directory_scan,
            ),
            (
                "total directory entries scanned",
                self.max_total_directory_entries_scanned,
            ),
        ] {
            if value == 0 {
                return Err(ProjectFilesystemError::InvalidLimit(name));
            }
        }
        Ok(self)
    }
}

#[derive(Debug)]
pub enum ProjectFilesystemError {
    UnsupportedPlatform,
    InvalidLimit(&'static str),
    LimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    UnsafeEntry {
        locator: String,
        reason: &'static str,
    },
    PublicationConflict(&'static str),
    Io {
        operation: &'static str,
        locator: String,
        source: io::Error,
    },
    Project(ProjectError),
}

impl Display for ProjectFilesystemError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("project filesystem capture is unsupported on this platform")
            }
            Self::InvalidLimit(name) => write!(formatter, "invalid zero filesystem limit: {name}"),
            Self::LimitExceeded {
                resource,
                actual,
                limit,
            } => write!(
                formatter,
                "{resource} exceeds filesystem limit: {actual} > {limit}"
            ),
            Self::UnsafeEntry { locator, reason } => {
                write!(formatter, "unsafe project entry {locator}: {reason}")
            }
            Self::PublicationConflict(reason) => {
                write!(formatter, "project publication recovery conflict: {reason}")
            }
            Self::Io {
                operation,
                locator,
                source,
            } => write!(
                formatter,
                "project filesystem {operation} failed for {locator}: {source}"
            ),
            Self::Project(error) => Display::fmt(error, formatter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPublicationOutcome {
    InitialRevisionPublished,
    ReplacementRevisionPublished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRecoveryOutcome {
    NoTransaction,
    PreviousRevisionPreserved,
    ReplacementRevisionRecovered,
    CommittedRevisionVerified,
}

impl std::error::Error for ProjectFilesystemError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Project(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ProjectError> for ProjectFilesystemError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error)
    }
}

/// Capture and parse one canonical project rooted at an exact child of `ambient_parent`.
///
/// Linux is the only supported platform in this increment. Other targets return
/// [`ProjectFilesystemError::UnsupportedPlatform`] before inspecting either path argument.
pub fn capture_world_project(
    ambient_parent: &Path,
    root_basename: &OsStr,
    limits: ProjectFilesystemLimits,
) -> Result<WorldProject, ProjectFilesystemError> {
    #[cfg(target_os = "linux")]
    {
        linux::capture(ambient_parent, root_basename, limits)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (ambient_parent, root_basename, limits);
        Err(ProjectFilesystemError::UnsupportedPlatform)
    }
}

/// Durably publish one complete canonical project revision.
///
/// The caller-selected limits remain evidence limits rather than production maxima. On Linux the
/// replacement is staged, validated through the existing project parser, synchronized, and then
/// installed with one atomic root operation. Other targets return `UnsupportedPlatform` before
/// inspecting either path argument or the documents.
pub fn publish_world_project(
    ambient_parent: &Path,
    root_basename: &OsStr,
    documents: &CanonicalProjectDocuments,
    limits: ProjectFilesystemLimits,
) -> Result<ProjectPublicationOutcome, ProjectFilesystemError> {
    #[cfg(target_os = "linux")]
    {
        linux::publish(ambient_parent, root_basename, documents, limits)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (ambient_parent, root_basename, documents, limits);
        Err(ProjectFilesystemError::UnsupportedPlatform)
    }
}

/// Recover or verify an interrupted publication without starting a new save.
///
/// A successfully committed previous root remains installed as the pre-migration backup. A later
/// call to [`publish_world_project`] retires that backup only from its durable identity plan before
/// beginning the next transaction.
pub fn recover_world_project_publication(
    ambient_parent: &Path,
    root_basename: &OsStr,
    limits: ProjectFilesystemLimits,
) -> Result<ProjectRecoveryOutcome, ProjectFilesystemError> {
    #[cfg(target_os = "linux")]
    {
        linux::recover(ambient_parent, root_basename, limits)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (ambient_parent, root_basename, limits);
        Err(ProjectFilesystemError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{
        ProjectFilesystemError, ProjectFilesystemLimits, ProjectPublicationOutcome,
        ProjectRecoveryOutcome,
    };
    use crate::content::project::{
        LOCK_LOCATOR, MANIFEST_LOCATOR, PROJECT_LOCATOR, ProjectCapturePlan,
    };
    use crate::content::{
        CanonicalProjectDocuments, ProjectSnapshot, WorldProject, world_project_sha256,
    };
    use cap_fs_ext::{
        DirEntryExt, DirExt, FollowSymlinks, MetadataExt as IdentityMetadataExt,
        OpenOptionsFollowExt, OpenOptionsSyncExt,
    };
    use cap_std::ambient_authority;
    use cap_std::fs::{Dir, File, Metadata, OpenOptions};
    use rustix::fd::OwnedFd;
    use rustix::fs::{FlockOperation, Mode, OFlags, RenameFlags};
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Write};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::{Component, Path};

    const JOURNAL_SCHEMA: &str = "OTERYN_WORLD_PROJECT_PUBLICATION_JOURNAL/v2";
    const LINUX_NAME_MAX: usize = 255;
    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FileIdentity {
        device: u64,
        inode: u64,
    }

    impl FileIdentity {
        fn from_metadata(metadata: &Metadata) -> Self {
            Self {
                device: IdentityMetadataExt::dev(metadata),
                inode: IdentityMetadataExt::ino(metadata),
            }
        }
    }

    struct ScanBudget {
        per_scan: usize,
        total_limit: usize,
        total: usize,
    }

    impl ScanBudget {
        fn new(limits: ProjectFilesystemLimits) -> Self {
            Self {
                per_scan: limits.max_entries_per_directory_scan,
                total_limit: limits.max_total_directory_entries_scanned,
                total: 0,
            }
        }

        fn admit(&mut self, in_scan: &mut usize) -> Result<(), ProjectFilesystemError> {
            let next_scan =
                in_scan
                    .checked_add(1)
                    .ok_or(ProjectFilesystemError::LimitExceeded {
                        resource: "directory entries per scan",
                        actual: usize::MAX,
                        limit: self.per_scan,
                    })?;
            if next_scan > self.per_scan {
                return Err(ProjectFilesystemError::LimitExceeded {
                    resource: "directory entries per scan",
                    actual: next_scan,
                    limit: self.per_scan,
                });
            }
            let next_total =
                self.total
                    .checked_add(1)
                    .ok_or(ProjectFilesystemError::LimitExceeded {
                        resource: "total directory entries scanned",
                        actual: usize::MAX,
                        limit: self.total_limit,
                    })?;
            if next_total > self.total_limit {
                return Err(ProjectFilesystemError::LimitExceeded {
                    resource: "total directory entries scanned",
                    actual: next_total,
                    limit: self.total_limit,
                });
            }
            *in_scan = next_scan;
            self.total = next_total;
            Ok(())
        }
    }

    struct PinnedFile {
        file: File,
        metadata: Metadata,
        identity: FileIdentity,
        locator: String,
        length: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum PlannedKind {
        Directory,
        RegularFile,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PlannedEntry {
        parent: FileIdentity,
        name_hex: String,
        kind: PlannedKind,
        identity: FileIdentity,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TreePlan {
        root: FileIdentity,
        entries: Vec<PlannedEntry>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "record", rename_all = "snake_case", deny_unknown_fields)]
    enum JournalRecord {
        Header {
            schema: String,
            root_name_hex: String,
            journal_identity: FileIdentity,
        },
        PreviousEntry {
            entry: PlannedEntry,
        },
        PreviousReady {
            previous_root: Option<FileIdentity>,
            previous_digest: Option<String>,
            replacement_digest: String,
        },
        StageRoot {
            identity: FileIdentity,
        },
        StageEntry {
            entry: PlannedEntry,
        },
        StageReady,
        Installed,
        BackupInstalled,
        Committed,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct JournalLine {
        previous_sha256: String,
        payload: JournalRecord,
        sha256: String,
    }

    #[derive(Debug)]
    struct JournalState {
        journal_identity: FileIdentity,
        previous: Option<TreePlan>,
        previous_digest: Option<String>,
        replacement_digest: Option<String>,
        previous_ready: bool,
        stage_root: Option<FileIdentity>,
        stage_entries: Vec<PlannedEntry>,
        stage_ready: bool,
        installed: bool,
        backup_installed: bool,
        committed: bool,
        last_digest: String,
        complete_length: u64,
    }

    struct JournalWriter {
        file: File,
        last_digest: String,
        length: usize,
        max_length: usize,
    }

    #[derive(Debug)]
    struct InternalNames {
        stage: OsString,
        backup: OsString,
        journal: OsString,
    }

    #[derive(Debug)]
    struct CurrentEntry {
        name: OsString,
        kind: PlannedKind,
        identity: FileIdentity,
        children: Vec<CurrentEntry>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PublicationCheckpoint {
        PlanDurable,
        StageRootCreatedBeforePlan,
        StageEntryCreatedBeforePlan,
        StageReadyDurable,
        RootInstalledBeforePhase,
        BackupInstalledBeforePhase,
    }

    pub(super) fn capture(
        ambient_parent: &Path,
        root_basename: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<WorldProject, ProjectFilesystemError> {
        let limits = limits.validate()?;
        validate_root_basename(root_basename)?;
        let mut scans = ScanBudget::new(limits);
        let parent = Dir::open_ambient_dir(ambient_parent, ambient_authority())
            .map_err(|source| io_error("open ambient parent", "<project-parent>", source))?;
        let root_entry =
            exact_entry_metadata(&parent, root_basename, "<project-root>", &mut scans)?;
        let root = parent.open_dir_nofollow(root_basename).map_err(|source| {
            io_error(
                "open root without following links",
                "<project-root>",
                source,
            )
        })?;
        let opened_root = root
            .dir_metadata()
            .map_err(|source| io_error("inspect opened root", "<project-root>", source))?;
        if !opened_root.is_dir()
            || FileIdentity::from_metadata(&root_entry) != FileIdentity::from_metadata(&opened_root)
        {
            return Err(unsafe_entry(
                "<project-root>",
                "root is not the exact opened ordinary directory",
            ));
        }

        let mut identities = BTreeSet::new();
        let mut controls = Vec::with_capacity(3);
        let mut control_total = 0_usize;
        for locator in [PROJECT_LOCATOR, MANIFEST_LOCATOR, LOCK_LOCATOR] {
            let pinned = open_locator(&root, locator, limits, &mut scans, &mut identities)?;
            control_total =
                checked_total(control_total, pinned.length, limits.project.max_total_bytes)?;
            controls.push(pinned);
        }

        let mut control_bytes = BTreeMap::new();
        for pinned in controls {
            let locator = pinned.locator.clone();
            let bytes = consume_pinned(pinned)?;
            control_bytes.insert(locator, bytes);
        }
        let plan = ProjectCapturePlan::from_control_documents(
            &control_bytes[PROJECT_LOCATOR],
            &control_bytes[MANIFEST_LOCATOR],
            &control_bytes[LOCK_LOCATOR],
            limits.project,
        )?;

        let mut documents = control_bytes;
        for expected in plan.documents() {
            let pinned = open_locator(
                &root,
                &expected.locator,
                limits,
                &mut scans,
                &mut identities,
            )?;
            if pinned.length != expected.byte_length {
                return Err(unsafe_entry(
                    &expected.locator,
                    "opened byte length differs from manifest",
                ));
            }
            let bytes = consume_pinned(pinned)?;
            if world_project_sha256(&bytes) != expected.sha256 {
                return Err(ProjectFilesystemError::Project(
                    crate::content::ProjectError::DigestMismatch(expected.locator.clone()),
                ));
            }
            documents.insert(expected.locator.clone(), bytes);
        }

        ProjectSnapshot::new(documents, limits.project)?
            .parse(limits.project)
            .map_err(ProjectFilesystemError::from)
    }

    pub(super) fn publish(
        ambient_parent: &Path,
        root_basename: &OsStr,
        documents: &CanonicalProjectDocuments,
        limits: ProjectFilesystemLimits,
    ) -> Result<ProjectPublicationOutcome, ProjectFilesystemError> {
        publish_with_observer(
            ambient_parent,
            root_basename,
            documents,
            limits,
            &mut |_| Ok(()),
        )
    }

    fn publish_with_observer(
        ambient_parent: &Path,
        root_basename: &OsStr,
        documents: &CanonicalProjectDocuments,
        limits: ProjectFilesystemLimits,
        observer: &mut dyn FnMut(PublicationCheckpoint) -> Result<(), ProjectFilesystemError>,
    ) -> Result<ProjectPublicationOutcome, ProjectFilesystemError> {
        let limits = limits.validate()?;
        validate_root_basename(root_basename)?;
        let desired = ProjectSnapshot::new(documents.documents().clone(), limits.project)?
            .parse(limits.project)?;
        let names = internal_names(root_basename)?;
        let max_journal = journal_byte_limit(limits)?;
        let parent = Dir::open_ambient_dir(ambient_parent, ambient_authority())
            .map_err(|source| io_error("open ambient parent", "<project-parent>", source))?;
        let _publication_lock = lock_parent(&parent)?;

        recover_in_parent(&parent, root_basename, &names, limits, max_journal)?;
        retire_committed_receipt(&parent, root_basename, &names, limits, max_journal)?;
        require_internal_absent(&parent, &names, limits)?;

        let previous_identity = optional_entry_identity(
            &parent,
            root_basename,
            "<project-root>",
            &mut ScanBudget::new(limits),
        )?;
        let (previous, previous_digest) = if previous_identity.is_some() {
            let captured = capture(ambient_parent, root_basename, limits)?;
            let (locators, digest) = existing_locators_and_digest(&parent, root_basename, limits)?;
            let plan = scan_exact_tree(&parent, root_basename, &locators, limits)?;
            if Some(plan.root) != previous_identity {
                return Err(conflict(
                    "current root changed while publication was prepared",
                ));
            }
            let _ = captured;
            (Some(plan), Some(digest))
        } else {
            (None, None)
        };

        let replacement_digest = documents
            .documents()
            .get(PROJECT_LOCATOR)
            .map(|bytes| world_project_sha256(bytes))
            .ok_or_else(|| {
                ProjectFilesystemError::Project(crate::content::ProjectError::MissingDocument(
                    PROJECT_LOCATOR.to_owned(),
                ))
            })?;
        let mut journal =
            JournalWriter::create(&parent, &names.journal, root_basename, max_journal)?;
        if let Some(plan) = &previous {
            for entry in &plan.entries {
                journal.append(&JournalRecord::PreviousEntry {
                    entry: entry.clone(),
                })?;
            }
        }
        journal.append(&JournalRecord::PreviousReady {
            previous_root: previous.as_ref().map(|plan| plan.root),
            previous_digest: previous_digest.clone(),
            replacement_digest,
        })?;
        sync_dir(&parent, "sync parent after durable publication plan")?;
        observer(PublicationCheckpoint::PlanDurable)?;

        let stage_plan = write_stage(
            &parent,
            &names.stage,
            documents,
            limits,
            &mut journal,
            observer,
        )?;
        let staged = capture(ambient_parent, &names.stage, limits)?;
        if staged != desired {
            return Err(conflict(
                "staged project differs from validated replacement",
            ));
        }
        journal.append(&JournalRecord::StageReady)?;
        observer(PublicationCheckpoint::StageReadyDurable)?;

        match previous {
            Some(previous_plan) => {
                verify_tree_complete(&parent, root_basename, &previous_plan, limits)?;
                let expected_previous_digest =
                    existing_root_digest(&parent, root_basename, limits)?;
                if Some(expected_previous_digest.as_str()) != previous_digest.as_deref() {
                    return Err(conflict("previous root changed before atomic exchange"));
                }
                atomic_exchange(&parent, root_basename, &names.stage)?;
                sync_dir(&parent, "sync parent after atomic root exchange")?;
                require_identity(
                    &parent,
                    root_basename,
                    PlannedKind::Directory,
                    stage_plan.root,
                    "<project-root>",
                )?;
                require_identity(
                    &parent,
                    &names.stage,
                    PlannedKind::Directory,
                    previous_plan.root,
                    "<publication-stage>",
                )?;
                observer(PublicationCheckpoint::RootInstalledBeforePhase)?;
                journal.append(&JournalRecord::Installed)?;
                atomic_rename_noreplace(
                    &parent,
                    &names.stage,
                    &names.backup,
                    "install previous backup",
                )?;
                sync_dir(&parent, "sync parent after previous backup install")?;
                observer(PublicationCheckpoint::BackupInstalledBeforePhase)?;
                journal.append(&JournalRecord::BackupInstalled)?;
                journal.append(&JournalRecord::Committed)?;
                Ok(ProjectPublicationOutcome::ReplacementRevisionPublished)
            }
            None => {
                atomic_rename_noreplace(
                    &parent,
                    &names.stage,
                    root_basename,
                    "install initial project root",
                )?;
                sync_dir(&parent, "sync parent after initial root install")?;
                observer(PublicationCheckpoint::RootInstalledBeforePhase)?;
                journal.append(&JournalRecord::Installed)?;
                journal.append(&JournalRecord::Committed)?;
                Ok(ProjectPublicationOutcome::InitialRevisionPublished)
            }
        }
    }

    pub(super) fn recover(
        ambient_parent: &Path,
        root_basename: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<ProjectRecoveryOutcome, ProjectFilesystemError> {
        let limits = limits.validate()?;
        validate_root_basename(root_basename)?;
        let names = internal_names(root_basename)?;
        let max_journal = journal_byte_limit(limits)?;
        let parent = Dir::open_ambient_dir(ambient_parent, ambient_authority())
            .map_err(|source| io_error("open ambient parent", "<project-parent>", source))?;
        let _publication_lock = lock_parent(&parent)?;
        recover_in_parent(&parent, root_basename, &names, limits, max_journal)
    }

    fn internal_names(root_basename: &OsStr) -> Result<InternalNames, ProjectFilesystemError> {
        let token = world_project_sha256(root_basename.as_bytes());
        let make = |suffix: &str| OsString::from(format!(".oteryn-{token}.{suffix}"));
        let names = InternalNames {
            stage: make("stage"),
            backup: make("previous"),
            journal: make("journal"),
        };
        for name in [&names.stage, &names.backup, &names.journal] {
            if name.as_bytes().len() > LINUX_NAME_MAX || name == root_basename {
                return Err(conflict(
                    "derived publication name is not a distinct Linux name",
                ));
            }
        }
        Ok(names)
    }

    fn journal_byte_limit(
        limits: ProjectFilesystemLimits,
    ) -> Result<usize, ProjectFilesystemError> {
        // Two bounded tree plans (previous and stage), encoded raw names, plus fixed phase records.
        let per_entry = LINUX_NAME_MAX
            .checked_mul(2)
            .and_then(|value| value.checked_add(384))
            .ok_or(ProjectFilesystemError::LimitExceeded {
                resource: "publication journal bytes",
                actual: usize::MAX,
                limit: usize::MAX,
            })?;
        limits
            .max_total_directory_entries_scanned
            .checked_mul(2)
            .and_then(|count| count.checked_mul(per_entry))
            .and_then(|bytes| bytes.checked_add(16_384))
            .ok_or(ProjectFilesystemError::LimitExceeded {
                resource: "publication journal bytes",
                actual: usize::MAX,
                limit: usize::MAX,
            })
    }

    fn raw_hex(value: &OsStr) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(value.as_bytes().len() * 2);
        for byte in value.as_bytes() {
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    fn decode_raw_hex(value: &str) -> Result<OsString, ProjectFilesystemError> {
        if !value.len().is_multiple_of(2) || value.len() > LINUX_NAME_MAX * 2 {
            return Err(conflict("journal contains an invalid raw name"));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(value.len() / 2)
            .map_err(|_| conflict("journal raw-name allocation failed"))?;
        for pair in value.as_bytes().chunks_exact(2) {
            let high = hex_digit(pair[0]).ok_or_else(|| conflict("journal raw name is not hex"))?;
            let low = hex_digit(pair[1]).ok_or_else(|| conflict("journal raw name is not hex"))?;
            bytes.push((high << 4) | low);
        }
        if bytes.is_empty() || bytes.contains(&b'/') || bytes.contains(&0) {
            return Err(conflict("journal raw name is not one Linux component"));
        }
        Ok(OsString::from_vec(bytes))
    }

    fn hex_digit(value: u8) -> Option<u8> {
        match value {
            b'0'..=b'9' => Some(value - b'0'),
            b'a'..=b'f' => Some(value - b'a' + 10),
            _ => None,
        }
    }

    fn conflict(reason: &'static str) -> ProjectFilesystemError {
        ProjectFilesystemError::PublicationConflict(reason)
    }

    fn sync_dir(directory: &Dir, operation: &'static str) -> Result<(), ProjectFilesystemError> {
        let sync_handle = rustix::fs::openat(
            directory,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|source| io_error(operation, "<publication-directory>", source.into()))?;
        rustix::fs::fsync(sync_handle)
            .map_err(|source| io_error(operation, "<publication-directory>", source.into()))
    }

    fn lock_parent(parent: &Dir) -> Result<OwnedFd, ProjectFilesystemError> {
        let handle = rustix::fs::openat(
            parent,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|source| {
            io_error(
                "open project parent lock handle",
                "<project-parent>",
                source.into(),
            )
        })?;
        rustix::fs::flock(&handle, FlockOperation::NonBlockingLockExclusive).map_err(|source| {
            io_error(
                "acquire exclusive project publication lock",
                "<project-parent>",
                source.into(),
            )
        })?;
        Ok(handle)
    }

    fn atomic_exchange(
        parent: &Dir,
        root_basename: &OsStr,
        stage: &OsStr,
    ) -> Result<(), ProjectFilesystemError> {
        rustix::fs::renameat_with(parent, root_basename, parent, stage, RenameFlags::EXCHANGE)
            .map_err(|source| {
                io_error(
                    "atomically exchange project roots",
                    "<project-root>",
                    source.into(),
                )
            })
    }

    fn atomic_rename_noreplace(
        parent: &Dir,
        from: &OsStr,
        to: &OsStr,
        operation: &'static str,
    ) -> Result<(), ProjectFilesystemError> {
        rustix::fs::renameat_with(parent, from, parent, to, RenameFlags::NOREPLACE)
            .map_err(|source| io_error(operation, "<publication-rename>", source.into()))
    }

    fn optional_entry_identity(
        parent: &Dir,
        name: &OsStr,
        locator: &str,
        scans: &mut ScanBudget,
    ) -> Result<Option<FileIdentity>, ProjectFilesystemError> {
        let Some(metadata) = optional_exact_entry_metadata(parent, name, locator, scans)? else {
            return Ok(None);
        };
        if !metadata.is_dir() {
            return Err(conflict(
                "publication root role is not an ordinary directory",
            ));
        }
        let opened = parent.open_dir_nofollow(name).map_err(|source| {
            io_error(
                "open publication directory without following links",
                locator,
                source,
            )
        })?;
        let opened_metadata = opened
            .dir_metadata()
            .map_err(|source| io_error("inspect opened publication directory", locator, source))?;
        let identity = FileIdentity::from_metadata(&metadata);
        if !opened_metadata.is_dir() || FileIdentity::from_metadata(&opened_metadata) != identity {
            return Err(conflict(
                "publication directory identity changed while opening",
            ));
        }
        Ok(Some(identity))
    }

    fn require_identity(
        parent: &Dir,
        name: &OsStr,
        kind: PlannedKind,
        identity: FileIdentity,
        locator: &str,
    ) -> Result<(), ProjectFilesystemError> {
        let metadata = parent.symlink_metadata(name).map_err(|source| {
            io_error(
                "inspect publication entry without following links",
                locator,
                source,
            )
        })?;
        let kind_matches = match kind {
            PlannedKind::Directory => metadata.is_dir(),
            PlannedKind::RegularFile => {
                metadata.is_file() && IdentityMetadataExt::nlink(&metadata) == 1
            }
        };
        if !kind_matches || FileIdentity::from_metadata(&metadata) != identity {
            return Err(conflict("publication entry type or identity changed"));
        }
        Ok(())
    }

    impl JournalWriter {
        fn create(
            parent: &Dir,
            journal_name: &OsStr,
            root_basename: &OsStr,
            max_length: usize,
        ) -> Result<Self, ProjectFilesystemError> {
            let mut options = safe_write_options();
            options.create_new(true).read(true);
            let file = parent.open_with(journal_name, &options).map_err(|source| {
                io_error(
                    "create publication journal",
                    "<publication-journal>",
                    source,
                )
            })?;
            let metadata = file.metadata().map_err(|source| {
                io_error(
                    "inspect publication journal",
                    "<publication-journal>",
                    source,
                )
            })?;
            let identity = FileIdentity::from_metadata(&metadata);
            if !metadata.is_file() || IdentityMetadataExt::nlink(&metadata) != 1 {
                return Err(conflict(
                    "new publication journal is not a unique regular file",
                ));
            }
            let mut writer = Self {
                file,
                last_digest: ZERO_DIGEST.to_owned(),
                length: 0,
                max_length,
            };
            writer.append(&JournalRecord::Header {
                schema: JOURNAL_SCHEMA.to_owned(),
                root_name_hex: raw_hex(root_basename),
                journal_identity: identity,
            })?;
            sync_dir(parent, "sync parent after publication journal creation")?;
            Ok(writer)
        }

        fn open_existing(
            parent: &Dir,
            journal_name: &OsStr,
            state: &JournalState,
            max_length: usize,
        ) -> Result<Self, ProjectFilesystemError> {
            require_identity(
                parent,
                journal_name,
                PlannedKind::RegularFile,
                state.journal_identity,
                "<publication-journal>",
            )?;
            let mut options = safe_write_options();
            options.read(true).append(true);
            let file = parent.open_with(journal_name, &options).map_err(|source| {
                io_error(
                    "open publication journal for recovery",
                    "<publication-journal>",
                    source,
                )
            })?;
            let metadata = file.metadata().map_err(|source| {
                io_error(
                    "inspect recovery journal handle",
                    "<publication-journal>",
                    source,
                )
            })?;
            if FileIdentity::from_metadata(&metadata) != state.journal_identity
                || !metadata.is_file()
                || IdentityMetadataExt::nlink(&metadata) != 1
            {
                return Err(conflict(
                    "publication journal changed while opening for recovery",
                ));
            }
            if metadata.len() != state.complete_length {
                file.set_len(state.complete_length).map_err(|source| {
                    io_error(
                        "trim incomplete journal suffix",
                        "<publication-journal>",
                        source,
                    )
                })?;
                file.sync_all().map_err(|source| {
                    io_error(
                        "sync trimmed publication journal",
                        "<publication-journal>",
                        source,
                    )
                })?;
            }
            Ok(Self {
                file,
                last_digest: state.last_digest.clone(),
                length: usize::try_from(state.complete_length).map_err(|_| {
                    ProjectFilesystemError::LimitExceeded {
                        resource: "publication journal bytes",
                        actual: usize::MAX,
                        limit: max_length,
                    }
                })?,
                max_length,
            })
        }

        fn append(&mut self, payload: &JournalRecord) -> Result<(), ProjectFilesystemError> {
            let payload_bytes = serde_json::to_vec(payload)
                .map_err(|_| conflict("publication journal record cannot be encoded"))?;
            let mut chained = Vec::new();
            chained
                .try_reserve_exact(self.last_digest.len() + payload_bytes.len())
                .map_err(|_| conflict("publication journal checksum allocation failed"))?;
            chained.extend_from_slice(self.last_digest.as_bytes());
            chained.extend_from_slice(&payload_bytes);
            let digest = world_project_sha256(&chained);
            let line = JournalLine {
                previous_sha256: self.last_digest.clone(),
                payload: payload.clone(),
                sha256: digest.clone(),
            };
            let mut encoded = serde_json::to_vec(&line)
                .map_err(|_| conflict("publication journal line cannot be encoded"))?;
            encoded.push(b'\n');
            let next = self.length.checked_add(encoded.len()).ok_or(
                ProjectFilesystemError::LimitExceeded {
                    resource: "publication journal bytes",
                    actual: usize::MAX,
                    limit: self.max_length,
                },
            )?;
            if next > self.max_length {
                return Err(ProjectFilesystemError::LimitExceeded {
                    resource: "publication journal bytes",
                    actual: next,
                    limit: self.max_length,
                });
            }
            self.file.write_all(&encoded).map_err(|source| {
                io_error(
                    "append publication journal",
                    "<publication-journal>",
                    source,
                )
            })?;
            self.file.sync_all().map_err(|source| {
                io_error("sync publication journal", "<publication-journal>", source)
            })?;
            self.length = next;
            self.last_digest = digest;
            Ok(())
        }
    }

    fn safe_write_options() -> OpenOptions {
        let mut options = OpenOptions::new();
        options
            .write(true)
            .follow(FollowSymlinks::No)
            .nonblock(true);
        cap_std::fs::OpenOptionsExt::custom_flags(&mut options, OFlags::NOCTTY.bits() as i32);
        options
    }

    fn read_journal(
        parent: &Dir,
        journal_name: &OsStr,
        root_basename: &OsStr,
        max_length: usize,
        limits: ProjectFilesystemLimits,
    ) -> Result<Option<JournalState>, ProjectFilesystemError> {
        let mut scans = ScanBudget::new(limits);
        let Some(entry_metadata) = optional_exact_entry_metadata(
            parent,
            journal_name,
            "<publication-journal>",
            &mut scans,
        )?
        else {
            return Ok(None);
        };
        if !entry_metadata.is_file() || IdentityMetadataExt::nlink(&entry_metadata) != 1 {
            return Err(conflict("publication journal is not a unique regular file"));
        }
        let file = parent
            .open_with(journal_name, &safe_read_options())
            .map_err(|source| {
                io_error("open publication journal", "<publication-journal>", source)
            })?;
        let metadata = file.metadata().map_err(|source| {
            io_error(
                "inspect opened publication journal",
                "<publication-journal>",
                source,
            )
        })?;
        let identity = FileIdentity::from_metadata(&metadata);
        if !metadata.is_file()
            || IdentityMetadataExt::nlink(&metadata) != 1
            || FileIdentity::from_metadata(&entry_metadata) != identity
        {
            return Err(conflict("publication journal changed while opening"));
        }
        let length =
            usize::try_from(metadata.len()).map_err(|_| ProjectFilesystemError::LimitExceeded {
                resource: "publication journal bytes",
                actual: usize::MAX,
                limit: max_length,
            })?;
        if length > max_length {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "publication journal bytes",
                actual: length,
                limit: max_length,
            });
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| ProjectFilesystemError::LimitExceeded {
                resource: "publication journal allocation",
                actual: length,
                limit: max_length,
            })?;
        bytes.resize(length, 0);
        let mut file = file;
        file.read_exact(&mut bytes).map_err(|source| {
            io_error(
                "read exact publication journal",
                "<publication-journal>",
                source,
            )
        })?;
        parse_journal(&bytes, identity, root_basename, limits).map(Some)
    }

    fn parse_journal(
        bytes: &[u8],
        journal_identity: FileIdentity,
        root_basename: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<JournalState, ProjectFilesystemError> {
        let complete_length = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .ok_or_else(|| conflict("publication journal has no durable header"))?;
        let record_capacity = limits
            .max_total_directory_entries_scanned
            .checked_mul(2)
            .and_then(|value| value.checked_add(8))
            .ok_or(ProjectFilesystemError::LimitExceeded {
                resource: "publication journal records",
                actual: usize::MAX,
                limit: limits.max_total_directory_entries_scanned,
            })?;
        let mut records = Vec::new();
        records
            .try_reserve(record_capacity)
            .map_err(|_| conflict("publication journal record allocation failed"))?;
        let mut last_digest = ZERO_DIGEST.to_owned();
        for raw_line in bytes[..complete_length].split(|byte| *byte == b'\n') {
            if raw_line.is_empty() {
                continue;
            }
            let line: JournalLine = serde_json::from_slice(raw_line)
                .map_err(|_| conflict("publication journal contains malformed durable data"))?;
            if line.previous_sha256 != last_digest {
                return Err(conflict(
                    "publication journal checksum chain is discontinuous",
                ));
            }
            let payload_bytes = serde_json::to_vec(&line.payload)
                .map_err(|_| conflict("publication journal payload cannot be normalized"))?;
            let mut chained = Vec::new();
            chained
                .try_reserve_exact(last_digest.len() + payload_bytes.len())
                .map_err(|_| conflict("publication journal checksum allocation failed"))?;
            chained.extend_from_slice(last_digest.as_bytes());
            chained.extend_from_slice(&payload_bytes);
            let digest = world_project_sha256(&chained);
            if line.sha256 != digest {
                return Err(conflict("publication journal checksum does not match"));
            }
            last_digest = digest;
            records.push(line.payload);
        }

        let mut iter = records.into_iter();
        match iter.next() {
            Some(JournalRecord::Header {
                schema,
                root_name_hex,
                journal_identity: recorded_identity,
            }) if schema == JOURNAL_SCHEMA
                && root_name_hex == raw_hex(root_basename)
                && recorded_identity == journal_identity => {}
            _ => {
                return Err(conflict(
                    "publication journal header does not match this root",
                ));
            }
        }

        let mut previous_entries = Vec::new();
        let mut previous = None;
        let mut previous_digest = None;
        let mut replacement_digest = None;
        let mut previous_ready = false;
        let mut stage_root = None;
        let mut stage_entries = Vec::new();
        let mut stage_ready = false;
        let mut installed = false;
        let mut backup_installed = false;
        let mut committed = false;
        for record in iter {
            match record {
                JournalRecord::PreviousEntry { entry }
                    if !previous_ready
                        && previous_entries.len() < limits.max_total_directory_entries_scanned =>
                {
                    validate_planned_entry(&entry)?;
                    previous_entries.push(entry);
                }
                JournalRecord::PreviousReady {
                    previous_root,
                    previous_digest: digest,
                    replacement_digest: replacement,
                } if !previous_ready => {
                    if previous_root.is_none() != previous_entries.is_empty()
                        || previous_root.is_none() != digest.is_none()
                    {
                        return Err(conflict(
                            "publication journal previous plan is inconsistent",
                        ));
                    }
                    validate_digest(&replacement)?;
                    if let Some(value) = &digest {
                        validate_digest(value)?;
                    }
                    previous = previous_root.map(|root| TreePlan {
                        root,
                        entries: std::mem::take(&mut previous_entries),
                    });
                    previous_digest = digest;
                    replacement_digest = Some(replacement);
                    previous_ready = true;
                }
                JournalRecord::StageRoot { identity }
                    if previous_ready && stage_root.is_none() && !stage_ready =>
                {
                    stage_root = Some(identity);
                }
                JournalRecord::StageEntry { entry }
                    if stage_root.is_some()
                        && !stage_ready
                        && stage_entries.len() < limits.max_total_directory_entries_scanned =>
                {
                    validate_planned_entry(&entry)?;
                    stage_entries.push(entry);
                }
                JournalRecord::StageReady if stage_root.is_some() && !stage_ready => {
                    stage_ready = true;
                }
                JournalRecord::Installed if stage_ready && !installed => installed = true,
                JournalRecord::BackupInstalled
                    if installed && previous.is_some() && !backup_installed =>
                {
                    backup_installed = true;
                }
                JournalRecord::Committed
                    if installed && !committed && (previous.is_none() || backup_installed) =>
                {
                    committed = true;
                }
                JournalRecord::Header { .. }
                | JournalRecord::PreviousEntry { .. }
                | JournalRecord::PreviousReady { .. }
                | JournalRecord::StageRoot { .. }
                | JournalRecord::StageEntry { .. }
                | JournalRecord::StageReady
                | JournalRecord::Installed
                | JournalRecord::BackupInstalled
                | JournalRecord::Committed => {
                    return Err(conflict("publication journal phase order is invalid"));
                }
            }
        }
        validate_plan_entries(previous.as_ref().map_or(&[][..], |plan| &plan.entries))?;
        validate_plan_entries(&stage_entries)?;
        Ok(JournalState {
            journal_identity,
            previous,
            previous_digest,
            replacement_digest,
            previous_ready,
            stage_root,
            stage_entries,
            stage_ready,
            installed,
            backup_installed,
            committed,
            last_digest,
            complete_length: u64::try_from(complete_length)
                .map_err(|_| conflict("publication journal length is not representable"))?,
        })
    }

    fn validate_digest(value: &str) -> Result<(), ProjectFilesystemError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(conflict("publication journal contains an invalid digest"));
        }
        Ok(())
    }

    fn validate_planned_entry(entry: &PlannedEntry) -> Result<(), ProjectFilesystemError> {
        let _ = decode_raw_hex(&entry.name_hex)?;
        Ok(())
    }

    fn validate_plan_entries(entries: &[PlannedEntry]) -> Result<(), ProjectFilesystemError> {
        let mut keys = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for entry in entries {
            if !keys.insert((entry.parent, entry.name_hex.clone()))
                || !identities.insert(entry.identity)
            {
                return Err(conflict(
                    "publication journal plan contains an alias or duplicate",
                ));
            }
        }
        Ok(())
    }

    fn existing_locators_and_digest(
        parent: &Dir,
        root_basename: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<(BTreeSet<String>, String), ProjectFilesystemError> {
        let root = parent.open_dir_nofollow(root_basename).map_err(|source| {
            io_error(
                "open current root for publication planning",
                "<project-root>",
                source,
            )
        })?;
        let mut scans = ScanBudget::new(limits);
        let mut identities = BTreeSet::new();
        let mut controls = BTreeMap::new();
        for locator in [PROJECT_LOCATOR, MANIFEST_LOCATOR, LOCK_LOCATOR] {
            let pinned = open_locator(&root, locator, limits, &mut scans, &mut identities)?;
            controls.insert(locator.to_owned(), consume_pinned(pinned)?);
        }
        let plan = ProjectCapturePlan::from_control_documents(
            &controls[PROJECT_LOCATOR],
            &controls[MANIFEST_LOCATOR],
            &controls[LOCK_LOCATOR],
            limits.project,
        )?;
        let mut locators = BTreeSet::from([
            PROJECT_LOCATOR.to_owned(),
            MANIFEST_LOCATOR.to_owned(),
            LOCK_LOCATOR.to_owned(),
        ]);
        for document in plan.documents() {
            locators.insert(document.locator.clone());
        }
        Ok((locators, world_project_sha256(&controls[PROJECT_LOCATOR])))
    }

    fn expected_paths(
        locators: &BTreeSet<String>,
    ) -> Result<BTreeMap<String, PlannedKind>, ProjectFilesystemError> {
        let mut paths = BTreeMap::new();
        for locator in locators {
            let segments: Vec<_> = locator.split('/').collect();
            for index in 1..segments.len() {
                let path = segments[..index].join("/");
                match paths.insert(path, PlannedKind::Directory) {
                    Some(PlannedKind::RegularFile) => {
                        return Err(conflict("project locator file is also a directory"));
                    }
                    Some(PlannedKind::Directory) | None => {}
                }
            }
            match paths.insert(locator.clone(), PlannedKind::RegularFile) {
                Some(PlannedKind::Directory) => {
                    return Err(conflict("project locator directory is also a file"));
                }
                Some(PlannedKind::RegularFile) | None => {}
            }
        }
        Ok(paths)
    }

    fn scan_exact_tree(
        parent: &Dir,
        root_name: &OsStr,
        locators: &BTreeSet<String>,
        limits: ProjectFilesystemLimits,
    ) -> Result<TreePlan, ProjectFilesystemError> {
        let expected = expected_paths(locators)?;
        if expected.len() > limits.max_total_directory_entries_scanned {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "publication tree entries",
                actual: expected.len(),
                limit: limits.max_total_directory_entries_scanned,
            });
        }
        let mut scans = ScanBudget::new(limits);
        let root_entry = exact_entry_metadata(parent, root_name, "<publication-tree>", &mut scans)?;
        if !root_entry.is_dir() {
            return Err(conflict("publication tree root is not a directory"));
        }
        let root = parent.open_dir_nofollow(root_name).map_err(|source| {
            io_error(
                "open publication tree root without following links",
                "<publication-tree>",
                source,
            )
        })?;
        let root_metadata = root.dir_metadata().map_err(|source| {
            io_error(
                "inspect publication tree root",
                "<publication-tree>",
                source,
            )
        })?;
        let root_identity = FileIdentity::from_metadata(&root_metadata);
        if !root_metadata.is_dir() || FileIdentity::from_metadata(&root_entry) != root_identity {
            return Err(conflict("publication tree root changed during planning"));
        }
        let mut found = BTreeSet::new();
        let mut identities = BTreeSet::from([root_identity]);
        let mut entries = Vec::new();
        scan_exact_directory(
            &root,
            root_identity,
            "",
            &expected,
            &mut found,
            &mut identities,
            &mut entries,
            &mut scans,
        )?;
        if found.len() != expected.len() {
            return Err(conflict(
                "publication tree is missing a planned project entry",
            ));
        }
        Ok(TreePlan {
            root: root_identity,
            entries,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_exact_directory(
        directory: &Dir,
        directory_identity: FileIdentity,
        prefix: &str,
        expected: &BTreeMap<String, PlannedKind>,
        found: &mut BTreeSet<String>,
        identities: &mut BTreeSet<FileIdentity>,
        planned: &mut Vec<PlannedEntry>,
        scans: &mut ScanBudget,
    ) -> Result<(), ProjectFilesystemError> {
        let entries = directory.entries().map_err(|source| {
            io_error("enumerate publication tree", "<publication-tree>", source)
        })?;
        let mut in_scan = 0_usize;
        for result in entries {
            scans.admit(&mut in_scan)?;
            let entry = result.map_err(|source| {
                io_error("read publication tree entry", "<publication-tree>", source)
            })?;
            let name = entry.file_name();
            let text = name
                .to_str()
                .ok_or_else(|| conflict("publication tree contains a non-canonical raw name"))?;
            let relative = if prefix.is_empty() {
                text.to_owned()
            } else {
                format!("{prefix}/{text}")
            };
            let expected_kind = expected
                .get(&relative)
                .copied()
                .ok_or_else(|| conflict("publication tree contains an unmanaged entry"))?;
            let metadata = directory.symlink_metadata(&name).map_err(|source| {
                io_error(
                    "inspect publication tree entry without following links",
                    &relative,
                    source,
                )
            })?;
            let identity = FileIdentity::from_metadata(&metadata);
            if !identities.insert(identity) {
                return Err(conflict("publication tree contains an identity alias"));
            }
            let actual_kind = if metadata.is_dir() {
                PlannedKind::Directory
            } else if metadata.is_file() && IdentityMetadataExt::nlink(&metadata) == 1 {
                PlannedKind::RegularFile
            } else {
                return Err(conflict(
                    "publication tree contains a link or special entry",
                ));
            };
            if actual_kind != expected_kind {
                return Err(conflict(
                    "publication tree entry type differs from its locator role",
                ));
            }
            planned.push(PlannedEntry {
                parent: directory_identity,
                name_hex: raw_hex(&name),
                kind: actual_kind,
                identity,
            });
            found.insert(relative.clone());
            if actual_kind == PlannedKind::Directory {
                let child = directory.open_dir_nofollow(&name).map_err(|source| {
                    io_error(
                        "open publication tree child without following links",
                        &relative,
                        source,
                    )
                })?;
                let opened = child.dir_metadata().map_err(|source| {
                    io_error("inspect opened publication tree child", &relative, source)
                })?;
                if !opened.is_dir() || FileIdentity::from_metadata(&opened) != identity {
                    return Err(conflict(
                        "publication tree directory changed during planning",
                    ));
                }
                scan_exact_directory(
                    &child, identity, &relative, expected, found, identities, planned, scans,
                )?;
            }
        }
        Ok(())
    }

    fn write_stage(
        parent: &Dir,
        stage_name: &OsStr,
        documents: &CanonicalProjectDocuments,
        limits: ProjectFilesystemLimits,
        journal: &mut JournalWriter,
        observer: &mut dyn FnMut(PublicationCheckpoint) -> Result<(), ProjectFilesystemError>,
    ) -> Result<TreePlan, ProjectFilesystemError> {
        let locators: BTreeSet<_> = documents.documents().keys().cloned().collect();
        let expected = expected_paths(&locators)?;
        if expected.len() > limits.max_total_directory_entries_scanned {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "publication stage entries",
                actual: expected.len(),
                limit: limits.max_total_directory_entries_scanned,
            });
        }
        let mut child_counts = BTreeMap::<String, usize>::new();
        for path in expected.keys() {
            let parent_path = path.rsplit_once('/').map_or("", |(value, _)| value);
            let count = child_counts.entry(parent_path.to_owned()).or_default();
            *count = count
                .checked_add(1)
                .ok_or(ProjectFilesystemError::LimitExceeded {
                    resource: "publication directory entries",
                    actual: usize::MAX,
                    limit: limits.max_entries_per_directory_scan,
                })?;
            if *count > limits.max_entries_per_directory_scan {
                return Err(ProjectFilesystemError::LimitExceeded {
                    resource: "publication directory entries",
                    actual: *count,
                    limit: limits.max_entries_per_directory_scan,
                });
            }
        }

        parent.create_dir(stage_name).map_err(|source| {
            io_error(
                "create publication stage root",
                "<publication-stage>",
                source,
            )
        })?;
        let root_metadata = parent.symlink_metadata(stage_name).map_err(|source| {
            io_error(
                "inspect publication stage root",
                "<publication-stage>",
                source,
            )
        })?;
        if !root_metadata.is_dir() {
            return Err(conflict("new publication stage root is not a directory"));
        }
        let root_identity = FileIdentity::from_metadata(&root_metadata);
        let root = parent.open_dir_nofollow(stage_name).map_err(|source| {
            io_error(
                "open publication stage root without following links",
                "<publication-stage>",
                source,
            )
        })?;
        let opened_root = root.dir_metadata().map_err(|source| {
            io_error(
                "inspect opened publication stage root",
                "<publication-stage>",
                source,
            )
        })?;
        if FileIdentity::from_metadata(&opened_root) != root_identity {
            return Err(conflict("publication stage root changed while opening"));
        }
        observer(PublicationCheckpoint::StageRootCreatedBeforePlan)?;
        journal.append(&JournalRecord::StageRoot {
            identity: root_identity,
        })?;

        let mut identities = BTreeMap::from([(String::new(), root_identity)]);
        let mut planned = Vec::new();
        let mut directories: Vec<_> = expected
            .iter()
            .filter_map(|(path, kind)| (*kind == PlannedKind::Directory).then_some(path.clone()))
            .collect();
        directories.sort_by_key(|path| (path.matches('/').count(), path.clone()));
        for path in &directories {
            let (parent_path, name) = split_relative(path)?;
            let parent_dir = open_relative_directory(&root, parent_path, &identities)?;
            parent_dir
                .create_dir(name)
                .map_err(|source| io_error("create staged project directory", path, source))?;
            let metadata = parent_dir
                .symlink_metadata(name)
                .map_err(|source| io_error("inspect staged project directory", path, source))?;
            if !metadata.is_dir() {
                return Err(conflict("created stage directory has the wrong type"));
            }
            let identity = FileIdentity::from_metadata(&metadata);
            observer(PublicationCheckpoint::StageEntryCreatedBeforePlan)?;
            let entry = PlannedEntry {
                parent: identities[parent_path],
                name_hex: raw_hex(OsStr::new(name)),
                kind: PlannedKind::Directory,
                identity,
            };
            journal.append(&JournalRecord::StageEntry {
                entry: entry.clone(),
            })?;
            planned.push(entry);
            identities.insert(path.clone(), identity);
        }

        let mut file_order: Vec<_> = documents
            .documents()
            .keys()
            .filter(|locator| {
                !matches!(
                    locator.as_str(),
                    PROJECT_LOCATOR | MANIFEST_LOCATOR | LOCK_LOCATOR
                )
            })
            .cloned()
            .collect();
        file_order.extend([
            MANIFEST_LOCATOR.to_owned(),
            LOCK_LOCATOR.to_owned(),
            PROJECT_LOCATOR.to_owned(),
        ]);
        for locator in file_order {
            let (parent_path, name) = split_relative(&locator)?;
            let parent_dir = open_relative_directory(&root, parent_path, &identities)?;
            let mut options = safe_write_options();
            options.create_new(true);
            let mut file = parent_dir
                .open_with(name, &options)
                .map_err(|source| io_error("create staged project file", &locator, source))?;
            let metadata = file
                .metadata()
                .map_err(|source| io_error("inspect staged project file", &locator, source))?;
            if !metadata.is_file() || IdentityMetadataExt::nlink(&metadata) != 1 {
                return Err(conflict("created stage file is not a unique regular file"));
            }
            let identity = FileIdentity::from_metadata(&metadata);
            observer(PublicationCheckpoint::StageEntryCreatedBeforePlan)?;
            let entry = PlannedEntry {
                parent: identities[parent_path],
                name_hex: raw_hex(OsStr::new(name)),
                kind: PlannedKind::RegularFile,
                identity,
            };
            journal.append(&JournalRecord::StageEntry {
                entry: entry.clone(),
            })?;
            file.write_all(&documents.documents()[&locator])
                .map_err(|source| io_error("write staged project file", &locator, source))?;
            file.sync_all()
                .map_err(|source| io_error("sync staged project file", &locator, source))?;
            planned.push(entry);
        }

        for path in directories.iter().rev() {
            let directory = open_relative_directory(&root, path, &identities)?;
            sync_dir(&directory, "sync staged project directory")?;
        }
        sync_dir(&root, "sync publication stage root")?;
        sync_dir(parent, "sync parent after stage creation")?;
        validate_plan_entries(&planned)?;
        Ok(TreePlan {
            root: root_identity,
            entries: planned,
        })
    }

    fn split_relative(path: &str) -> Result<(&str, &str), ProjectFilesystemError> {
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
        if name.is_empty() {
            return Err(conflict("project locator has an empty final component"));
        }
        Ok((parent, name))
    }

    fn open_relative_directory(
        root: &Dir,
        path: &str,
        identities: &BTreeMap<String, FileIdentity>,
    ) -> Result<Dir, ProjectFilesystemError> {
        let mut current = root
            .try_clone()
            .map_err(|source| io_error("clone publication stage root", path, source))?;
        let mut prefix = String::new();
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            let expected = identities
                .get(&prefix)
                .copied()
                .ok_or_else(|| conflict("staged directory has no durable identity"))?;
            let next = current.open_dir_nofollow(segment).map_err(|source| {
                io_error(
                    "open staged directory without following links",
                    path,
                    source,
                )
            })?;
            let metadata = next
                .dir_metadata()
                .map_err(|source| io_error("inspect opened staged directory", path, source))?;
            if !metadata.is_dir() || FileIdentity::from_metadata(&metadata) != expected {
                return Err(conflict("staged directory identity changed"));
            }
            current = next;
        }
        Ok(current)
    }

    fn recover_in_parent(
        parent: &Dir,
        root_basename: &OsStr,
        names: &InternalNames,
        limits: ProjectFilesystemLimits,
        max_journal: usize,
    ) -> Result<ProjectRecoveryOutcome, ProjectFilesystemError> {
        let Some(state) = read_journal(parent, &names.journal, root_basename, max_journal, limits)?
        else {
            if entry_exists(parent, &names.stage, limits)?
                || entry_exists(parent, &names.backup, limits)?
            {
                return Err(conflict(
                    "publication residue has no durable authorization journal",
                ));
            }
            return Ok(ProjectRecoveryOutcome::NoTransaction);
        };

        if !state.previous_ready {
            if entry_exists(parent, &names.stage, limits)?
                || entry_exists(parent, &names.backup, limits)?
            {
                return Err(conflict(
                    "unplanned publication entry exists before the durable plan",
                ));
            }
            remove_journal(parent, &names.journal, &state)?;
            return Ok(ProjectRecoveryOutcome::PreviousRevisionPreserved);
        }

        let Some(stage_root) = state.stage_root else {
            if entry_exists(parent, &names.stage, limits)?
                || entry_exists(parent, &names.backup, limits)?
            {
                return Err(conflict(
                    "publication stage exists without a durable root identity",
                ));
            }
            require_previous_live(parent, root_basename, &state, limits)?;
            remove_journal(parent, &names.journal, &state)?;
            return Ok(ProjectRecoveryOutcome::PreviousRevisionPreserved);
        };
        let stage_plan = TreePlan {
            root: stage_root,
            entries: state.stage_entries.clone(),
        };

        if !state.stage_ready {
            require_previous_live(parent, root_basename, &state, limits)?;
            if entry_exists(parent, &names.backup, limits)? {
                return Err(conflict(
                    "backup appeared before the staged revision was durable",
                ));
            }
            remove_tree_subset(parent, &names.stage, &stage_plan, limits)?;
            remove_journal(parent, &names.journal, &state)?;
            return Ok(ProjectRecoveryOutcome::PreviousRevisionPreserved);
        }

        let live = directory_identity(parent, root_basename, limits)?;
        let stage = directory_identity(parent, &names.stage, limits)?;
        let backup = directory_identity(parent, &names.backup, limits)?;
        let previous_root = state.previous.as_ref().map(|plan| plan.root);
        match previous_root {
            None => {
                if backup.is_some() {
                    return Err(conflict("initial publication has an unexpected backup"));
                }
                if live.is_none() && stage == Some(stage_root) && !state.installed {
                    remove_tree_subset(parent, &names.stage, &stage_plan, limits)?;
                    remove_journal(parent, &names.journal, &state)?;
                    return Ok(ProjectRecoveryOutcome::PreviousRevisionPreserved);
                }
                if live == Some(stage_root) && stage.is_none() {
                    verify_replacement(parent, root_basename, &stage_plan, &state, limits)?;
                    let was_committed = state.committed;
                    let mut journal =
                        JournalWriter::open_existing(parent, &names.journal, &state, max_journal)?;
                    if !state.installed {
                        journal.append(&JournalRecord::Installed)?;
                    }
                    if !state.committed {
                        journal.append(&JournalRecord::Committed)?;
                    }
                    return Ok(if was_committed {
                        ProjectRecoveryOutcome::CommittedRevisionVerified
                    } else {
                        ProjectRecoveryOutcome::ReplacementRevisionRecovered
                    });
                }
            }
            Some(previous_identity) => {
                let previous_plan = state
                    .previous
                    .as_ref()
                    .ok_or_else(|| conflict("previous plan disappeared during recovery"))?;
                if live == Some(previous_identity)
                    && (stage == Some(stage_root) || stage.is_none())
                    && backup.is_none()
                    && !state.installed
                {
                    verify_previous(parent, root_basename, previous_plan, &state, limits)?;
                    remove_tree_subset(parent, &names.stage, &stage_plan, limits)?;
                    remove_journal(parent, &names.journal, &state)?;
                    return Ok(ProjectRecoveryOutcome::PreviousRevisionPreserved);
                }
                if live == Some(stage_root) && stage == Some(previous_identity) && backup.is_none()
                {
                    verify_replacement(parent, root_basename, &stage_plan, &state, limits)?;
                    verify_previous(parent, &names.stage, previous_plan, &state, limits)?;
                    let mut journal =
                        JournalWriter::open_existing(parent, &names.journal, &state, max_journal)?;
                    if !state.installed {
                        journal.append(&JournalRecord::Installed)?;
                    }
                    atomic_rename_noreplace(
                        parent,
                        &names.stage,
                        &names.backup,
                        "finish previous backup install",
                    )?;
                    sync_dir(parent, "sync recovered previous backup install")?;
                    journal.append(&JournalRecord::BackupInstalled)?;
                    journal.append(&JournalRecord::Committed)?;
                    return Ok(ProjectRecoveryOutcome::ReplacementRevisionRecovered);
                }
                if live == Some(stage_root)
                    && stage.is_none()
                    && (backup == Some(previous_identity) || backup.is_none())
                {
                    verify_replacement(parent, root_basename, &stage_plan, &state, limits)?;
                    if backup.is_some() {
                        verify_tree_subset(parent, &names.backup, previous_plan, limits)?;
                    } else if !state.committed {
                        return Err(conflict("previous root vanished before commit was durable"));
                    }
                    let was_committed = state.committed;
                    let mut journal =
                        JournalWriter::open_existing(parent, &names.journal, &state, max_journal)?;
                    if !state.installed {
                        journal.append(&JournalRecord::Installed)?;
                    }
                    if !state.backup_installed {
                        journal.append(&JournalRecord::BackupInstalled)?;
                    }
                    if !state.committed {
                        journal.append(&JournalRecord::Committed)?;
                    }
                    return Ok(if was_committed {
                        ProjectRecoveryOutcome::CommittedRevisionVerified
                    } else {
                        ProjectRecoveryOutcome::ReplacementRevisionRecovered
                    });
                }
            }
        }
        Err(conflict(
            "publication entry identities do not match a recoverable phase",
        ))
    }

    fn require_previous_live(
        parent: &Dir,
        root_basename: &OsStr,
        state: &JournalState,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        match &state.previous {
            Some(plan) => verify_previous(parent, root_basename, plan, state, limits),
            None if directory_identity(parent, root_basename, limits)?.is_none() => Ok(()),
            None => Err(conflict("initial publication unexpectedly has a live root")),
        }
    }

    fn verify_replacement(
        parent: &Dir,
        name: &OsStr,
        plan: &TreePlan,
        state: &JournalState,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        verify_tree_complete(parent, name, plan, limits)?;
        let expected = state
            .replacement_digest
            .as_deref()
            .ok_or_else(|| conflict("publication journal has no replacement digest"))?;
        verify_root_digest(parent, name, expected, limits)
    }

    fn verify_previous(
        parent: &Dir,
        name: &OsStr,
        plan: &TreePlan,
        state: &JournalState,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        verify_tree_complete(parent, name, plan, limits)?;
        let expected = state
            .previous_digest
            .as_deref()
            .ok_or_else(|| conflict("publication journal has no previous digest"))?;
        verify_root_digest(parent, name, expected, limits)
    }

    fn verify_root_digest(
        parent: &Dir,
        name: &OsStr,
        expected: &str,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        if existing_root_digest(parent, name, limits)? != expected {
            return Err(conflict(
                "project root digest differs from the durable journal role",
            ));
        }
        Ok(())
    }

    fn existing_root_digest(
        parent: &Dir,
        name: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<String, ProjectFilesystemError> {
        let root = parent.open_dir_nofollow(name).map_err(|source| {
            io_error(
                "open root for journal digest verification",
                "<project-root>",
                source,
            )
        })?;
        let mut scans = ScanBudget::new(limits);
        let mut identities = BTreeSet::new();
        let pinned = open_locator(&root, PROJECT_LOCATOR, limits, &mut scans, &mut identities)?;
        let bytes = consume_pinned(pinned)?;
        Ok(world_project_sha256(&bytes))
    }

    fn verify_tree_complete(
        parent: &Dir,
        root_name: &OsStr,
        plan: &TreePlan,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        let current = current_tree_subset(parent, root_name, plan, limits)?
            .ok_or_else(|| conflict("durable publication tree is absent"))?;
        let actual = count_current_entries(&current)?;
        if actual != plan.entries.len() {
            return Err(conflict(
                "durable publication tree is only a partial subset",
            ));
        }
        Ok(())
    }

    fn verify_tree_subset(
        parent: &Dir,
        root_name: &OsStr,
        plan: &TreePlan,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        let _ = current_tree_subset(parent, root_name, plan, limits)?;
        Ok(())
    }

    fn current_tree_subset(
        parent: &Dir,
        root_name: &OsStr,
        plan: &TreePlan,
        limits: ProjectFilesystemLimits,
    ) -> Result<Option<CurrentEntry>, ProjectFilesystemError> {
        let mut scans = ScanBudget::new(limits);
        let Some(metadata) = optional_exact_entry_metadata(
            parent,
            root_name,
            "<publication-cleanup-root>",
            &mut scans,
        )?
        else {
            return Ok(None);
        };
        if !metadata.is_dir() || FileIdentity::from_metadata(&metadata) != plan.root {
            return Err(conflict(
                "cleanup root type or identity differs from its durable plan",
            ));
        }
        let root = parent.open_dir_nofollow(root_name).map_err(|source| {
            io_error(
                "open cleanup root without following links",
                "<publication-cleanup-root>",
                source,
            )
        })?;
        let opened = root.dir_metadata().map_err(|source| {
            io_error(
                "inspect opened cleanup root",
                "<publication-cleanup-root>",
                source,
            )
        })?;
        if !opened.is_dir() || FileIdentity::from_metadata(&opened) != plan.root {
            return Err(conflict("cleanup root changed while opening"));
        }
        let mut by_key = BTreeMap::new();
        for entry in &plan.entries {
            by_key.insert((entry.parent, entry.name_hex.clone()), entry);
        }
        let children = inspect_subset_directory(&root, plan.root, &by_key, &mut scans)?;
        Ok(Some(CurrentEntry {
            name: root_name.to_owned(),
            kind: PlannedKind::Directory,
            identity: plan.root,
            children,
        }))
    }

    fn inspect_subset_directory(
        directory: &Dir,
        directory_identity: FileIdentity,
        plan: &BTreeMap<(FileIdentity, String), &PlannedEntry>,
        scans: &mut ScanBudget,
    ) -> Result<Vec<CurrentEntry>, ProjectFilesystemError> {
        let entries = directory.entries().map_err(|source| {
            io_error("enumerate cleanup tree", "<publication-cleanup>", source)
        })?;
        let mut in_scan = 0_usize;
        let mut current = Vec::new();
        for result in entries {
            scans.admit(&mut in_scan)?;
            let entry = result.map_err(|source| {
                io_error("read cleanup tree entry", "<publication-cleanup>", source)
            })?;
            let name = entry.file_name();
            let key = (directory_identity, raw_hex(&name));
            let expected = plan
                .get(&key)
                .copied()
                .ok_or_else(|| conflict("cleanup tree contains an unplanned addition"))?;
            let metadata = directory.symlink_metadata(&name).map_err(|source| {
                io_error(
                    "inspect cleanup entry without following links",
                    "<publication-cleanup>",
                    source,
                )
            })?;
            let actual_kind = if metadata.is_dir() {
                PlannedKind::Directory
            } else if metadata.is_file() && IdentityMetadataExt::nlink(&metadata) == 1 {
                PlannedKind::RegularFile
            } else {
                return Err(conflict("cleanup entry became a link or special file"));
            };
            if actual_kind != expected.kind
                || FileIdentity::from_metadata(&metadata) != expected.identity
            {
                return Err(conflict("cleanup entry type or identity was substituted"));
            }
            let children = if actual_kind == PlannedKind::Directory {
                let child = directory.open_dir_nofollow(&name).map_err(|source| {
                    io_error(
                        "open cleanup child without following links",
                        "<publication-cleanup>",
                        source,
                    )
                })?;
                let opened = child.dir_metadata().map_err(|source| {
                    io_error(
                        "inspect opened cleanup child",
                        "<publication-cleanup>",
                        source,
                    )
                })?;
                if !opened.is_dir() || FileIdentity::from_metadata(&opened) != expected.identity {
                    return Err(conflict("cleanup directory changed while opening"));
                }
                inspect_subset_directory(&child, expected.identity, plan, scans)?
            } else {
                Vec::new()
            };
            current.push(CurrentEntry {
                name,
                kind: actual_kind,
                identity: expected.identity,
                children,
            });
        }
        Ok(current)
    }

    fn count_current_entries(root: &CurrentEntry) -> Result<usize, ProjectFilesystemError> {
        let mut total = 0_usize;
        let mut pending: Vec<_> = root.children.iter().collect();
        while let Some(entry) = pending.pop() {
            total = total
                .checked_add(1)
                .ok_or(ProjectFilesystemError::LimitExceeded {
                    resource: "publication cleanup entries",
                    actual: usize::MAX,
                    limit: usize::MAX,
                })?;
            pending.extend(entry.children.iter());
        }
        Ok(total)
    }

    fn remove_tree_subset(
        parent: &Dir,
        root_name: &OsStr,
        plan: &TreePlan,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        let Some(current) = current_tree_subset(parent, root_name, plan, limits)? else {
            return Ok(());
        };
        let root = parent.open_dir_nofollow(root_name).map_err(|source| {
            io_error(
                "open authorized cleanup root",
                "<publication-cleanup-root>",
                source,
            )
        })?;
        remove_current_children(&root, &current.children)?;
        require_identity(
            parent,
            root_name,
            PlannedKind::Directory,
            current.identity,
            "<publication-cleanup-root>",
        )?;
        parent.remove_dir(root_name).map_err(|source| {
            io_error(
                "remove authorized cleanup root",
                "<publication-cleanup-root>",
                source,
            )
        })?;
        sync_dir(parent, "sync parent after authorized tree cleanup")
    }

    fn remove_current_children(
        parent: &Dir,
        children: &[CurrentEntry],
    ) -> Result<(), ProjectFilesystemError> {
        for entry in children {
            match entry.kind {
                PlannedKind::RegularFile => {
                    require_identity(
                        parent,
                        &entry.name,
                        PlannedKind::RegularFile,
                        entry.identity,
                        "<publication-cleanup-file>",
                    )?;
                    parent.remove_file(&entry.name).map_err(|source| {
                        io_error(
                            "remove authorized cleanup file",
                            "<publication-cleanup-file>",
                            source,
                        )
                    })?;
                }
                PlannedKind::Directory => {
                    require_identity(
                        parent,
                        &entry.name,
                        PlannedKind::Directory,
                        entry.identity,
                        "<publication-cleanup-directory>",
                    )?;
                    let directory = parent.open_dir_nofollow(&entry.name).map_err(|source| {
                        io_error(
                            "open authorized cleanup directory without following links",
                            "<publication-cleanup-directory>",
                            source,
                        )
                    })?;
                    let opened = directory.dir_metadata().map_err(|source| {
                        io_error(
                            "inspect authorized cleanup directory",
                            "<publication-cleanup-directory>",
                            source,
                        )
                    })?;
                    if FileIdentity::from_metadata(&opened) != entry.identity {
                        return Err(conflict("cleanup directory changed after preflight"));
                    }
                    remove_current_children(&directory, &entry.children)?;
                    require_identity(
                        parent,
                        &entry.name,
                        PlannedKind::Directory,
                        entry.identity,
                        "<publication-cleanup-directory>",
                    )?;
                    parent.remove_dir(&entry.name).map_err(|source| {
                        io_error(
                            "remove authorized cleanup directory",
                            "<publication-cleanup-directory>",
                            source,
                        )
                    })?;
                }
            }
        }
        sync_dir(parent, "sync authorized cleanup directory")
    }

    fn retire_committed_receipt(
        parent: &Dir,
        root_basename: &OsStr,
        names: &InternalNames,
        limits: ProjectFilesystemLimits,
        max_journal: usize,
    ) -> Result<(), ProjectFilesystemError> {
        let Some(state) = read_journal(parent, &names.journal, root_basename, max_journal, limits)?
        else {
            return Ok(());
        };
        if !state.committed {
            return Err(conflict("uncommitted journal remained after recovery"));
        }
        if entry_exists(parent, &names.stage, limits)? {
            return Err(conflict("committed publication still has a stage entry"));
        }
        if let Some(previous) = &state.previous {
            remove_tree_subset(parent, &names.backup, previous, limits)?;
        } else if entry_exists(parent, &names.backup, limits)? {
            return Err(conflict("initial publication has an unauthorized backup"));
        }
        remove_journal(parent, &names.journal, &state)
    }

    fn remove_journal(
        parent: &Dir,
        journal_name: &OsStr,
        state: &JournalState,
    ) -> Result<(), ProjectFilesystemError> {
        require_identity(
            parent,
            journal_name,
            PlannedKind::RegularFile,
            state.journal_identity,
            "<publication-journal>",
        )?;
        parent.remove_file(journal_name).map_err(|source| {
            io_error(
                "remove authorized publication journal",
                "<publication-journal>",
                source,
            )
        })?;
        sync_dir(parent, "sync parent after publication journal removal")
    }

    fn require_internal_absent(
        parent: &Dir,
        names: &InternalNames,
        limits: ProjectFilesystemLimits,
    ) -> Result<(), ProjectFilesystemError> {
        if entry_exists(parent, &names.stage, limits)?
            || entry_exists(parent, &names.backup, limits)?
            || entry_exists(parent, &names.journal, limits)?
        {
            return Err(conflict("publication residue remained after recovery"));
        }
        Ok(())
    }

    fn entry_exists(
        parent: &Dir,
        name: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<bool, ProjectFilesystemError> {
        optional_exact_entry_metadata(
            parent,
            name,
            "<publication-entry>",
            &mut ScanBudget::new(limits),
        )
        .map(|entry| entry.is_some())
    }

    fn directory_identity(
        parent: &Dir,
        name: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<Option<FileIdentity>, ProjectFilesystemError> {
        optional_entry_identity(
            parent,
            name,
            "<publication-directory>",
            &mut ScanBudget::new(limits),
        )
    }

    fn validate_root_basename(root_basename: &OsStr) -> Result<(), ProjectFilesystemError> {
        if root_basename.as_bytes().len() > LINUX_NAME_MAX {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "project root basename bytes",
                actual: root_basename.as_bytes().len(),
                limit: LINUX_NAME_MAX,
            });
        }
        let path = Path::new(root_basename);
        let mut components = path.components();
        if !matches!(components.next(), Some(Component::Normal(name)) if name == root_basename)
            || components.next().is_some()
        {
            return Err(unsafe_entry(
                "<project-root>",
                "root basename must be one ordinary path component",
            ));
        }
        Ok(())
    }

    fn open_locator(
        root: &Dir,
        locator: &str,
        limits: ProjectFilesystemLimits,
        scans: &mut ScanBudget,
        identities: &mut BTreeSet<FileIdentity>,
    ) -> Result<PinnedFile, ProjectFilesystemError> {
        let mut current = root
            .try_clone()
            .map_err(|source| io_error("clone opened root", locator, source))?;
        let mut segments = locator.split('/').peekable();
        while let Some(segment) = segments.next() {
            if segments.peek().is_none() {
                return open_regular_file(
                    &current,
                    OsStr::new(segment),
                    locator,
                    limits.project.max_document_bytes,
                    scans,
                    identities,
                );
            }
            current = open_directory_component(&current, OsStr::new(segment), locator, scans)?;
        }
        Err(unsafe_entry(locator, "locator has no file component"))
    }

    fn open_directory_component(
        parent: &Dir,
        name: &OsStr,
        locator: &str,
        scans: &mut ScanBudget,
    ) -> Result<Dir, ProjectFilesystemError> {
        let entry = exact_entry_metadata(parent, name, locator, scans)?;
        if !entry.is_dir() {
            return Err(unsafe_entry(locator, "path component is not a directory"));
        }
        let directory = parent.open_dir_nofollow(name).map_err(|source| {
            io_error(
                "open directory component without following links",
                locator,
                source,
            )
        })?;
        let opened = directory
            .dir_metadata()
            .map_err(|source| io_error("inspect opened directory component", locator, source))?;
        if !opened.is_dir()
            || FileIdentity::from_metadata(&entry) != FileIdentity::from_metadata(&opened)
        {
            return Err(unsafe_entry(
                locator,
                "directory component changed during admission",
            ));
        }
        Ok(directory)
    }

    fn open_regular_file(
        parent: &Dir,
        name: &OsStr,
        locator: &str,
        max_document_bytes: usize,
        scans: &mut ScanBudget,
        identities: &mut BTreeSet<FileIdentity>,
    ) -> Result<PinnedFile, ProjectFilesystemError> {
        let entry = exact_entry_metadata(parent, name, locator, scans)?;
        if !entry.is_file() {
            return Err(unsafe_entry(
                locator,
                "entry is not an ordinary regular file",
            ));
        }

        let options = safe_read_options();
        let file = parent
            .open_with(name, &options)
            .map_err(|source| io_error("open file without following links", locator, source))?;
        let metadata = file
            .metadata()
            .map_err(|source| io_error("inspect opened file", locator, source))?;
        let identity = FileIdentity::from_metadata(&metadata);
        if !metadata.is_file()
            || IdentityMetadataExt::nlink(&metadata) != 1
            || FileIdentity::from_metadata(&entry) != identity
        {
            return Err(unsafe_entry(
                locator,
                "file type, link count or identity changed during admission",
            ));
        }
        if !identities.insert(identity) {
            return Err(unsafe_entry(
                locator,
                "filesystem identity is already bound to another locator",
            ));
        }
        let length =
            usize::try_from(metadata.len()).map_err(|_| ProjectFilesystemError::LimitExceeded {
                resource: "project document bytes",
                actual: usize::MAX,
                limit: max_document_bytes,
            })?;
        if length > max_document_bytes {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "project document bytes",
                actual: length,
                limit: max_document_bytes,
            });
        }
        Ok(PinnedFile {
            file,
            metadata,
            identity,
            locator: locator.to_owned(),
            length,
        })
    }

    fn safe_read_options() -> OpenOptions {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        cap_std::fs::OpenOptionsExt::custom_flags(&mut options, OFlags::NOCTTY.bits() as i32);
        options
    }

    fn consume_pinned(mut pinned: PinnedFile) -> Result<Vec<u8>, ProjectFilesystemError> {
        let current = pinned.file.metadata().map_err(|source| {
            io_error(
                "recheck opened file before reading",
                &pinned.locator,
                source,
            )
        })?;
        if !current.is_file()
            || IdentityMetadataExt::nlink(&current) != 1
            || FileIdentity::from_metadata(&current) != pinned.identity
            || current.len() != pinned.metadata.len()
        {
            return Err(unsafe_entry(
                &pinned.locator,
                "opened file changed immediately before reading",
            ));
        }

        let mut bytes = Vec::new();
        bytes.try_reserve_exact(pinned.length).map_err(|_| {
            ProjectFilesystemError::LimitExceeded {
                resource: "project document allocation",
                actual: pinned.length,
                limit: pinned.length,
            }
        })?;
        bytes.resize(pinned.length, 0);
        pinned
            .file
            .read_exact(&mut bytes)
            .map_err(|source| io_error("read exact opened file bytes", &pinned.locator, source))?;
        let mut extra = [0_u8; 1];
        if pinned
            .file
            .read(&mut extra)
            .map_err(|source| io_error("check opened file end", &pinned.locator, source))?
            != 0
        {
            return Err(unsafe_entry(
                &pinned.locator,
                "file grew beyond its admitted byte length",
            ));
        }
        Ok(bytes)
    }

    fn exact_entry_metadata(
        parent: &Dir,
        expected: &OsStr,
        locator: &str,
        scans: &mut ScanBudget,
    ) -> Result<Metadata, ProjectFilesystemError> {
        optional_exact_entry_metadata(parent, expected, locator, scans)?
            .ok_or_else(|| unsafe_entry(locator, "exact directory-entry spelling is absent"))
    }

    fn optional_exact_entry_metadata(
        parent: &Dir,
        expected: &OsStr,
        locator: &str,
        scans: &mut ScanBudget,
    ) -> Result<Option<Metadata>, ProjectFilesystemError> {
        let entries = parent
            .entries()
            .map_err(|source| io_error("enumerate directory", locator, source))?;
        let mut in_scan = 0_usize;
        let mut found = None;
        for result in entries {
            scans.admit(&mut in_scan)?;
            let entry =
                result.map_err(|source| io_error("read directory entry", locator, source))?;
            if entry.file_name() == expected {
                if found.is_some() {
                    return Err(unsafe_entry(locator, "duplicate exact directory entry"));
                }
                found = Some(
                    entry
                        .full_metadata()
                        .map_err(|source| io_error("inspect directory entry", locator, source))?,
                );
            }
        }
        Ok(found)
    }

    fn checked_total(
        current: usize,
        increment: usize,
        limit: usize,
    ) -> Result<usize, ProjectFilesystemError> {
        let actual =
            current
                .checked_add(increment)
                .ok_or(ProjectFilesystemError::LimitExceeded {
                    resource: "project total bytes",
                    actual: usize::MAX,
                    limit,
                })?;
        if actual > limit {
            return Err(ProjectFilesystemError::LimitExceeded {
                resource: "project total bytes",
                actual,
                limit,
            });
        }
        Ok(actual)
    }

    fn unsafe_entry(locator: &str, reason: &'static str) -> ProjectFilesystemError {
        ProjectFilesystemError::UnsafeEntry {
            locator: locator.to_owned(),
            reason,
        }
    }

    fn io_error(
        operation: &'static str,
        locator: &str,
        source: io::Error,
    ) -> ProjectFilesystemError {
        ProjectFilesystemError::Io {
            operation,
            locator: locator.to_owned(),
            source,
        }
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
        use super::*;
        use crate::content::{
            DefinitionIdentityDocument, ItemStackDocument, ProjectDraft, ProjectReferenceRecord,
            ProjectionDocument,
        };
        use rustix::fs::{Mode, mkfifoat};
        use std::fs;
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
                    coordinate_frame: "global-target-2026-07-28".to_owned(),
                    records: vec![ProjectReferenceRecord::Item {
                        identity: DefinitionIdentityDocument {
                            family: "Item".to_owned(),
                            key: "oteryn:reference.item.publication-unit-proof".to_owned(),
                            revision: "definition-r1".to_owned(),
                        },
                        client_projection: ProjectionDocument::ClientSafe,
                        materializable: true,
                        stack_class: ItemStackDocument::StackCapable,
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
    }
}
