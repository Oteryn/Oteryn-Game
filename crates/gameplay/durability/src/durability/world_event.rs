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
    Ok(())
}

impl DurabilityRoot {
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
            let state = if let Some(row) = row {
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
