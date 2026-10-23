//! Free tutorial departures retain their command receipt and pending arrival atomically.
//! Progress clears only in the actual placement consume transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::character_tutorial::{TutorialProgressOutcome, tutorial_progress_in_transaction};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::npc_travel::{PendingArrival, read_fenced};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::newhaven_travel::NewhavenRoute;
use oteryn_gameplay_quests::tutorial::{TutorialEvent, tutorial_transition};
use sqlx::Row;

impl DurabilityRoot {
    /// Current fences precede reconciliation too. `None` is a previously consumed or
    /// death-cancelled command, never permission to move again.
    pub async fn commit_newhaven_travel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        arrival: PendingArrival,
        route: NewhavenRoute,
    ) -> Result<Option<PendingArrival>, CharacterProgressionError> {
        if command.game_session_id() != fence.game_session_id
            || arrival.world_id != fence.runtime_scope.world_id()
            || (
                arrival.destination.x,
                arrival.destination.y,
                arrival.destination.floor,
            ) != route.destination()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(error));}
            let previous = sqlx::query("SELECT route,occurrence_id::text FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric(20,0)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            if let Some(previous) = previous {
                let occurrence = super::character_progression::uuid_text(previous.try_get("occurrence_id")?)?;
                if previous.try_get::<String,_>("route")? != route.key() || occurrence != arrival.occurrence.as_bytes() {
                    return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                }
                let pending = read_fenced(&mut tx,&recovery,&node,fence).await.map_err(|_|DurabilityError::InvalidStoredState)?;
                commit_semantic_transaction(tx,deadline).await?;
                return Ok(Ok(pending.filter(|row| row.occurrence == arrival.occurrence)));
            }
            let level: Option<i64> = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let blocked = super::pvp_state::load_pvp_state(&mut tx,fence.character_id).await?
                .is_some_and(|row| row.pz_block_remaining_ms.is_some() || row.kill_block_remaining_ms.is_some());
            let obligation: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let cooldown: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid AND arrived_at > clock_timestamp()-interval '3 seconds')")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if level.and_then(|level|u32::try_from(level).ok()).unwrap_or(0) < route.min_level() || blocked || obligation || cooldown {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            }
            sqlx::query("INSERT INTO game_newhaven_travel_receipts(character_id,game_session_id,command_id,occurrence_id,route) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),encode($4,'hex')::uuid,$5)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).bind(arrival.occurrence.as_bytes().as_slice()).bind(route.key()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(arrival.occurrence.as_bytes().as_slice())
                .bind(arrival.world_id.as_bytes().as_slice()).bind(arrival.destination.x).bind(arrival.destination.y).bind(arrival.destination.floor).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(Some(arrival)))
        })).await?
    }
}

/// Called after an actual placement, under the same root lock as arrival consumption.
pub(super) async fn complete_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    occurrence: super::npc_travel::TravelOccurrence,
) -> Result<(), DurabilityError> {
    let completed = sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=clock_timestamp() WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid AND arrived_at IS NULL")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice()).execute(&mut **tx).await?;
    if completed.rows_affected() == 1
        && let TutorialProgressOutcome::Read(Some(stored)) =
            tutorial_progress_in_transaction(tx, character, None).await?
        && let Some(next) = tutorial_transition(
            true,
            Some(stored.progress),
            TutorialEvent::Departure,
            "none",
            false,
        )
    {
        tutorial_progress_in_transaction(tx, character, Some((stored.version, next.progress)))
            .await?;
    }
    Ok(())
}
