//! Runtime half of the exact corpse read. The caller retains its runtime owner
//! and SQL transaction through the subsequent quest-tool operation.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};

#[derive(Clone, Copy, Debug)]
pub struct FreshCorpseTarget<'a> {
    pub ground: &'a SpellGroundTarget,
    pub item: [u8; 16],
    pub race: &'a str,
    pub race_revision: &'a str,
    pub maximum_age_ms: NonZeroU32,
}

/// Read a corpse for the current runtime actor. This is not a progress or Item
/// mutation permit; the enclosing Root transaction still owns those decisions.
pub async fn read_for_actor(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    target: FreshCorpseTarget<'_>,
) -> Result<Option<TypedDefinitionRef>> {
    check_transaction(tx, authority).await?;
    if authority.due_read_only
        || authority.runtime_scope != fence.runtime_scope
        || authority.character != *fence.character_id.as_bytes()
        || authority.command.game_session_id() != fence.game_session_id
        || authority.character_lease_generation != fence.character_lease_generation
        || authority.connection_generation != fence.connection_generation
        || authority.ownership_generation != fence.scope_ownership_generation.get()
        || actor.runtime_scope() != fence.runtime_scope
        || runtime.binding().scope_generation().get() != fence.scope_ownership_generation.get()
    {
        return Ok(None);
    }
    let Ok(lease) = runtime.bound_attacker_lease(actor, fence.game_session_id) else {
        return Ok(None);
    };
    if lease.character_id().as_bytes() != fence.character_id.as_bytes()
        || lease.generation() != fence.character_lease_generation
    {
        return Ok(None);
    }
    let Ok(position) = runtime.read_actor_position(actor) else {
        return Ok(None);
    };
    let position = position.position();
    let cell =
        crate::durability::spell_items_abi::decode_ground_cell(&target.ground.spatial_position)
            .map_err(SpellItemError::Rejected)?;
    if i32::from(position.floor) != cell.z
        || position.x.abs_diff(cell.x) > 1
        || position.y.abs_diff(cell.y) > 1
    {
        return Ok(None);
    }
    // The Character issuer already acquired the shared physical owner lock,
    // after locking its Character. Preserve that order and physical XID.
    let scope = SpellItemScopeAuthority {
        physical_transaction: authority.physical_transaction.clone(),
        runtime_scope: authority.runtime_scope,
        world: authority.world,
        channel: authority.channel,
        ownership_generation: authority.ownership_generation,
    };
    read_current_fresh(
        tx,
        &scope,
        target.ground,
        target.item,
        target.race,
        target.race_revision,
        target.maximum_age_ms,
    )
    .await
}
