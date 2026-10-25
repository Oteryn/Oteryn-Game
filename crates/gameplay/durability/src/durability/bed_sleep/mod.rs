//! Same-transaction House sleeper and terminal session owner (BED-1d).
//!
//! This is the durable participant, not a map/reservation or offline-training
//! authority. A House runtime must supply its resolved head, native sex and
//! reserved skill. The Channel composition supplies no such owner. In particular,
//! a stored coordinate or a client handle must never construct this context.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::spell_house_abi::HousePresenceProof;
use super::{DurabilityError, DurabilityRoot};
use crate::content::LogicalCell;
use crate::domain::appearance::Sex;
use crate::foundation::AuthenticatedTransportRefV1;

/// Immutable intent from the owning House runtime; all durable fences and ACLs
/// are independently read at commit. These fields do not prove placement.
#[derive(Debug, Clone)]
pub struct BedSleepRequest {
    pub fence: CurrentCharacterItemFence,
    pub transport: AuthenticatedTransportRefV1,
    pub command_id: u64,
    pub house: HouseId,
    pub head: LogicalCell,
    pub sex: Sex,
    pub skill: u8,
    /// Final living actor capture, committed with the terminal sleep. Legacy producers
    /// without a capture cannot receive bed regeneration.
    pub final_position: Option<FinalLastPosition>,
}

/// Available only on an authority with the current House owner context. Keeping
/// the recovery authority borrowed prevents detached historical recovery claims.
pub struct BedSleepOwner<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub recovery: &'a ReconciledCharacterAuthority<'f, 's>,
    pub node: &'a NodeIncarnationProof,
    pub presence: &'a (dyn HousePresenceProof + Sync),
    pub request: BedSleepRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedSleepOutcome {
    Slept,
    Stale,
    Denied,
    Occupied,
    Conflict,
}

impl BedSleepOwner<'_, '_, '_> {
    /// Commit the sleeper, receipt and terminal House session together. A retry
    /// returns the first receipt; a changed intent, stale fence or occupied bed
    /// cannot write. This transaction emits no type-2 audit event.
    pub async fn commit(self) -> Result<BedSleepOutcome, DurabilityError> {
        let request = self.request;
        if request.final_position.is_some_and(|final_position| {
            final_position.write_sequence == 0
                || final_position.record.world_id != request.house.world_id()
                || final_position.record.rest.is_none()
        }) || request.command_id == 0
            || !(1..=6).contains(&request.skill)
            || request.fence.runtime_scope != request.house.runtime_scope()?
            || self.presence.world() != request.house.world_id()
            || self.presence.house_key() != request.house.house_key()
            || self.presence.actor_session() != request.fence.game_session_id
            || self.presence.scope_generation() != request.fence.scope_ownership_generation.get()
        {
            return Ok(BedSleepOutcome::Denied);
        }
        let recovery = self
            .recovery
            .record_for(self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = self.node.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Ok(BedSleepOutcome::Stale);
                    }
                    // Serialize the historical-receipt check with the SQL sleep owner.
                    sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(request.fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                    let replayed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_bed_sleep_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)")
                        .bind(request.fence.game_session_id.as_bytes().as_slice()).bind(request.command_id.to_string()).fetch_one(&mut *tx).await?;
                    let fact = node.fact();
                    let value: String = sqlx::query_scalar(
                        "SELECT game_house_bed_sleep(encode($1,'hex')::uuid,\
                         encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,\
                         $5::text::numeric,$6::text::numeric,$7::text::numeric,\
                         $8,$9::text::numeric,$10,$11,$12,$13,$14,\
                         encode($15,'hex')::uuid,$16::text::numeric)",
                    )
                    .bind(request.fence.game_session_id.as_bytes().as_slice())
                    .bind(request.fence.character_id.as_bytes().as_slice())
                    .bind(request.house.world_id().as_bytes().as_slice())
                    .bind(request.house.house_key())
                    .bind(request.fence.connection_generation.get().to_string())
                    .bind(request.fence.character_lease_generation.to_string())
                    .bind(request.fence.scope_ownership_generation.get().to_string())
                    .bind(request.transport.to_bytes().as_slice())
                    .bind(request.command_id.to_string())
                    .bind(request.head.x)
                    .bind(request.head.y)
                    .bind(request.head.z)
                    .bind(match request.sex {
                        Sex::Female => "female",
                        Sex::Male => "male",
                    })
                    .bind(i32::from(request.skill))
                    .bind(fact.node_id().as_bytes().as_slice())
                    .bind(fact.registration_revision().to_string())
                    .fetch_one(&mut *tx)
                    .await?;
                    let outcome = match value.as_str() {
                        "slept" => BedSleepOutcome::Slept,
                        "stale" => BedSleepOutcome::Stale,
                        "denied" => BedSleepOutcome::Denied,
                        "occupied" => BedSleepOutcome::Occupied,
                        "conflict" => BedSleepOutcome::Conflict,
                        _ => return Err(DurabilityError::InvalidStoredState),
                    };
                    if outcome == BedSleepOutcome::Slept && !replayed
                        && let Some(final_position) = request.final_position
                    {
                        let key = LastPositionKey {
                            character_lease_generation: request.fence.character_lease_generation,
                            write_sequence: final_position.write_sequence,
                        };
                        if upsert_last_position(&mut tx, request.fence.character_id.as_bytes(), key, &final_position.record).await? != LastPositionWrite::Written {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await
    }
}
