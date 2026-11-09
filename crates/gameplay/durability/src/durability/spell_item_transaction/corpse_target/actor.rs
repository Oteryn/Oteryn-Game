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

/// Read a corpse for the current runtime actor. The enclosing Root transaction
/// still owns progress and Item mutation decisions.
pub async fn read_for_actor(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    target: FreshCorpseTarget<'_>,
) -> Result<Option<TypedDefinitionRef>> {
    check_transaction(tx, authority).await?;
    let Some(snapshot) = CorpseActorSnapshot::read_current(runtime, actor, fence) else {
        return Ok(None);
    };
    snapshot.read_in_transaction(tx, authority, target).await
}

#[derive(Debug)]
pub(crate) struct CorpseActorSnapshot<'runtime> {
    fence: CurrentCharacterGameplayFence,
    position: crate::foundation::MovementLocalPosition,
    _runtime: std::marker::PhantomData<&'runtime ()>,
}
impl<'runtime> CorpseActorSnapshot<'runtime> {
    pub(crate) fn read_current(
        runtime: &'runtime ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
    ) -> Option<Self> {
        if actor.runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation().get() != fence.scope_ownership_generation.get()
        {
            return None;
        }
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return None;
        }
        Some(Self {
            fence,
            position: runtime.read_actor_position(actor).ok()?.position(),
            _runtime: std::marker::PhantomData,
        })
    }
    pub(crate) async fn read_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        authority: &SpellItemAuthority,
        target: FreshCorpseTarget<'_>,
    ) -> Result<Option<TypedDefinitionRef>> {
        let fence = self.fence;
        check_transaction(tx, authority).await?;
        if authority.due_read_only
            || authority.runtime_scope != fence.runtime_scope
            || authority.character != *fence.character_id.as_bytes()
            || authority.command.game_session_id() != fence.game_session_id
            || authority.character_lease_generation != fence.character_lease_generation
            || authority.connection_generation != fence.connection_generation
            || authority.ownership_generation != fence.scope_ownership_generation.get()
        {
            return Ok(None);
        }
        let cell =
            crate::durability::spell_items_abi::decode_ground_cell(&target.ground.spatial_position)
                .map_err(SpellItemError::Rejected)?;
        // Native corpse custody stores -z; runtime actor positions use z.
        if i32::from(self.position.floor) != -cell.z
            || self.position.x.abs_diff(cell.x) > 1
            || self.position.y.abs_diff(cell.y) > 1
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
}
