//! Independently retained signed links detect a database restore behind an
//! acknowledged operator result. They never authorize automatic restore replay.
use super::{Authenticated, DurabilityError, StaffTrust};
use oteryn_protocol_oteryn::staff::{MAX_ENVELOPE_BYTES, SignedEnvelope, encode_envelope};
use sqlx::PgConnection;
use std::{fs::OpenOptions, io::Read, io::Write, sync::Arc};

fn prefix(trust: &StaffTrust) -> String {
    format!("staff-link-{}-", trust.environment())
}

impl StaffTrust {
    pub(super) async fn retain(
        self: Arc<Self>,
        auth: Authenticated,
    ) -> Result<(), DurabilityError> {
        tokio::task::spawn_blocking(move || {
            let seq = auth.seq().ok_or(DurabilityError::InvalidStoredState)?;
            let bytes =
                encode_envelope(&auth.envelope).map_err(|_| DurabilityError::InvalidStoredState)?;
            self.retain_link(seq, &bytes)
        })
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
    }

    fn retain_link(&self, seq: u64, bytes: &[u8]) -> Result<(), DurabilityError> {
        let path = self
            .state_directory()
            .join(format!("{}{seq:020}", prefix(self)));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                // A torn write deliberately leaves evidence requiring reconciliation.
                file.write_all(bytes)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                file.sync_all()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut found = Vec::new();
                std::fs::File::open(&path)
                    .and_then(|file| {
                        file.take(MAX_ENVELOPE_BYTES as u64 + 1)
                            .read_to_end(&mut found)
                    })
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if found != bytes {
                    return Err(DurabilityError::InvalidStoredState);
                }
                std::fs::File::open(&path)
                    .and_then(|file| file.sync_all())
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            }
            Err(_) => return Err(DurabilityError::InvalidStoredState),
        }
        #[cfg(unix)]
        std::fs::File::open(self.state_directory())
            .and_then(|directory| directory.sync_all())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(())
    }

    pub(super) async fn retained_tip(self: Arc<Self>) -> Result<u64, DurabilityError> {
        tokio::task::spawn_blocking(move || {
            let mut tip = 0;
            for entry in std::fs::read_dir(self.state_directory())
                .map_err(|_| DurabilityError::InvalidStoredState)?
            {
                let entry = entry.map_err(|_| DurabilityError::InvalidStoredState)?;
                let name = entry.file_name();
                let name = name.to_str().ok_or(DurabilityError::InvalidStoredState)?;
                let Some(seq) = name.strip_prefix(&prefix(&self)) else {
                    continue;
                };
                if seq.len() != 20 || !seq.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let seq = seq
                    .parse::<u64>()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if seq == 0 {
                    return Err(DurabilityError::InvalidStoredState);
                }
                tip = tip.max(seq);
            }
            Ok(tip)
        })
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
    }
}

/// Called on each verified committed chain row, at boot and before extension.
/// Existing retained bytes must agree; missing files are repaired from verified DB evidence.
pub(super) async fn verify_and_retain(
    tx: &mut PgConnection,
    trust: &Arc<StaffTrust>,
    head: i64,
) -> Result<bool, DurabilityError> {
    if trust.clone().retained_tip().await? > head as u64 {
        return Ok(false);
    }
    let mut seq = 0i64;
    loop {
        let rows: Vec<(i64, String, Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT control_seq,principal,canonical,signature FROM game_staff_control_links WHERE environment=$1 AND control_seq>$2 AND control_seq<=$3 ORDER BY control_seq LIMIT 64",
        ).bind(trust.environment()).bind(seq).bind(head).fetch_all(&mut *tx).await?;
        if rows.is_empty() {
            break;
        }
        for (next, principal, canonical, signature) in rows {
            let bytes = encode_envelope(&SignedEnvelope {
                principal,
                canonical,
                signature,
            })
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let pinned = trust.clone();
            tokio::task::spawn_blocking(move || pinned.retain_link(next as u64, &bytes))
                .await
                .map_err(|_| DurabilityError::InvalidStoredState)??;
            seq = next;
        }
    }
    Ok(seq == head)
}
