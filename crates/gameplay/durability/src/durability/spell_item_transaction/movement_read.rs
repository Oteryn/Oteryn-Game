//! Physical movement may inspect a grace player's destination without restoring input.
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};

fn forced_fence_current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: &CurrentCharacterItemFence,
    content: [u8; 32],
) -> bool {
    runtime.forced_player_step_current(actor, fence.game_session_id)
        && runtime.binding().runtime_scope() == fence.runtime_scope
        && runtime.binding().scope_generation() == fence.scope_ownership_generation
        && runtime.content_pin().server_artifact_digest() == content
        && runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}

/// The usual active-session tile read, with a bounded physical-push exception.
/// The runtime owner stays borrowed across SQL and the push proof is rechecked
/// before returning detached evidence. Existing command readers remain active-only.
#[allow(clippy::too_many_arguments)]
pub async fn read_player_step_tile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
    target: &SpellGroundTarget,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<StandingTileRead> {
    let forced = runtime.forced_player_step_current(actor, fence.game_session_id);
    if forced && !forced_fence_current(runtime, actor, fence, content_digest) {
        return Err(SpellItemError::Rejected("physical step fence mismatch"));
    }
    let record = recovery
        .record_for(root)
        .map_err(|_| SpellItemError::Rejected("standing read recovery authority"))?;
    let read = read_standing_player_tile_with_recovery(
        tx,
        permit,
        root,
        &record,
        node,
        fence,
        content_digest,
        target,
        forced,
    )
    .await?;
    if forced && !forced_fence_current(runtime, actor, fence, content_digest) {
        return Err(SpellItemError::Rejected(
            "physical step proof expired during read",
        ));
    }
    Ok(read)
}

impl DurabilityRoot {
    /// Complete, bounded occupancy of the current Ground namespace. Include all live
    /// rows, even ones the visual projection cannot decode or the field owner would
    /// reject. Malformed coordinates make absence unprovable anywhere in the scope.
    pub async fn read_movement_ground_cells(
        &self,
        scope: crate::foundation::RuntimeScopeRefV1,
    ) -> std::result::Result<std::collections::HashSet<[u8; 10]>, DurabilityError> {
        let world = scope.world_id();
        let (kind, channel, instance) = super::super::item_mint::ground_scope_columns(scope);
        let max = super::super::item_loop_move::ITEM_LOOP_CHANNEL_DROPS_MAX
            + super::super::field_removal::CHANNEL_FIELDS_MAX
            + super::super::item_mint::COMBAT01_CORPSES_PER_SCOPE_MAX;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                let rows: Vec<Vec<u8>> = sqlx::query_scalar(
                    "SELECT DISTINCT g.spatial_position
                       FROM game_item_ground_locations g
                       JOIN game_item_instances i USING (item_instance_id, world_id)
                      WHERE g.world_id=encode($1,'hex')::uuid AND g.scope_kind=$2
                        AND g.channel_id IS NOT DISTINCT FROM encode($3,'hex')::uuid
                        AND g.instance_id IS NOT DISTINCT FROM encode($4,'hex')::uuid
                        AND i.lifecycle=1
                      LIMIT $5",
                )
                .bind(world.as_bytes().as_slice())
                .bind(kind)
                .bind(channel.map(|id| id.to_vec()))
                .bind(instance.map(|id| id.to_vec()))
                .bind(max + 1)
                .fetch_all(&mut *tx)
                .await?;
                if rows.len() > max as usize {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let cells = rows.into_iter()
                    .map(|cell| cell.try_into().map_err(|_| DurabilityError::InvalidStoredState))
                    .collect::<std::result::Result<_, _>>()?;
                super::super::db::commit_semantic_read(tx, deadline).await?;
                Ok(cells)
            })
        }).await
    }
}
