//! The source floor item is a participant of the fenced Quest receipt transaction.
use super::{
    CurrentCharacterGameplayFence, DurabilityError, QuestWorldToolContext, SpellLanePermit,
};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::durability::{
    DurabilityRoot,
    native_map_items_abi::{
        NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
        map_initializer_seal,
    },
    quest_state::{QuestCause, QuestTransitionRequest},
    runtime_scope_assignment::NodeIncarnationProof,
    spell_item_transaction::SpellItemError,
};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FloorReward {
    binding: NativeMapOwnerBinding,
    placement: NativeMapItemPlacement,
}

impl map_initializer_seal::Sealed for FloorReward {}

impl NativeMapInitializationProof for FloorReward {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        Ok(self.binding.clone())
    }

    fn placements(&self) -> &[NativeMapItemPlacement] {
        std::slice::from_ref(&self.placement)
    }
}

impl QuestWorldToolContext {
    /// Retain one exact placement while the caller still holds the live map turn.
    pub fn with_floor_reward(
        mut self,
        current: &impl NativeMapInitializationProof,
    ) -> Option<Self> {
        let [placement] = current.placements() else {
            return None;
        };
        let binding = current.current_binding().ok()?;
        if placement.ground.map_revision != self.map_revision
            || placement.ground.content_revision != self.content_revision
            || placement.ground.native_room_placement_context != self.placement_context
        {
            return None;
        }
        self.floor_reward = Some(FloorReward {
            binding,
            placement: placement.clone(),
        });
        Some(self)
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the existing map owner and exact Quest use fence are independent"
)]
pub(crate) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    root: &DurabilityRoot,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let Some(award) = &callback.effects.floor_reward else {
        return Ok(context.floor_reward.is_none());
    };
    let (Some(proof), Some(source), QuestCause::Use(command)) = (
        &context.floor_reward,
        &callback.selector.native,
        request.cause,
    ) else {
        return Ok(false);
    };
    let position = source.position;
    let placement = &proof.placement;
    if command.game_session_id() != fence.game_session_id
        || fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(
                proof.binding.world,
                proof.binding.channel,
            )
        || fence.scope_ownership_generation.get() != proof.binding.scope_generation
        || placement.definition.production_key != award.item
        || placement.quantity != u32::from(award.count)
        || placement.ground.spatial_position
            != crate::durability::map_item_mint_audit::map_item_spatial_position(
                position.x,
                position.y,
                position.floor,
            )
        || placement.source_attributes["transition"] != callback.key
        || placement.source_attributes["kind"] != "quest_floor_reward"
    {
        return Ok(false);
    }
    match super::super::native_map_items::initialize_with_recovery(
        tx, permit, root, recovery, node, proof,
    )
    .await
    {
        Ok(items) => Ok(items.len() == 1 && !items[0].replayed),
        Err(SpellItemError::Database(error)) => Err(error.into()),
        Err(SpellItemError::Durability(error)) => Err(error),
        Err(_) => Ok(false),
    }
}
