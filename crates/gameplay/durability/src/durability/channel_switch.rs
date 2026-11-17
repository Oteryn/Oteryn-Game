//! chsw / GAME-CHANNEL-01 §8-§10: the durable facts of a voluntary Channel switch, read under the
//! session fence: the cooldown left by the server clock, the locks the database can prove
//! (migration 2858) and whether the target is a current Channel of the same World. The guard
//! itself only advances inside the fresh admission receipt insert.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::channel_switch::{
    COOLDOWN_MS, MAX_LOAD_CHANNELS, SwitchLock, SwitchStatus,
};
use sqlx::Row;

/// `$3` NULL reads the locks that do not depend on a target.
pub const PREFLIGHT_SQL: &str = "SELECT COALESCE((SELECT cooldown_left_ms FROM game_channel_switch_status(encode($1,'hex')::uuid,encode($2,'hex')::uuid)),0) AS cooldown_left_ms,\
 game_channel_switch_lock(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) AS lock_reason,\
 $3::bytea IS NULL OR EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_kind=1 AND state=1 AND world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid) AS destination";

/// Characters online per current Channel of the World `$1`, from the durable non-terminal
/// sessions (a house session counts on its origin Channel), in ascending Channel order. No
/// heartbeat and no capacity: the count is what the reconnect journal already proves.
pub const LOAD_SQL: &str = "WITH channels AS (SELECT DISTINCT channel_id FROM game_runtime_scope_assignments \
 WHERE scope_kind=1 AND state=1 AND world_id=encode($1,'hex')::uuid ORDER BY channel_id LIMIT $2) \
 SELECT uuid_send(c.channel_id) AS channel_id, count(s.game_session_id) AS online FROM channels c \
 LEFT JOIN game_durability_reconnect_sessions s ON s.world_id=encode($1,'hex')::uuid AND s.session_state IN(1,2) \
 AND COALESCE(s.runtime_scope_channel_id,s.origin_channel_id)=c.channel_id \
 GROUP BY c.channel_id ORDER BY c.channel_id";

/// The stored facts as one wire status. The cooldown outranks a database lock only when no lock
/// applies; a skull lock counts only against a named target.
pub fn preflight_status(
    cooldown_left_ms: i64,
    lock_reason: Option<&str>,
    destination: bool,
    has_target: bool,
) -> Option<SwitchStatus> {
    let cooldown_left_ms = u32::try_from(cooldown_left_ms).ok()?.min(COOLDOWN_MS);
    let lock = match lock_reason {
        Some("COMBAT") => SwitchLock::Combat,
        Some("SKULL") if has_target => SwitchLock::Skull,
        Some("SKULL") | None => SwitchLock::None,
        Some(_) => return None,
    };
    Some(if lock != SwitchLock::None {
        SwitchStatus {
            lock,
            cooldown_left_ms: 0,
        }
    } else if !destination {
        SwitchStatus {
            lock: SwitchLock::Destination,
            cooldown_left_ms: 0,
        }
    } else if cooldown_left_ms > 0 {
        SwitchStatus {
            lock: SwitchLock::Cooldown,
            cooldown_left_ms,
        }
    } else {
        SwitchStatus::READY
    })
}

impl DurabilityRoot {
    /// The switch facts of the fenced Character in `world`, toward `target` when named.
    /// `Ok(None)` when the session fence is no longer current.
    pub async fn read_channel_switch_preflight(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        world: [u8; 16],
        target: Option<[u8; 16]>,
    ) -> Result<Option<SwitchStatus>, DurabilityError> {
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(None);
                    }
                    let row = sqlx::query(PREFLIGHT_SQL)
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(world.as_slice())
                        .bind(target.as_ref().map(|t| t.as_slice()))
                        .fetch_one(&mut *tx)
                        .await?;
                    let lock: Option<String> = row.try_get("lock_reason")?;
                    let status = preflight_status(
                        row.try_get("cooldown_left_ms")?,
                        lock.as_deref(),
                        row.try_get("destination")?,
                        target.is_some(),
                    )
                    .ok_or(DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(status))
                })
            })
            .await
    }

    /// Command 6251: the online count of each current Channel of `world`. `Ok(None)` when the
    /// session fence is no longer current.
    pub async fn read_channel_load(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        world: [u8; 16],
    ) -> Result<Option<Vec<([u8; 16], u32)>>, DurabilityError> {
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    if assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(None);
                    }
                    let rows = sqlx::query(LOAD_SQL)
                        .bind(world.as_slice())
                        .bind(
                            i64::try_from(MAX_LOAD_CHANNELS)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                        )
                        .fetch_all(&mut *tx)
                        .await?;
                    let mut load = Vec::with_capacity(rows.len());
                    for row in rows {
                        let channel: Vec<u8> = row.try_get("channel_id")?;
                        let online: i64 = row.try_get("online")?;
                        load.push((
                            <[u8; 16]>::try_from(channel.as_slice())
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            u32::try_from(online)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                        ));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(load))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_facts_map_to_one_status() {
        let status = |lock, cooldown_left_ms| SwitchStatus {
            lock,
            cooldown_left_ms,
        };
        assert_eq!(
            preflight_status(0, None, true, true),
            Some(SwitchStatus::READY)
        );
        assert_eq!(
            preflight_status(1_000, None, true, true),
            Some(status(SwitchLock::Cooldown, 1_000))
        );
        assert_eq!(
            preflight_status(1_000, Some("COMBAT"), true, false),
            Some(status(SwitchLock::Combat, 0))
        );
        assert_eq!(
            preflight_status(0, Some("SKULL"), true, true),
            Some(status(SwitchLock::Skull, 0))
        );
        // Without a target the skull lock is not known yet.
        assert_eq!(
            preflight_status(0, Some("SKULL"), true, false),
            Some(SwitchStatus::READY)
        );
        assert_eq!(
            preflight_status(0, None, false, true),
            Some(status(SwitchLock::Destination, 0))
        );
        assert_eq!(preflight_status(0, Some("OTHER"), true, true), None);
        assert_eq!(preflight_status(-1, None, true, true), None);
    }
}
