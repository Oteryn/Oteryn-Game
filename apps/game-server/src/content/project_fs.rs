//! Secure Linux filesystem capture for canonical editable World projects.
//!
//! This adapter stops at immutable byte capture. Filesystem publication, journalling and crash
//! recovery remain outside this boundary. Non-Linux targets fail before filesystem access until
//! an opened-handle identity API satisfies the source profile there.

use super::{ProjectError, ProjectEvidenceLimits, WorldProject};
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

#[cfg(target_os = "linux")]
mod linux {
    use super::{ProjectFilesystemError, ProjectFilesystemLimits};
    use crate::content::project::{
        LOCK_LOCATOR, MANIFEST_LOCATOR, PROJECT_LOCATOR, ProjectCapturePlan,
    };
    use crate::content::{ProjectSnapshot, WorldProject, world_project_sha256};
    use cap_fs_ext::{
        DirEntryExt, DirExt, FollowSymlinks, MetadataExt as IdentityMetadataExt,
        OpenOptionsFollowExt, OpenOptionsSyncExt,
    };
    use cap_std::ambient_authority;
    use cap_std::fs::{Dir, File, Metadata, OpenOptions};
    use rustix::fs::OFlags;
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::OsStr;
    use std::io::{self, Read};
    use std::path::{Component, Path};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    fn validate_root_basename(root_basename: &OsStr) -> Result<(), ProjectFilesystemError> {
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
        found.ok_or_else(|| unsafe_entry(locator, "exact directory-entry spelling is absent"))
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
        use rustix::fs::{Mode, mkfifoat};
        use std::fs;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{Duration, Instant};

        static SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
