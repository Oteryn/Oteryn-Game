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
    /// Exact predecessor evidence retained in the successor itself (all zero
    /// only for generation one), so a restart can validate the replaced record.
    pub predecessor_event_id: [u8; 16],
    pub predecessor_issued_at: u64,
    pub issued_at: u64,
    pub issuer_identity: String,
}

/// The one Character-generation guard shared by ordinary transactions and recovery.
#[derive(Debug)]
pub struct CharacterRecoveryStore {
    directory: PathBuf,
    authority_scope_id: String,
    issuer_identity: String,
    generation_guard: Arc<RwLock<()>>,
}

pub struct SealedCharacterRecoveryFence<'a> {
    pub record: CharacterRecoveryFenceV1,
    _generation: RwLockReadGuard<'a, ()>,
    _process: File,
}

pub struct CharacterRecoveryTransition<'a> {
    pub record: CharacterRecoveryFenceV1,
    _generation: RwLockWriteGuard<'a, ()>,
    _process: File,
}

impl CharacterRecoveryStore {
    pub fn open(
        retained_directory: impl AsRef<Path>,
        authority_scope_id: impl Into<String>,
        issuer_identity: impl Into<String>,
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
        Ok(Self {
            directory,
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
            predecessor_event_id: [0; 16],
            predecessor_issued_at: 0,
            issued_at,
            issuer_identity: self.issuer_identity.clone(),
        };
        match self.read_optional()? {
            None if expected_predecessor == 0 => self.replace(&proposed)?,
            // Exact ambiguous re-run: the retained successor carries its evidence.
            Some(current)
                if current.recovery_generation == successor
                    && CharacterRecoveryFenceV1 {
                        predecessor_event_id: [0; 16],
                        predecessor_issued_at: 0,
                        ..current.clone()
                    } == proposed =>
            {
                proposed = current;
            }
            Some(current) if current.recovery_generation == expected_predecessor => {
                self.require_configured_identity(&current)?;
                proposed.predecessor_event_id = current.recovery_event_id;
                proposed.predecessor_issued_at = current.issued_at;
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
        let file = secure_open(&self.directory.join(LOCK_NAME), true, false)?;
        ensure_regular(&file)?;
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
        ensure_regular(&file)?;
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
        ensure_regular(&file)?;
        let result = (|| {
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

fn secure_open(path: &Path, write: bool, create_new: bool) -> Result<File, CharacterRecoveryError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(write)
        .create(write && !create_new)
        .create_new(create_new);
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
        || record.recovery_event_id == [0; 16]
        || record.issued_at == 0
        || (record.predecessor_generation == 0)
            != (record.predecessor_event_id == [0; 16] && record.predecessor_issued_at == 0)
        || (record.predecessor_generation != 0
            && (record.predecessor_event_id == [0; 16] || record.predecessor_issued_at == 0))
    {
        return Err(CharacterRecoveryError::Rejected);
    }
    let hex = |bytes: &[u8; 16]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let value = format!(
        "{MAGIC}\nscope={}\ngeneration={}\nevent={}\npredecessor={}\npredecessor_event={}\npredecessor_issued_at={}\nissued_at={}\nissuer={}\n",
        record.authority_scope_id,
        record.recovery_generation,
        hex(&record.recovery_event_id),
        record.predecessor_generation,
        hex(&record.predecessor_event_id),
        record.predecessor_issued_at,
        record.issued_at,
        record.issuer_identity
    );
    if value.len() > MAX_RECORD_BYTES {
        return Err(CharacterRecoveryError::Rejected);
    }
    Ok(value.into_bytes())
}

fn decode(bytes: &[u8]) -> Result<CharacterRecoveryFenceV1, CharacterRecoveryError> {
    let text = std::str::from_utf8(bytes).map_err(|_| CharacterRecoveryError::Unavailable)?;
    let lines = text
        .strip_suffix('\n')
        .ok_or(CharacterRecoveryError::Unavailable)?
        .split('\n')
        .collect::<Vec<_>>();
    if lines.len() != 9 || lines[0] != MAGIC {
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
    let id = |value: &str| -> Result<[u8; 16], CharacterRecoveryError> {
        if value.len() != 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(CharacterRecoveryError::Unavailable);
        }
        let mut out = [0; 16];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| CharacterRecoveryError::Unavailable)?;
        }
        Ok(out)
    };
    let number = |index: usize, prefix: &str| -> Result<u64, CharacterRecoveryError> {
        field(index, prefix)?
            .parse::<u64>()
            .map_err(|_| CharacterRecoveryError::Unavailable)
    };
    let recovery_event_id = id(field(3, "event=")?)?;
    let predecessor_generation = number(4, "predecessor=")?;
    let predecessor_event_id = id(field(5, "predecessor_event=")?)?;
    let predecessor_issued_at = number(6, "predecessor_issued_at=")?;
    let issued_at = number(7, "issued_at=")?;
    let issuer_identity = field(8, "issuer=")?.to_owned();
    let record = CharacterRecoveryFenceV1 {
        authority_scope_id,
        recovery_generation,
        recovery_event_id,
        predecessor_generation,
        predecessor_event_id,
        predecessor_issued_at,
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
        let path = std::env::temp_dir().join(format!(
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
}
