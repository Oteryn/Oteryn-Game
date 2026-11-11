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
