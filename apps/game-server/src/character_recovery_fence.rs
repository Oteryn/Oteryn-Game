//! Character-specific, non-rollback recovery register retained outside PostgreSQL.

use rustix::fs::{FlockOperation, OFlags};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

const RECORD_NAME: &str = "character-recovery-fence-v1.record";
const LOCK_NAME: &str = "character-recovery-fence-v1.lock";
const MAGIC: &str = "CHARACTER_RESTORE_NONROLLBACK_FENCE_V1";
const MAX_RECORD_BYTES: usize = 1024;
const MAX_ID_BYTES: usize = 128;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum CharacterRecoveryError {
    Rejected,
    Conflict,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRecoveryFenceV1 {
    pub authority_scope_id: String,
    pub recovery_generation: u64,
    pub recovery_event_id: [u8; 16],
    pub predecessor_generation: u64,
    /// SHA-256 of the exact canonical record this one replaced (all zero only
    /// for generation one). The replaced record carries its own predecessor
    /// digest, so the whole chain is bound.
    pub predecessor_digest: [u8; 32],
    pub issued_at: u64,
    pub issuer_identity: String,
}

/// The one Character-generation guard shared by ordinary transactions and recovery.
#[derive(Debug)]
pub struct CharacterRecoveryStore {
    directory: PathBuf,
    /// The service user that owns the directory, lock and record
    /// (OPS-NODE-BOOT-01 D1). Files this process creates are handed to it.
    owner: u32,
    authority_scope_id: String,
    issuer_identity: String,
    generation_guard: Arc<RwLock<()>>,
}

pub struct SealedCharacterRecoveryFence<'a> {
    record: CharacterRecoveryFenceV1,
    _generation: RwLockReadGuard<'a, ()>,
    _process: File,
}

pub struct CharacterRecoveryTransition<'a> {
    record: CharacterRecoveryFenceV1,
    _generation: RwLockWriteGuard<'a, ()>,
    _process: File,
}

impl SealedCharacterRecoveryFence<'_> {
    /// The externally locked record, immutable for the life of the seal.
    #[must_use]
    pub const fn record(&self) -> &CharacterRecoveryFenceV1 {
        &self.record
    }
}

impl CharacterRecoveryTransition<'_> {
    /// The retained successor record, immutable for the life of the transition.
    #[must_use]
    pub const fn record(&self) -> &CharacterRecoveryFenceV1 {
        &self.record
    }
}

impl CharacterRecoveryStore {
    /// Open the store owned by this process's effective user.
    pub fn open(
        retained_directory: impl AsRef<Path>,
        authority_scope_id: impl Into<String>,
        issuer_identity: impl Into<String>,
    ) -> Result<Self, CharacterRecoveryError> {
        Self::open_owned_by(
            retained_directory,
            authority_scope_id,
            issuer_identity,
            rustix::process::geteuid().as_raw(),
        )
    }

    /// Open the store whose directory, lock and record belong to `owner`. The
    /// directory must be owned by `owner` and not writable by group or others,
    /// and its parent must not be writable by group or others. Lock and record
    /// files created here are exclusive, mode 0600 and handed to `owner`
    /// through the open descriptor before any content is written.
    pub fn open_owned_by(
        retained_directory: impl AsRef<Path>,
        authority_scope_id: impl Into<String>,
        issuer_identity: impl Into<String>,
        owner: u32,
    ) -> Result<Self, CharacterRecoveryError> {
        let authority_scope_id = authority_scope_id.into();
        let issuer_identity = issuer_identity.into();
        validate_id(&authority_scope_id)?;
        validate_id(&issuer_identity)?;
        let directory = retained_directory.as_ref();
        let metadata =
            fs::symlink_metadata(directory).map_err(|_| CharacterRecoveryError::Unavailable)?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(CharacterRecoveryError::Unavailable);
        }
        let directory = directory
            .canonicalize()
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        let handle = secure_open(&directory, false, false)?;
        let stat = rustix::fs::fstat(&handle).map_err(|_| CharacterRecoveryError::Unavailable)?;
        if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::Directory
            || stat.st_uid != owner
            || stat.st_mode & 0o022 != 0
        {
            return Err(CharacterRecoveryError::Unavailable);
        }
        let parent = File::open(
            directory
                .parent()
                .ok_or(CharacterRecoveryError::Unavailable)?,
        )
        .map_err(|_| CharacterRecoveryError::Unavailable)?;
        let stat = rustix::fs::fstat(&parent).map_err(|_| CharacterRecoveryError::Unavailable)?;
        if (stat.st_uid != owner && stat.st_uid != 0) || stat.st_mode & 0o022 != 0 {
            return Err(CharacterRecoveryError::Unavailable);
        }
        Ok(Self {
            directory,
            owner,
            authority_scope_id,
            issuer_identity,
            generation_guard: Arc::new(RwLock::new(())),
        })
    }

    /// Seal the authenticated external record for the complete lifetime of a DB transaction.
    pub fn seal_current(&self) -> Result<SealedCharacterRecoveryFence<'_>, CharacterRecoveryError> {
        let generation = self
            .generation_guard
            .read()
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        let process = self.lock(FlockOperation::LockShared)?;
        let record = self.read_exact()?;
        self.require_configured_identity(&record)?;
        Ok(SealedCharacterRecoveryFence {
            record,
            _generation: generation,
            _process: process,
        })
    }

    /// Explicitly authorize generation one for a genuinely empty Character store.
    pub fn authorize_fresh_store(
        &self,
        recovery_event_id: [u8; 16],
        issued_at: u64,
    ) -> Result<CharacterRecoveryTransition<'_>, CharacterRecoveryError> {
        self.begin_transition(0, recovery_event_id, issued_at)
    }

    /// Advance exactly one generation. A matching successor is reconciliation, not a retry.
    pub fn begin_recovery(
        &self,
        expected_predecessor: u64,
        recovery_event_id: [u8; 16],
        issued_at: u64,
    ) -> Result<CharacterRecoveryTransition<'_>, CharacterRecoveryError> {
        if expected_predecessor == 0 {
            return Err(CharacterRecoveryError::Rejected);
        }
        self.begin_transition(expected_predecessor, recovery_event_id, issued_at)
    }

    fn begin_transition(
        &self,
        expected_predecessor: u64,
        recovery_event_id: [u8; 16],
        issued_at: u64,
    ) -> Result<CharacterRecoveryTransition<'_>, CharacterRecoveryError> {
        // Only a canonical UUIDv7 (version 7, RFC variant) is admissible in the
        // Character store; reject before the external register can advance.
        if recovery_event_id[6] >> 4 != 7 || recovery_event_id[8] & 0xc0 != 0x80 || issued_at == 0 {
            return Err(CharacterRecoveryError::Rejected);
        }
        let successor = expected_predecessor
            .checked_add(1)
            .ok_or(CharacterRecoveryError::Rejected)?;
        let generation = self
            .generation_guard
            .write()
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        let process = self.lock(FlockOperation::LockExclusive)?;
        let mut proposed = CharacterRecoveryFenceV1 {
            authority_scope_id: self.authority_scope_id.clone(),
            recovery_generation: successor,
            recovery_event_id,
            predecessor_generation: expected_predecessor,
            predecessor_digest: [0; 32],
            issued_at,
            issuer_identity: self.issuer_identity.clone(),
        };
        match self.read_optional()? {
            None if expected_predecessor == 0 => self.replace(&proposed)?,
            // Exact ambiguous re-run: the retained successor carries its evidence.
            Some(current)
                if current.recovery_generation == successor
                    && CharacterRecoveryFenceV1 {
                        predecessor_digest: [0; 32],
                        ..current.clone()
                    } == proposed =>
            {
                proposed = current;
            }
            Some(current) if current.recovery_generation == expected_predecessor => {
                self.require_configured_identity(&current)?;
                proposed.predecessor_digest = recovery_record_digest(&current)?;
                self.replace(&proposed)?;
            }
            Some(_) => return Err(CharacterRecoveryError::Conflict),
            None => return Err(CharacterRecoveryError::Conflict),
        }
        if self.read_exact()? != proposed {
            return Err(CharacterRecoveryError::Unavailable);
        }
        Ok(CharacterRecoveryTransition {
            record: proposed,
            _generation: generation,
            _process: process,
        })
    }

    fn require_configured_identity(
        &self,
        record: &CharacterRecoveryFenceV1,
    ) -> Result<(), CharacterRecoveryError> {
        if record.authority_scope_id != self.authority_scope_id
            || record.issuer_identity != self.issuer_identity
        {
            return Err(CharacterRecoveryError::Conflict);
        }
        Ok(())
    }

    fn lock(&self, operation: FlockOperation) -> Result<File, CharacterRecoveryError> {
        let path = self.directory.join(LOCK_NAME);
        let file = match secure_open(&path, true, false) {
            Ok(file) => file,
            Err(_) if !path.exists() && fs::symlink_metadata(&path).is_err() => {
                // Fresh installation: exclusive create, handed to the owner.
                let file = match secure_open(&path, true, true) {
                    Ok(file) => file,
                    Err(_) => secure_open(&path, true, false)?,
                };
                self.hand_off(&file)?;
                file
            }
            Err(error) => return Err(error),
        };
        self.ensure_owned(&file)?;
        rustix::fs::flock(&file, operation).map_err(|_| CharacterRecoveryError::Unavailable)?;
        Ok(file)
    }

    fn read_optional(&self) -> Result<Option<CharacterRecoveryFenceV1>, CharacterRecoveryError> {
        let path = self.directory.join(RECORD_NAME);
        let file = match secure_open(&path, false, false) {
            Ok(file) => file,
            Err(CharacterRecoveryError::Unavailable) if !path.exists() => return Ok(None),
            Err(error) => return Err(error),
        };
        self.ensure_owned(&file)?;
        let mut bytes = Vec::new();
        file.take((MAX_RECORD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CharacterRecoveryError::Unavailable);
        }
        decode(&bytes).map(Some)
    }

    fn read_exact(&self) -> Result<CharacterRecoveryFenceV1, CharacterRecoveryError> {
        self.read_optional()?
            .ok_or(CharacterRecoveryError::Unavailable)
    }

    fn replace(&self, record: &CharacterRecoveryFenceV1) -> Result<(), CharacterRecoveryError> {
        let bytes = encode(record)?;
        let temp_name = format!(
            ".character-recovery-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let temp = self.directory.join(&temp_name);
        let mut file = secure_open(&temp, true, true)?;
        let result = (|| {
            self.hand_off(&file)?;
            self.ensure_owned(&file)?;
            file.write_all(&bytes)
                .map_err(|_| CharacterRecoveryError::Unavailable)?;
            file.sync_all()
                .map_err(|_| CharacterRecoveryError::Unavailable)?;
            fs::rename(&temp, self.directory.join(RECORD_NAME))
                .map_err(|_| CharacterRecoveryError::Unavailable)?;
            File::open(&self.directory)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| CharacterRecoveryError::Unavailable)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

impl CharacterRecoveryStore {
    /// Transfer a newly created file to the owner through its descriptor and
    /// synchronize the directory entry.
    fn hand_off(&self, file: &File) -> Result<(), CharacterRecoveryError> {
        rustix::fs::fchown(file, Some(rustix::fs::Uid::from_raw(self.owner)), None)
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        file.sync_all()
            .map_err(|_| CharacterRecoveryError::Unavailable)?;
        File::open(&self.directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| CharacterRecoveryError::Unavailable)
    }

    /// A regular file owned by the store owner, not writable by group or others.
    fn ensure_owned(&self, file: &File) -> Result<(), CharacterRecoveryError> {
        ensure_regular(file)?;
        let stat = rustix::fs::fstat(file).map_err(|_| CharacterRecoveryError::Unavailable)?;
        if stat.st_uid != self.owner || stat.st_mode & 0o022 != 0 {
            return Err(CharacterRecoveryError::Unavailable);
        }
        Ok(())
    }
}

/// Open without following a final symbolic link. Created files are mode 0600.
fn secure_open(path: &Path, write: bool, create_new: bool) -> Result<File, CharacterRecoveryError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(write)
        .create_new(create_new)
        .mode(0o600);
    options.custom_flags(OFlags::NOFOLLOW.bits() as i32);
    options
        .open(path)
        .map_err(|_| CharacterRecoveryError::Unavailable)
}

fn ensure_regular(file: &File) -> Result<(), CharacterRecoveryError> {
    if !file
        .metadata()
        .map_err(|_| CharacterRecoveryError::Unavailable)?
        .file_type()
        .is_file()
    {
        return Err(CharacterRecoveryError::Unavailable);
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), CharacterRecoveryError> {
    if value.is_empty()
        || value.len() > MAX_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
    {
        return Err(CharacterRecoveryError::Rejected);
    }
    Ok(())
}

fn encode(record: &CharacterRecoveryFenceV1) -> Result<Vec<u8>, CharacterRecoveryError> {
    validate_id(&record.authority_scope_id)?;
    validate_id(&record.issuer_identity)?;
    if record.recovery_generation == 0
        || record.recovery_generation
            != record
                .predecessor_generation
                .checked_add(1)
                .ok_or(CharacterRecoveryError::Rejected)?
        || record.recovery_event_id[6] >> 4 != 7
        || record.recovery_event_id[8] & 0xc0 != 0x80
        || record.issued_at == 0
        || (record.predecessor_generation == 0) != (record.predecessor_digest == [0; 32])
    {
        return Err(CharacterRecoveryError::Rejected);
    }
    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let value = format!(
        "{MAGIC}\nscope={}\ngeneration={}\nevent={}\npredecessor={}\npredecessor_digest={}\nissued_at={}\nissuer={}\n",
        record.authority_scope_id,
        record.recovery_generation,
        hex(&record.recovery_event_id),
        record.predecessor_generation,
        hex(&record.predecessor_digest),
        record.issued_at,
        record.issuer_identity
    );
    if value.len() > MAX_RECORD_BYTES {
        return Err(CharacterRecoveryError::Rejected);
    }
    Ok(value.into_bytes())
}

/// SHA-256 of a record's exact canonical encoding.
pub fn recovery_record_digest(
    record: &CharacterRecoveryFenceV1,
) -> Result<[u8; 32], CharacterRecoveryError> {
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(encode(record)?).into())
}

fn decode(bytes: &[u8]) -> Result<CharacterRecoveryFenceV1, CharacterRecoveryError> {
    let text = std::str::from_utf8(bytes).map_err(|_| CharacterRecoveryError::Unavailable)?;
    let lines = text
        .strip_suffix('\n')
        .ok_or(CharacterRecoveryError::Unavailable)?
        .split('\n')
        .collect::<Vec<_>>();
    if lines.len() != 8 || lines[0] != MAGIC {
        return Err(CharacterRecoveryError::Unavailable);
    }
    let field = |index: usize, prefix: &str| {
        lines[index]
            .strip_prefix(prefix)
            .filter(|value| !value.is_empty())
            .ok_or(CharacterRecoveryError::Unavailable)
    };
    let authority_scope_id = field(1, "scope=")?.to_owned();
    let recovery_generation = field(2, "generation=")?
        .parse::<u64>()
        .map_err(|_| CharacterRecoveryError::Unavailable)?;
    fn hex_bytes<const N: usize>(value: &str) -> Result<[u8; N], CharacterRecoveryError> {
        if value.len() != N * 2
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(CharacterRecoveryError::Unavailable);
        }
        let mut out = [0; N];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| CharacterRecoveryError::Unavailable)?;
        }
        Ok(out)
    }
    let number = |index: usize, prefix: &str| -> Result<u64, CharacterRecoveryError> {
        field(index, prefix)?
            .parse::<u64>()
            .map_err(|_| CharacterRecoveryError::Unavailable)
    };
    let recovery_event_id = hex_bytes::<16>(field(3, "event=")?)?;
    let predecessor_generation = number(4, "predecessor=")?;
    let predecessor_digest = hex_bytes::<32>(field(5, "predecessor_digest=")?)?;
    let issued_at = number(6, "issued_at=")?;
    let issuer_identity = field(7, "issuer=")?.to_owned();
    let record = CharacterRecoveryFenceV1 {
        authority_scope_id,
        recovery_generation,
        recovery_event_id,
        predecessor_generation,
        predecessor_digest,
        issued_at,
        issuer_identity,
    };
    if encode(&record).map_err(|_| CharacterRecoveryError::Unavailable)? != bytes {
        return Err(CharacterRecoveryError::Unavailable);
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn directory(label: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        // The fence requires a parent that is not writable by group or others.
        let parent = std::env::temp_dir().join(format!(
            "oteryn-character-fences-unit-{}",
            std::process::id()
        ));
        fs::create_dir_all(&parent).expect("fence parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("fence parent mode");
        let path = parent.join(format!(
            "oteryn-character-recovery-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("retained directory");
        path
    }

    fn event(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    #[test]
    fn fresh_authorization_and_ordinary_restart_preserve_generation() {
        let directory = directory("restart");
        {
            let store = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
                .expect("store");
            let transition = store
                .authorize_fresh_store(event(1), 100)
                .expect("fresh authorization");
            assert_eq!(transition.record.recovery_generation, 1);
        }
        let restarted = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
            .expect("restart");
        let sealed = restarted.seal_current().expect("sealed current fence");
        assert_eq!(sealed.record.recovery_generation, 1);
        assert_eq!(sealed.record.recovery_event_id, event(1));
        drop(sealed);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn strict_successor_and_same_event_reconciliation_are_exact() {
        let directory = directory("successor");
        let store = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
            .expect("store");
        drop(
            store
                .authorize_fresh_store(event(1), 100)
                .expect("fresh authorization"),
        );
        let successor = store.begin_recovery(1, event(2), 200).expect("successor");
        assert_eq!(successor.record.recovery_generation, 2);
        drop(successor);
        drop(
            store
                .begin_recovery(1, event(2), 200)
                .expect("ambiguous result reconciles by exact readback"),
        );
        assert!(matches!(
            store.begin_recovery(1, event(3), 200),
            Err(CharacterRecoveryError::Conflict)
        ));
        assert!(matches!(
            store.begin_recovery(0, event(3), 300),
            Err(CharacterRecoveryError::Rejected)
        ));
        assert!(matches!(
            store.begin_recovery(u64::MAX, event(3), 300),
            Err(CharacterRecoveryError::Rejected)
        ));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn retained_record_with_non_rfc_event_id_is_malformed() {
        let directory = directory("variant");
        let store = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
            .expect("store");
        drop(
            store
                .authorize_fresh_store(event(1), 100)
                .expect("fresh authorization"),
        );
        let path = directory.join(RECORD_NAME);
        let valid = fs::read_to_string(&path).expect("retained record");
        // Byte 8 of the event id is `80`; `40` keeps version 7 but breaks the RFC variant.
        let malformed = valid.replacen(
            "event=0102030405067008800a",
            "event=0102030405067008400a",
            1,
        );
        assert_ne!(malformed, valid);
        fs::write(&path, malformed).expect("rewrite record");
        assert!(matches!(
            store.seal_current(),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn missing_malformed_and_nonregular_external_state_fail_closed() {
        let directory = directory("closed");
        let store = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
            .expect("store");
        assert!(matches!(
            store.seal_current(),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::write(directory.join(RECORD_NAME), b"truncated").expect("write malformed record");
        assert!(matches!(
            store.seal_current(),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::remove_file(directory.join(RECORD_NAME)).expect("remove malformed record");
        fs::create_dir(directory.join(RECORD_NAME)).expect("nonregular record");
        assert!(matches!(
            store.seal_current(),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn directory_and_files_require_the_owner_and_no_group_or_other_write() {
        use std::os::unix::fs::PermissionsExt;
        let me = rustix::process::geteuid().as_raw();
        let path = directory("ownership");
        // Created files are mode 0600 and owned by the store owner.
        let store = CharacterRecoveryStore::open(&path, "scope", "issuer").expect("store");
        store
            .authorize_fresh_store(event(1), 1)
            .expect("fresh store");
        for name in [LOCK_NAME, RECORD_NAME] {
            let metadata = fs::metadata(path.join(name)).expect("metadata");
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }
        drop(store);
        // Another owner is refused.
        assert!(matches!(
            CharacterRecoveryStore::open_owned_by(&path, "scope", "issuer", me.wrapping_add(1)),
            Err(CharacterRecoveryError::Unavailable)
        ));
        // A group-writable directory is refused.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o770)).expect("mode");
        assert!(matches!(
            CharacterRecoveryStore::open(&path, "scope", "issuer"),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("mode");
        // A group-writable record is refused before use.
        fs::set_permissions(path.join(RECORD_NAME), fs::Permissions::from_mode(0o620))
            .expect("mode");
        let store = CharacterRecoveryStore::open(&path, "scope", "issuer").expect("store");
        assert!(matches!(
            store.seal_current(),
            Err(CharacterRecoveryError::Unavailable)
        ));
        fs::remove_dir_all(&path).expect("cleanup");
    }
}
