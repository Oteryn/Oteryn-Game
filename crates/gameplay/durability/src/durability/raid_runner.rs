//! Fenced durable execution of a channel raid run.
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidDefinition, RaidRunAction, RaidRunCursor};
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommittedRaidAction {
    Announcement { text: String, advance: bool },
    Wave { wave: u16, inserted: u32 },
    Complete,
    Lost,
}

impl DurabilityRoot {
    /// Advances at most one action. The channel assignment row is the independent
    /// current-authority fact; the persisted run generation is only expected evidence.
    pub async fn advance_raid_run(
        &self,
        firing_id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        definition: RaidDefinition,
        now_ms: i64,
    ) -> Result<Option<CommittedRaidAction>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let current: Option<String> = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=(SELECT world_id FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid) AND channel_id=encode($2,'hex')::uuid AND assignment_state=2")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if current.as_deref() != Some(&generation.get().to_string()) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let firing = sqlx::query("SELECT fired_at_ms,state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).fetch_optional(&mut *tx).await?;
            let Some(firing) = firing else { commit_semantic_transaction(tx, deadline).await?; return Ok(None); };
            if firing.try_get::<i16,_>("state")? != 1 { commit_semantic_transaction(tx, deadline).await?; return Ok(None); }
            let fired_at: i64 = firing.try_get("fired_at_ms")?;
            let row = sqlx::query("SELECT owner_generation::text,committed_steps,state FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let (steps, state) = if let Some(row) = row {
                let owner: String = row.try_get("owner_generation")?;
                let state: i16 = row.try_get("state")?;
                if owner != generation.get().to_string() {
                    if state == 1 {
                        sqlx::query("UPDATE game_world_raid_channel_runs SET state=3,updated_at_ms=$3 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                            .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(now_ms).execute(&mut *tx).await?;
                        sqlx::query("UPDATE game_world_raid_creatures SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
                            .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut *tx).await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Some(CommittedRaidAction::Lost));
                }
                (u32::try_from(row.try_get::<i32,_>("committed_steps")?).map_err(|_| DurabilityError::InvalidStoredState)?, state)
            } else {
                sqlx::query("INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4)")
                    .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string()).bind(now_ms).execute(&mut *tx).await?;
                (0, 1)
            };
            if state != 1 { commit_semantic_transaction(tx, deadline).await?; return Ok(None); }
            let cursor = RaidRunCursor { committed_steps: steps };
            let Some(step) = cursor.next_due(&definition, fired_at, now_ms).map_err(|_| DurabilityError::InvalidStoredState)? else {
                commit_semantic_transaction(tx, deadline).await?; return Ok(None);
            };
            let (kind, result) = match &step.action {
                RaidRunAction::Announce { text, kind } => (1_i16, CommittedRaidAction::Announcement { text: (*text).to_owned(), advance: matches!(kind, oteryn_gameplay_encounter_rules::raid::AnnouncementKind::Advance) }),
                RaidRunAction::SpawnWave { wave, entries, .. } => {
                    let mut inserted = 0_u32;
                    for (entry, value) in entries.iter().enumerate() {
                        for index in 0..value.count {
                            let done = sqlx::query("INSERT INTO game_world_raid_creatures(firing_id,channel_id,wave,entry,creature_index,creature_key,anchor_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
                                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(i16::try_from(*wave).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i16::try_from(entry).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i16::try_from(index).map_err(|_| DurabilityError::InvalidStoredState)?).bind(value.creature.as_str()).bind(value.area_anchor.as_str()).execute(&mut *tx).await?;
                            inserted = inserted.saturating_add(u32::try_from(done.rows_affected()).unwrap_or(0));
                        }
                    }
                    (2, CommittedRaidAction::Wave { wave: *wave, inserted })
                }
                RaidRunAction::Complete => {
                    sqlx::query("UPDATE game_world_raid_creatures SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
                        .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut *tx).await?;
                    (3, CommittedRaidAction::Complete)
                }
            };
            let next = cursor.commit(&step).map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_world_raid_step_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(i32::try_from(step.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?).bind(kind).bind(now_ms).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=$3,state=$4,updated_at_ms=$5 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$6::text::numeric")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(i32::try_from(next.committed_steps).map_err(|_| DurabilityError::InvalidStoredState)?).bind(if matches!(result, CommittedRaidAction::Complete) {2_i16} else {1_i16}).bind(now_ms).bind(generation.get().to_string()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(result))
        })).await
    }

    pub async fn due_raid_firings(
        &self,
        channel: ChannelId,
    ) -> Result<Vec<([u8; 16], ProductionKey)>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let rows = sqlx::query("SELECT decode(replace(f.firing_id::text,'-',''),'hex'),f.raid_key FROM game_world_raid_firings f JOIN game_runtime_scope_assignments a ON a.world_id=f.world_id AND a.channel_id=encode($1,'hex')::uuid AND a.assignment_state=2 WHERE f.state=1 ORDER BY f.fired_at_ms,f.firing_id LIMIT 32")
                .bind(channel.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut out=Vec::with_capacity(rows.len());
            for row in rows {
                let id: Vec<u8> = row.try_get(0)?;
                let bytes = id
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let raw: String = row.try_get(1)?;
                let key = ProductionKey::new(&raw)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                out.push((bytes, key));
            }
            commit_semantic_transaction(tx, deadline).await?; Ok(out)
        })).await
    }
}
