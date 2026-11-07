//! Immutable reset decision and session-fenced durable announcement observations.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{CharacterProgressionError, assert_item_only_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::WorldId;
use oteryn_gameplay_world_change::scheduled::announcement_for_epoch;
use oteryn_protocol_oteryn::world_event::{
    WorldEventState, decode_world_event_state, encode_world_event_state,
};
use sqlx::{Postgres, Row, Transaction};

/// Called exclusively inside reset finalization, including unknown-commit replay.
pub(super) async fn record_reset(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    is_new: bool,
) -> Result<(), DurabilityError> {
    if epoch == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let announcement = announcement_for_epoch(epoch).unwrap_or_default();
    if is_new {
        sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
        .bind(!announcement.is_empty()).bind(announcement).execute(&mut **tx).await?;
    }
    let stored: (bool, String) = sqlx::query_as("SELECT active,announcement FROM game_world_event_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    if stored != (!announcement.is_empty(), announcement.to_owned()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    capture_calendar(tx, world, epoch).await?;
    super::world_event_outcomes::capture_reset(tx, world, epoch).await?;
    Ok(())
}

/// A pinned world's kill-count rule. Binding is immutable within an occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldChangeCounter {
    pub key: String,
    pub creature: String,
    pub goal: u32,
}

impl DurabilityRoot {
    /// Bind a source-qualified counter using the reconciled world owner. Reusing
    /// a key with a different rule is rejected rather than resetting progress.
    pub async fn bind_world_change_counter(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        epoch: u64,
        rule: WorldChangeCounter,
    ) -> Result<(), DurabilityError> {
        if rule.key.is_empty() || rule.key.len() > 100 || !rule.key.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            || rule.creature.is_empty() || rule.creature.len() > 256 || rule.goal == 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
            if !current { return Err(DurabilityError::InvalidStoredState); }
            sqlx::query("INSERT INTO game_world_change_states(world_id,reset_epoch,change_key,active,creature_key,kill_goal) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,false,$4,$5) ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&rule.key).bind(&rule.creature).bind(i64::from(rule.goal)).execute(&mut *tx).await?;
            let matches: bool = sqlx::query_scalar("SELECT coalesce(creature_key=$4 AND kill_goal=$5,false) FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key=$3")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&rule.key).bind(&rule.creature).bind(i64::from(rule.goal)).fetch_one(&mut *tx).await?;
            if !matches { return Err(DurabilityError::InvalidStoredState); }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(())
        })).await
    }

    /// One occurrence snapshot for NPC, spawn and map projection owners.
    pub async fn world_change_snapshot(&self, world: WorldId, epoch: u64) -> Result<oteryn_gameplay_world_change::daily::DailyWorldChanges, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let (month, day): (i16,i16) = sqlx::query_as("SELECT c.month,c.day FROM game_world_change_calendar c JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE c.world_id=encode($1,'hex')::uuid AND c.reset_epoch=$2::text::numeric")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
            let mut changes = oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), epoch, u8::try_from(month).map_err(|_| DurabilityError::InvalidStoredState)?, u8::try_from(day).map_err(|_| DurabilityError::InvalidStoredState)?);
            changes.their_masters_voice = change_active(&mut tx, world, "their_masters_voice").await?;
            changes.grimvale_full_moon = change_active(&mut tx, world, "grimvale").await?;
            changes.fury_gates = change_active(&mut tx, world, "fury_gates").await?.then_some(changes.fury_exit);
            commit_semantic_transaction(tx, deadline).await?;
            Ok(changes)
        })).await
    }

    /// Advisory read for dialogue and travel previews; mutation owners recheck in transaction.
    pub async fn world_change_active(&self, world: WorldId, key: &str) -> Result<bool, DurabilityError> {
        if key.is_empty() || key.len() > 100 { return Err(DurabilityError::InvalidStoredState); }
        let key = key.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let active = change_active(&mut tx, world, &key).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(active)
        })).await
    }

    /// Read the calendar captured atomically by reset finalization.
    pub async fn world_change_calendar(&self, world: WorldId, epoch: u64) -> Result<(u8, u8), DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
            if epoch == 0 || !current { return Err(DurabilityError::InvalidStoredState); }
            // Upgrade an already-existing epoch from its immutable activation time.
            capture_calendar(&mut tx, world, epoch).await?;
            let (month, day): (i16, i16) = sqlx::query_as("SELECT c.month,c.day FROM game_world_change_calendar c JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE c.world_id=encode($1,'hex')::uuid AND c.reset_epoch=$2::text::numeric")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
            let calendar = (u8::try_from(month).map_err(|_| DurabilityError::InvalidStoredState)?, u8::try_from(day).map_err(|_| DurabilityError::InvalidStoredState)?);
            commit_semantic_transaction(tx, deadline).await?;
            Ok(calendar)
        })).await
    }

    /// Load the boot epoch only when its event decision exists. Unlike the map
    /// identity reader, this refuses nonzero epochs without an event binding;
    /// startup must never synthesize a missing decision or open admission.
    pub async fn active_world_event_epoch(
        &self,
        world: WorldId,
    ) -> Result<Option<u64>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(String, bool)> = sqlx::query_as("SELECT a.reset_epoch::text,a.reset_epoch=0 OR EXISTS(SELECT 1 FROM game_world_event_epochs e WHERE (e.world_id,e.reset_epoch)=(a.world_id,a.reset_epoch)) FROM game_world_reset_active a WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let epoch = row.map(|(epoch, event_bound)| {
                if !event_bound { return Err(DurabilityError::InvalidStoredState); }
                epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(epoch)
        })).await
    }

    /// Commit the current epoch's announcement observation before returning success.
    /// Replays retain the original bytes; a receipt never grants gameplay authority.
    pub async fn query_world_event(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        expected_epoch: u64,
    ) -> Result<Vec<u8>, CharacterProgressionError> {
        if command_id == 0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_item_only_fence(&mut tx, &fence, &node).await? {
                return Ok(Err(error));
            }
            // The shared save fence also permits reconnectable sessions. Player
            // commands require an active controller, even when replaying a receipt.
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
                .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !active {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            }
            let world = fence.runtime_scope.world_id();
            // PK lookup is bounded. Character/session locks serialize duplicate commands.
            let receipt = sqlx::query("SELECT result,character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid AS matches FROM game_world_event_query_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command_id.to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                .fetch_optional(&mut *tx).await?;
            if let Some(receipt) = receipt {
                if !receipt.try_get::<bool,_>("matches")? {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let bytes: Vec<u8> = receipt.try_get("result")?;
                decode_world_event_state(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(bytes));
            }
            // The admission relation lock also fences reset publication/activation.
            let row = sqlx::query("SELECT a.reset_epoch::text,e.active,e.announcement FROM game_world_reset_active a LEFT JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let mut state = if let Some(row) = row {
                let epoch = row.try_get::<String,_>(0)?.parse::<u64>()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if epoch != expected_epoch {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                }
                if epoch == 0 {
                    WorldEventState { reset_epoch: 0, active: false, announcement: String::new() }
                } else {
                    WorldEventState { reset_epoch: epoch, active: row.try_get(1)?, announcement: row.try_get(2)? }
                }
            } else if expected_epoch == 0 {
                WorldEventState { reset_epoch: 0, active: false, announcement: String::new() }
            } else {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            };
            if state.announcement != announcement_for_epoch(state.reset_epoch).unwrap_or_default() {
                return Err(DurabilityError::InvalidStoredState);
            }
            if state.reset_epoch != 0 {
                let decisions: Vec<(String, bool)> = sqlx::query_as("SELECT change_key,active FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key IN ('fury_gates','grimvale','their_masters_voice') ORDER BY change_key LIMIT 3")
                    .bind(world.as_bytes().as_slice()).bind(state.reset_epoch.to_string()).fetch_all(&mut *tx).await?;
                if decisions.len() != 3 { return Err(DurabilityError::InvalidStoredState); }
                let mut names = Vec::new();
                for (key, active) in decisions {
                    if active {
                        names.push(match key.as_str() {
                            "fury_gates" => "Fury Gates",
                            "grimvale" => "Grimvale",
                            "their_masters_voice" => "Their Master's Voice",
                            _ => return Err(DurabilityError::InvalidStoredState),
                        });
                    }
                }
                if !names.is_empty() {
                    if !state.announcement.is_empty() { state.announcement.push(' '); }
                    state.announcement.push_str("World changes: ");
                    state.announcement.push_str(&names.join(", "));
                    state.announcement.push('.');
                    state.active = true;
                }
            }
            let bytes = encode_world_event_state(&state).map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_world_event_query_receipts(game_session_id,command_id,character_id,world_id,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command_id.to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                .bind(&bytes).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(bytes))
        })).await?
    }
}

/// Read the active occurrence only; old epochs can never grant current progress.
pub(super) async fn change_active(tx: &mut Transaction<'_, Postgres>, world: WorldId, key: &str) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_change_states s JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE s.world_id=encode($1,'hex')::uuid AND s.change_key=$2 AND s.active)")
        .bind(world.as_bytes().as_slice()).bind(key).fetch_one(&mut **tx).await?)
}

/// Composed with the existing sealed lethal-occurrence receipt. Replay returns
/// before this hook; rollback of any award also rolls back the global counter.
pub(super) async fn capture_kill<const N: usize>(
    tx: &mut Transaction<'_, Postgres>,
    input: &super::party_kill_xp::PartyKillXpRequest<N>,
) -> Result<(), DurabilityError> {
    let Some(member) = input.members.first() else { return Err(DurabilityError::InvalidStoredState); };
    let world = member.fence.runtime_scope.world_id();
    // Stable row order across Channels sharing a World. Goals saturate so an
    // arbitrarily long-lived occurrence cannot overflow or repeatedly flip.
    sqlx::query("WITH locked AS (SELECT s.world_id,s.reset_epoch,s.change_key FROM game_world_change_states s JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE s.world_id=encode($1,'hex')::uuid AND s.creature_key=$2 ORDER BY s.change_key FOR UPDATE OF s) UPDATE game_world_change_states s SET counter=s.counter+CASE WHEN s.counter<s.kill_goal THEN 1 ELSE 0 END,active=(s.counter>=s.kill_goal-1) FROM locked l WHERE (s.world_id,s.reset_epoch,s.change_key)=(l.world_id,l.reset_epoch,l.change_key)")
        .bind(world.as_bytes().as_slice()).bind(&input.creature).execute(&mut **tx).await?;
    Ok(())
}

async fn capture_calendar(tx: &mut Transaction<'_, Postgres>, world: WorldId, epoch: u64) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_world_change_calendar(world_id,reset_epoch,month,day) SELECT world_id,reset_epoch,EXTRACT(MONTH FROM activated_at AT TIME ZONE 'UTC')::smallint,EXTRACT(DAY FROM activated_at AT TIME ZONE 'UTC')::smallint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&mut **tx).await?;
    let (month, day): (i16,i16) = sqlx::query_as("SELECT month,day FROM game_world_change_calendar WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    let month = u8::try_from(month).map_err(|_| DurabilityError::InvalidStoredState)?;
    let day = u8::try_from(day).map_err(|_| DurabilityError::InvalidStoredState)?;
    let changes = oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), epoch, month, day);
    for (key, active) in [("their_masters_voice", changes.their_masters_voice), ("grimvale", changes.grimvale_full_moon), ("fury_gates", changes.fury_gates.is_some())] {
        sqlx::query("INSERT INTO game_world_change_states(world_id,reset_epoch,change_key,active) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(key).bind(active).execute(&mut **tx).await?;
    }
    Ok(())
}
