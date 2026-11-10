//! Node-local ANL custody: sync the unchanged envelope before its source acknowledgement.
use super::super::config::AuditCollectorConfig;
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::io;

#[derive(Clone, Copy)]
enum Source {
    Bazaar,
    AccountClosure,
    T94,
}
impl Source {
    fn name(self) -> &'static str {
        match self {
            Self::Bazaar => "bazaar",
            Self::AccountClosure => "account_closure",
            Self::T94 => "t94",
        }
    }
}

pub(super) struct Collector<'a, 'f, 's> {
    pub(super) root: &'a DurabilityRoot,
    pub(super) authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(super) proof: &'a NodeIncarnationProof,
    pub(super) config: &'a AuditCollectorConfig,
    pub(super) t94_key_id: Option<String>,
}
impl Collector<'_, '_, '_> {
    pub(super) async fn run(&self, stop: &CancellationToken) {
        if !self.config.enabled {
            return;
        }
        let interval = Duration::from_millis(self.config.interval_ms);
        let mut delays = [interval; 3];
        let mut due = [tokio::time::Instant::now(); 3];
        while !stop.is_cancelled() {
            for (index, source) in [Source::Bazaar, Source::AccountClosure, Source::T94]
                .into_iter()
                .enumerate()
            {
                if stop.is_cancelled() {
                    return;
                }
                if tokio::time::Instant::now() < due[index] {
                    continue;
                }
                // Finish an in-flight write/ack before observing shutdown. No remote IO.
                if self.tick(source).await.is_ok() {
                    delays[index] = interval;
                } else {
                    event(&format!(
                        "event=audit_collector source={} result=retry",
                        source.name()
                    ));
                    delays[index] = delays[index].saturating_mul(2).min(Duration::from_secs(60));
                }
                due[index] = tokio::time::Instant::now() + delays[index];
            }
            let next = due
                .into_iter()
                .min()
                .unwrap_or_else(tokio::time::Instant::now);
            if first(tokio::time::sleep_until(next), stop.cancelled())
                .await
                .is_none()
            {
                return;
            }
        }
    }

    async fn tick(&self, source: Source) -> io::Result<()> {
        let unavailable = |_| io::Error::other("audit source unavailable");
        match source {
            Source::Bazaar => {
                let records = self
                    .root
                    .read_bazaar_audit(self.authority, self.proof, self.config.batch_size)
                    .await
                    .map_err(unavailable)?;
                for record in records {
                    self.append(
                        source,
                        &super::super::uuid_text(&record.event_id),
                        &record.envelope,
                        None,
                    )
                    .await?;
                    if !self
                        .root
                        .acknowledge_bazaar_audit(
                            self.authority,
                            self.proof,
                            record.event_id,
                            record.envelope_digest,
                        )
                        .await
                        .map_err(unavailable)?
                    {
                        return Err(io::Error::other("audit acknowledgement refused"));
                    }
                }
            }
            Source::AccountClosure => {
                let records = self
                    .root
                    .pending_account_closure_audit(self.authority, self.config.batch_size)
                    .await
                    .map_err(unavailable)?;
                for record in records {
                    self.append(
                        source,
                        &super::super::uuid_text(&record.event_id),
                        &record.envelope,
                        None,
                    )
                    .await?;
                    self.root
                        .acknowledge_account_closure_audit(
                            self.authority,
                            self.proof,
                            record.event_id,
                        )
                        .await
                        .map_err(unavailable)?;
                }
            }
            Source::T94 => {
                // The inherited API returns one record, already ordered by delivery_order.
                for _ in 0..self.config.batch_size {
                    let Some(record) = self
                        .root
                        .t94_next_audit(self.authority)
                        .await
                        .map_err(unavailable)?
                    else {
                        break;
                    };
                    let sequence = serde_json::json!({"publication_id": record.id, "projection_epoch": record.epoch.to_string(), "projection_revision": record.revision.to_string()});
                    self.append(
                        source,
                        &record.audit_event_id,
                        &record.audit,
                        Some(sequence),
                    )
                    .await?;
                    if !self
                        .root
                        .t94_ack_audit(
                            self.authority,
                            &record.audit_event_id,
                            &Sha256::digest(&record.audit),
                        )
                        .await
                        .map_err(unavailable)?
                    {
                        return Err(io::Error::other("audit acknowledgement refused"));
                    }
                }
            }
        }
        Ok(())
    }

    async fn append(
        &self,
        source: Source,
        event_id: &str,
        envelope: &[u8],
        sequence: Option<serde_json::Value>,
    ) -> io::Result<()> {
        let recorded_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_millis()
            .to_string();
        let key_id = if matches!(source, Source::T94) {
            self.t94_key_id.as_deref()
        } else {
            None
        };
        let mut line = serde_json::to_vec(
            &serde_json::json!({"format_version": 1, "source": source.name(), "event_id": event_id, "sequence": sequence, "recorded_at": recorded_at, "envelope_base64": STANDARD.encode(envelope), "producer_key_id": key_id}),
        )?;
        line.push(b'\n');
        let config = self.config.clone();
        oteryn_task_supervisor::spawn_blocking_lifecycle_supervised(
            "audit_journal_append",
            None,
            move || append_synced(&config, source.name(), &line),
        )
        .await
        .map_err(io::Error::other)?
    }
}

// Runs on Tokio's blocking pool: journal IO must not stall gameplay or supervision.
fn append_synced(config: &AuditCollectorConfig, source: &str, line: &[u8]) -> io::Result<()> {
    use std::fs::{DirBuilder, File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

    if line.len() as u64 > config.rotation_bytes {
        return Err(io::Error::other("audit record exceeds rotation size"));
    }
    match DirBuilder::new().mode(0o700).create(&config.directory) {
        Ok(()) => (),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error),
    }
    if !std::fs::symlink_metadata(&config.directory)?
        .file_type()
        .is_dir()
    {
        return Err(io::Error::other("audit directory is not a directory"));
    }
    // The configured parent must exist. Sync its directory entry even after a
    // prior create followed by a failed write, before transferring custody.
    let parent = config
        .directory
        .parent()
        .ok_or_else(|| io::Error::other("audit directory parent missing"))?;
    File::open(parent)?.sync_all()?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::NOFOLLOW).bits() as i32)
        .open(&config.directory)?;
    if directory.metadata()?.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::other("audit directory must be private"));
    }
    // Serialize append/repair/rotation even if another local process uses this directory.
    rustix::fs::flock(
        &directory,
        rustix::fs::FlockOperation::NonBlockingLockExclusive,
    )?;
    let path = |generation| {
        config.directory.join(if generation == 0 {
            format!("{source}.jsonl")
        } else {
            format!("{source}.{generation}.jsonl")
        })
    };
    let open = || {
        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(path(0))
    };
    let mut file = open()?;
    if !file.metadata()?.is_file() || file.metadata()?.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::other("audit journal is not a regular file"));
    }
    // Prune generations retained by an earlier, larger rotation_count. The config maximum is 32.
    for generation in config.rotation_count..32 {
        match std::fs::remove_file(path(generation)) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    let mut length = repair_tail(&mut file)?;
    if length > config.rotation_bytes.saturating_sub(line.len() as u64) {
        drop(file);
        if config.rotation_count == 1 {
            std::fs::remove_file(path(0))?;
        } else {
            // Rename replaces the oldest retained generation; no temporary file is needed.
            for generation in (1..config.rotation_count).rev() {
                match std::fs::rename(path(generation - 1), path(generation)) {
                    Ok(()) => (),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                    Err(error) => return Err(error),
                }
            }
        }
        directory.sync_all()?;
        file = open()?;
        length = 0;
    }
    file.seek(SeekFrom::Start(length))?;
    file.write_all(line)?;
    file.sync_all()?;
    // Covers file creation and rotation, including a create from an earlier failed attempt.
    directory.sync_all()
}

// A killed/failed write can leave an incomplete JSONL tail. Preserve every
// complete line and remove only the trailing bytes before retrying the source.
fn repair_tail(file: &mut std::fs::File) -> io::Result<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let length = file.metadata()?.len();
    let mut end = length;
    let mut buffer = [0; 4096];
    // The largest source record is <512 KiB after base64/metadata; bound recovery IO too.
    let floor = length.saturating_sub(512 * 1024);
    while end > floor {
        let start = end.saturating_sub(buffer.len() as u64).max(floor);
        let count = (end - start) as usize;
        file.seek(SeekFrom::Start(start))?;
        file.read_exact(&mut buffer[..count])?;
        if let Some(index) = buffer[..count].iter().rposition(|byte| *byte == b'\n') {
            let complete = start + index as u64 + 1;
            if complete != length {
                file.set_len(complete)?;
                file.sync_all()?;
            }
            return Ok(complete);
        }
        end = start;
    }
    if floor > 0 {
        return Err(io::Error::other(
            "audit journal has an oversized incomplete tail",
        ));
    }
    if length != 0 {
        file.set_len(0)?;
        file.sync_all()?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests;
