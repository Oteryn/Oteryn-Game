//! Q16: fresh authenticated admission restores the existing LastPosition through
//! the native House handoff, with explicit admission provenance (no client command).
use super::super::character_last_position::load_scoped_last_position;
use super::super::house_interior::{HouseInteriorSnapshot, load_house_interior_in_transaction};
use super::*;

#[derive(Debug)]
pub struct HouseAdmissionRead {
    pub interior: HouseInteriorSnapshot,
    pub hirelings: Vec<super::super::house_hireling::HirelingSnapshot>,
}

impl DurabilityRoot {
    /// The callback proves current content and collision while the native House
    /// item/ACL locks and the caller's runtime owner remain held. False rolls
    /// back the handoff; uncertain COMMIT keeps recovery on the existing receipt.
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing transfer plus exact saved predecessor and borrowed placement owner"
    )]
    pub async fn commit_saved_house_admission<C, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut request: HouseTransferRequest,
        saved: ScopedLastPosition,
        capabilities: CurrentCapabilityCheck,
        owner: &mut C,
        validate: F,
    ) -> Result<(HouseTransferAck, Vec<u8>), DurabilityError>
    where
        C: Send,
        F: FnOnce(HouseAdmissionRead, &mut C) -> bool + Send + 'static,
    {
        let Some(house) = &saved.house else {
            return Err(DurabilityError::Unavailable);
        };
        if request.command != 0
            || request.ack.house.as_deref() != Some(house.house.house_key())
            || request.house != house.house
            || request.origin != house.origin_channel
            || request.last_position.cell != house.origin
            || request.last_position.world_id != saved.position.record.world_id
            || request.ack.position
                != [
                    saved.position.record.cell.x,
                    saved.position.record.cell.y,
                    i32::from(saved.position.record.cell.floor),
                ]
        {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?.run_with_context(owner, move |holder, deadline, owner| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if !prove_current_incarnation(&mut tx, &node).await? { return Err(DurabilityError::Unavailable); }
                // Native Content -> Character -> Guild -> House -> Items order.
                sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||encode($1,'hex')::uuid::text||':'||encode($2,'hex')::uuid::text,0))")
                    .bind(request.house.world_id().as_bytes().as_slice()).bind(request.origin.as_bytes().as_slice())
                    .execute(&mut *tx).await?;
                if load_scoped_last_position(&mut tx, request.fence.character_id).await? != Some(saved.clone()) {
                    return Err(DurabilityError::Unavailable);
                }
                prepare(&mut tx, &node, &mut request).await?;
                let payload = encode_house_transfer_ack(&request.ack).map_err(|_| DurabilityError::Unavailable)?;
                // 0703 verifies the committed fresh receipt and exact old LastPosition key
                // in addition to every original 0215 source/assignment/admission guard.
                apply(&mut tx, &root, &node, &request, &payload, &capabilities, Some(&saved)).await?;
                let fence = CurrentCharacterItemFence {
                    game_session_id: GameSessionId::decode(&request.ack.session).map_err(|_| DurabilityError::InvalidStoredState)?,
                    connection_generation: crate::foundation::ConnectionGeneration::new(1).map_err(|_| DurabilityError::InvalidStoredState)?,
                    character_lease_generation: request.ack.lease_generation,
                    runtime_scope: request.house.runtime_scope()?,
                    scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(request.ack.scope_generation).map_err(|_| DurabilityError::InvalidStoredState)?,
                    character_id: request.fence.character_id,
                };
                if !super::super::house_hireling::fence::assert_house_inventory_fence(
                    &mut tx, &node, &fence, *request.origin.as_bytes(),
                ).await? { return Err(DurabilityError::Unavailable); }
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||encode($1,'hex')::uuid::text||':'||$2,0))")
                    .bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).execute(&mut *tx).await?;
                // The original equipment read independently proves the active content pin.
                super::super::character_equipment::read_house_equipment_in_transaction(
                    &mut tx, &node, &fence, *request.origin.as_bytes(), request.last_position.server_artifact_digest,
                ).await.map_err(|_| DurabilityError::Unavailable)?;
                let interior = load_house_interior_in_transaction(&mut tx, &request.house,
                    request.ack.scope_generation, &node).await.map_err(|_| DurabilityError::Unavailable)?;
                let hirelings = super::super::house_hireling::read_house_hirelings_in_transaction(&mut tx, &fence).await?;
                if !validate(HouseAdmissionRead { interior, hirelings }, owner) {
                    return Err(DurabilityError::Unavailable);
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok((request.ack, payload))
            })
        }).await
    }
}
