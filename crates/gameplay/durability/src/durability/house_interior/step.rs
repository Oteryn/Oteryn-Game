//! A bounded current House floor read retained through the native movement turn.
//! The same Character -> Guild -> House -> house-items locks as native placement
//! protect missing-row collision checks. A published step may release its
//! original native entry reservation through the existing handoff owner.
use super::*;
use super::super::character_equipment::EquipmentSnapshot;

#[derive(Debug, Clone)]
pub struct HouseStepReadRequest {
    pub fence: CurrentCharacterItemFence,
    pub origin_channel: [u8; 16],
    pub house: HouseId,
    pub content_digest: [u8; 32],
}

#[derive(Debug)]
pub struct HouseStepSnapshot {
    pub interior: HouseInteriorSnapshot,
    pub equipment: EquipmentSnapshot,
    pub hirelings: Vec<super::super::house_hireling::HirelingSnapshot>,
}

impl DurabilityRoot {
    /// `operation` is synchronous while all native current-read locks remain
    /// held. The borrowed owner keeps a published result independently of SQL
    /// cleanup: an unavailable COMMIT cannot undo a native actor step. A true
    /// result means a step was published; only then attempt its entry release.
    pub async fn with_house_step_read<C, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: HouseStepReadRequest,
        owner: &mut C,
        operation: F,
    ) -> DbResult<()>
    where
        C: Send,
        F: FnOnce(HouseStepSnapshot, &mut C) -> bool + Send + 'static,
    {
        if request.house.runtime_scope()? != request.fence.runtime_scope {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run_with_context(owner, move |holder, deadline, owner| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let world = request.house.world_id();
                let house = request.house.house_key();
                // Current Content -> Character -> Guild -> House -> Items,
                // matching native placement even for a refused foreign item.
                sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
                    .bind(world.as_bytes().as_slice()).bind(request.origin_channel.as_slice())
                    .execute(&mut *tx).await?;
                if !super::super::house_hireling::fence::assert_house_inventory_fence(
                    &mut tx, &node, &request.fence, request.origin_channel,
                ).await? { return Err(DurabilityError::Unavailable); }
                // The shared named-House fence above holds the native Guild
                // and property/access locks. Compose only the existing item
                // placement lock; there is no second House ACL authority.
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||encode($1,'hex')::uuid::text||':'||$2,0))")
                    .bind(world.as_bytes().as_slice()).bind(house).execute(&mut *tx).await?;
                let equipment = super::super::character_equipment::read_house_equipment_in_transaction(
                    &mut tx, &node, &request.fence, request.origin_channel, request.content_digest,
                ).await.map_err(|_| DurabilityError::Unavailable)?;
                let interior = load_house_interior_in_transaction(&mut tx, &request.house,
                    request.fence.scope_ownership_generation.get(), &node).await
                    .map_err(|_| DurabilityError::Unavailable)?;
                let hirelings = super::super::house_hireling::read_house_hirelings_in_transaction(
                    &mut tx, &request.fence,
                ).await?;
                if operation(HouseStepSnapshot { interior, equipment, hirelings }, owner) {
                    release_placed_entry(&mut tx, &node, &request.fence).await?;
                }
                // A failed or unknown cleanup may retain the reservation for
                // retry; it cannot undo an already published native step.
                commit_semantic_transaction(tx, deadline).await
            })
        }).await
    }
}

async fn release_placed_entry(
    tx: &mut Transaction<'_, Postgres>, node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
) -> DbResult<()> {
    use super::super::house_scope_handoff::{HouseReleaseOutcome, release_house_entry_tile_in_transaction};
    // The full current House fence and native property locks are still held.
    // A released handoff needs no further writes on subsequent movement turns.
    let Some(row) = sqlx::query("SELECT uuid_send(handoff_id) AS id, \
        floor(extract(epoch FROM clock_timestamp()))::bigint AS now \
        FROM game_house_scope_handoffs WHERE destination_game_session_id=encode($1,'hex')::uuid \
          AND character_id=encode($2,'hex')::uuid AND state=2")
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
    else { return Ok(()); };
    let id: [u8;16] = row.try_get::<Vec<u8>,_>("id")?.try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    match release_house_entry_tile_in_transaction(tx, node, id, row.try_get("now")?).await
        .map_err(|_| DurabilityError::Unavailable)? {
        HouseReleaseOutcome::Released(_) | HouseReleaseOutcome::Replayed(_)
            | HouseReleaseOutcome::NotPlaced => Ok(()),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}
