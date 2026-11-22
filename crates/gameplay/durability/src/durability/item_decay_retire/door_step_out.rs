//! Terminal Ground custody and immutable bundle tombstones for a blocked door step-out.
use super::*;
use crate::durability::{
    character_progression::assert_item_only_fence,
    item_transfer::CurrentCharacterItemFence,
    map_item_mint::{MapItemPlacement, lock_placement},
    map_item_mint_audit::map_revision_of,
    spell_item_transaction::assert_spell_item_scope_with_recovery,
};

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
                let world = *scope.world_id().as_bytes();
                let current_epoch: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                    .bind(world.as_slice()).bind(request.reset_epoch.to_string())
                    .fetch_one(&mut *tx).await?;
                if !current_epoch {
                    return Err(DurabilityError::CommitRejected);
                }
                let mut position = Vec::with_capacity(10);
                position.extend_from_slice(&i32::from(door.x).to_be_bytes());
                position.extend_from_slice(&i32::from(door.y).to_be_bytes());
                let mut native_position = position.clone();
                position.extend_from_slice(&i16::from(door.floor).to_be_bytes());
                native_position.extend_from_slice(&(-i16::from(door.floor)).to_be_bytes());
                let map_revision = map_revision_of(&request.bundle_digest);
                let native_map_revision = map_revision_of(&Sha256::digest(map_revision.as_bytes()));
                let content_revision = map_revision_of(&request.content_digest);
                let channel = scope.channel_id().map(|id| *id.as_bytes());
                let instance = scope.instance_id();
                for item in &request.items {
                    match item {
                        DoorGroundItem::Instance(item) => {
                            let retired: bool = sqlx::query_scalar(
                                "SELECT game_retire_door_ground(encode($1,'hex')::uuid,encode($2,'hex')::uuid,
                                 encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,
                                 $6::text::numeric,$7,$8,$9,$10,$11)",
                            )
                            .bind(item.as_slice())
                            .bind(fence.game_session_id.as_bytes().as_slice())
                            .bind(world.as_slice())
                            .bind(channel.as_ref().map(|id| id.as_slice()))
                            .bind(instance.as_ref().map(|id| id.as_slice()))
                            .bind(fence.scope_ownership_generation.get().to_string())
                            .bind(&position)
                            .bind(&map_revision)
                            .bind(&content_revision)
                            .bind(&native_position)
                            .bind(&native_map_revision)
                            .fetch_one(&mut *tx).await?;
                            if !retired {
                                return Err(DurabilityError::CommitRejected);
                            }
                        }
                        DoorGroundItem::Authored(key) => {
                            let channel = channel.ok_or(DurabilityError::CommitRejected)?;
                            let mut placement = request.bundle_digest.to_vec();
                            placement.extend_from_slice(&key.to_be_bytes());
                            lock_placement(&mut tx, &world, &channel, &placement, request.reset_epoch).await?;
                            sqlx::query(
                                "INSERT INTO game_door_authored_retirements
                                    (world_id,channel_id,base_bundle_digest,placement_key,reset_epoch,door_key,game_session_id)
                                 SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,
                                        $5::text::numeric,$6::text::numeric,encode($7,'hex')::uuid
                                 WHERE NOT EXISTS (SELECT 1 FROM game_map_item_mint_receipts
                                     WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                                       AND base_bundle_digest=$3 AND placement_key=$4::text::numeric
                                       AND reset_epoch=$5::text::numeric)
                                 ON CONFLICT DO NOTHING",
                            )
                            .bind(world.as_slice())
                            .bind(channel.as_slice())
                            .bind(request.bundle_digest.as_slice())
                            .bind(key.to_string())
                            .bind(request.reset_epoch.to_string())
                            .bind(request.door_key.to_string())
                            .bind(fence.game_session_id.as_bytes().as_slice())
                            .execute(&mut *tx).await?;
                        }
                    }
                }
                commit_semantic_transaction(tx, deadline).await
            })
        }).await
    }
}
