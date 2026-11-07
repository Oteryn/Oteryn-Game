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

async fn seasonal_for_epoch(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Vec<String>, DurabilityError> {
    // The reset owner already resolves 06:00 Europe/Berlin. Read its immutable
    // activated date rather than the process's clock or a client-supplied month.
    let row = sqlx::query("SELECT extract(month FROM activated_at AT TIME ZONE 'Europe/Berlin')::smallint AS month,extract(day FROM activated_at AT TIME ZONE 'Europe/Berlin')::smallint AS day FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    let month = u8::try_from(row.try_get::<i16, _>("month")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let day = u8::try_from(row.try_get::<i16, _>("day")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(
        oteryn_gameplay_foundation::scheduled_events::content_schedule()
            .seasonal_at(month, day)
            .into_iter()
            .map(str::to_owned)
            .collect(),
    )
}

fn announcement(base: String, seasonal: &[String]) -> String {
    let mut parts = Vec::new();
    if !base.is_empty() {
        parts.push(base);
    }
    for key in seasonal {
        let name = match key.as_str() {
            "halloween" => "Halloween",
            "christmas" => "Christmas",
            "piece_of_cake" => "A Piece of Cake",
            "autumn_vintage" => "Annual Autumn Vintage",
            _ => continue,
        };
        parts.push(format!("{name} is active."));
    }
    parts.join(" ")
}

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
        let seasonal = seasonal_for_epoch(tx, world, epoch).await?;
        sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement,seasonal_calendar_revision,seasonal_event_keys) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,1,$5) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
        .bind(!announcement.is_empty()).bind(announcement).bind(seasonal).execute(&mut **tx).await?;
    }
    let stored: (bool, String, i16, Vec<String>) = sqlx::query_as("SELECT active,announcement,seasonal_calendar_revision,seasonal_event_keys FROM game_world_event_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    if (stored.0, stored.1.as_str()) != (!announcement.is_empty(), announcement)
        || stored.2 == 1 && stored.3 != seasonal_for_epoch(tx, world, epoch).await?
        || stored.2 == 0 && !stored.3.is_empty()
        || !matches!(stored.2, 0 | 1)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    super::world_event_outcomes::capture_reset(tx, world, epoch).await?;
    Ok(())
}

impl DurabilityRoot {
    /// The calendar decisions already committed by the current reset, used by
    /// native seasonal NPC producers. A restart cannot activate a new season.
    pub async fn active_world_seasonal_events(
        &self,
        world: WorldId,
        expected_epoch: u64,
    ) -> Result<Vec<String>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT a.reset_epoch::text,e.seasonal_calendar_revision,e.seasonal_event_keys FROM game_world_reset_active a LEFT JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let events = if let Some(row) = row {
                let epoch: u64 = row.try_get::<String, _>(0)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                if epoch != expected_epoch { return Err(DurabilityError::InvalidStoredState); }
                if epoch == 0 { Vec::new() } else {
                    let revision: i16 = row.try_get(1)?;
                    let events: Vec<String> = row.try_get(2)?;
                    if revision == 1 && events != seasonal_for_epoch(&mut tx, world, epoch).await?
                        || revision == 0 && !events.is_empty()
                        || !matches!(revision, 0 | 1) { return Err(DurabilityError::InvalidStoredState); }
                    events
                }
            } else if expected_epoch == 0 { Vec::new() } else { return Err(DurabilityError::InvalidStoredState); };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(events)
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
            let row = sqlx::query("SELECT a.reset_epoch::text,e.active,e.announcement,e.seasonal_calendar_revision,e.seasonal_event_keys FROM game_world_reset_active a LEFT JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
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
                    let revision: i16 = row.try_get(3)?;
                    let seasonal: Vec<String> = row.try_get(4)?;
                    if revision == 1 && seasonal != seasonal_for_epoch(&mut tx, world, epoch).await?
                        || revision == 0 && !seasonal.is_empty()
                        || !matches!(revision, 0 | 1) { return Err(DurabilityError::InvalidStoredState); }
                    let base: String = row.try_get(2)?;
                    if row.try_get::<bool, _>(1)? != !base.is_empty()
                        || base != announcement_for_epoch(epoch).unwrap_or_default() {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let announcement = announcement(base, &seasonal);
                    WorldEventState { reset_epoch: epoch, active: !announcement.is_empty(), announcement }
                }
            } else if expected_epoch == 0 {
                WorldEventState { reset_epoch: 0, active: false, announcement: String::new() }
            } else {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            };
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
