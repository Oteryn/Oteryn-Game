//! Terminal Ground custody and immutable bundle tombstones for a blocked door step-out.
use super::*;
use crate::durability::{
    character_progression::assert_item_only_fence,
    item_transfer::CurrentCharacterItemFence,
    map_item_mint::MapItemPlacement,
    spell_item_transaction::assert_spell_item_scope_with_recovery,
};
use crate::foundation::MovementLocalPosition;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum DoorGroundItem {
    Instance([u8; 16]),
    Authored(u64),
}

/// The movement owner has already qualified the vacant protected doorway,
/// its exact bundle and the movable, non-corpse sources on that tile.
#[derive(Clone)]
pub struct DoorGroundRetirement {
    pub door_key: u64,
    pub bundle_digest: [u8; 32],
    pub content_digest: [u8; 32],
    pub reset_epoch: u64,
    pub origin: MovementLocalPosition,
    pub destination: MovementLocalPosition,
    pub origin_revision: u64,
    pub departure_revision: u64,
    pub items: Vec<DoorGroundItem>,
}

impl DurabilityRoot {
    /// Commit the entire cleanup with its receipts before the door pose changes.
    pub async fn retire_door_ground(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        mut request: DoorGroundRetirement,
    ) -> std::result::Result<(), DurabilityError> {
        let rejected = DurabilityError::CommitRejected;
        let door = MapItemPlacement::of_key(request.door_key).ok_or(rejected)?;
        if request.items.len() > 500
            || request.bundle_digest == [0; 32]
            || request.content_digest == [0; 32]
            || request.origin != (MovementLocalPosition {
                x: i32::from(door.x),
                y: i32::from(door.y),
                floor: -i16::from(door.floor),
            })
            || request.origin == request.destination
            || request.origin_revision >= request.departure_revision
            || request.items.iter().any(|item| match item {
                DoorGroundItem::Instance(_) => false,
                DoorGroundItem::Authored(key) => {
                    fence.runtime_scope.channel_id().is_none()
                        || *key == request.door_key
                        || !MapItemPlacement::of_key(*key)
                            .is_some_and(|at| (at.x, at.y, at.floor) == (door.x, door.y, door.floor))
                }
            })
        {
            return Err(DurabilityError::CommitRejected);
        }
        request.items.sort_unstable();
        request.items.dedup();
        let record = authority.record_for(self).map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let mut scope_permit = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut scope_permit, move |holder, deadline, permit| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &record).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                    return Err(DurabilityError::CommitRejected);
                }
                let scope = fence.runtime_scope;
                assert_spell_item_scope_with_recovery(
                    &mut tx, permit, &record, &node, scope, fence.scope_ownership_generation.get(),
                )
                .await
                .map_err(|_| DurabilityError::CommitRejected)?;
                let fact = node.fact();
                let proof = serde_json::json!({
                    "session": hex(fence.game_session_id.as_bytes()),
                    "character": hex(fence.character_id.as_bytes()),
                    "connection": fence.connection_generation.get(),
                    "lease": fence.character_lease_generation,
                    "world": hex(scope.world_id().as_bytes()),
                    "channel": scope.channel_id().map(|id| hex(id.as_bytes())),
                    "instance": scope.instance_id().map(|id| hex(&id)),
                    "ownership": fence.scope_ownership_generation.get(),
                    "node": hex(fact.node_id().as_bytes()),
                    "registration": fact.registration_revision(),
                    "epoch": request.reset_epoch,
                    "bundle": hex(&request.bundle_digest),
                    "content": hex(&request.content_digest),
                    "door": request.door_key,
                    "origin": [request.origin.x, request.origin.y, i32::from(request.origin.floor)],
                    "destination": [request.destination.x, request.destination.y, i32::from(request.destination.floor)],
                    "origin_revision": request.origin_revision,
                    "departure_revision": request.departure_revision,
                });
                for item in &request.items {
                    let (instance, authored) = match item {
                        DoorGroundItem::Instance(item) => (Some(item.as_slice()), None),
                        DoorGroundItem::Authored(key) => (
                            None, Some(i64::try_from(*key).map_err(|_| DurabilityError::CommitRejected)?),
                        ),
                    };
                    let retired: bool = sqlx::query_scalar(
                        "SELECT game_retire_door_ground(encode($1,'hex')::uuid,$2,$3,$4)",
                    )
                    .bind(instance)
                    .bind(authored)
                    .bind(&proof)
                    .bind(node.sql_possession_secret())
                    .fetch_one(&mut *tx).await?;
                    if !retired {
                        return Err(DurabilityError::CommitRejected);
                    }
                }
                commit_semantic_transaction(tx, deadline).await
            })
        }).await
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
