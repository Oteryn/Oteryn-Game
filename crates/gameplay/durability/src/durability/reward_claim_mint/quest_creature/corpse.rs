//! Fresh corpse admission participates in the same quest transaction as the
//! selected tool debit, reward MINT and progress. No detached permission exists.
use super::*;
use crate::durability::spell_item_transaction::{
    SpellItemError, assert_spell_item_authority_with_recovery,
    corpse_target::{CorpseActorSnapshot, FreshCorpseTarget},
};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, DurabilityRoot};
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, decode_use_intent_target};

/// UUID and cell resolved by the runtime's existing current map-handle owner.
#[derive(Clone, Debug)]
pub struct QuestCorpseOperand {
    pub item: [u8; 16],
    pub ground: SpellGroundTarget,
}
pub(super) struct CorpseUse<'runtime> {
    actor: CorpseActorSnapshot<'runtime>,
    operand: QuestCorpseOperand,
    permit: &'runtime SpellLanePermit,
}
impl std::fmt::Debug for CorpseUse<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CorpseUse")
            .field("operand", &self.operand)
            .finish_non_exhaustive()
    }
}
impl<'runtime> QuestCreatureUseProof<'runtime> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the runtime owner binds the exact command, source policy and resolved Ground operand"
    )]
    pub fn read_corpse(
        runtime: &'runtime ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &RewardCallback,
        intent: &UseIntent,
        operand: QuestCorpseOperand,
        permit: &'runtime SpellLanePermit,
    ) -> Option<Self> {
        if !callback.fresh_corpse
            || callback.carried_item.is_none()
            || command.game_session_id() != fence.game_session_id
            || operand.item == [0; 16]
            || permit.check_scope(fence.runtime_scope).is_err()
            || intent.use_with.is_some()
            || intent.use_with_position.is_some()
            || !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
        {
            return None;
        }
        Some(Self {
            fence,
            command,
            callback: callback.key.clone(),
            key: callback.creature_target.clone()?,
            intent: intent.clone(),
            corpse: Some(CorpseUse {
                actor: CorpseActorSnapshot::read_current(runtime, actor, fence)?,
                operand,
                permit,
            }),
            _runtime: std::marker::PhantomData,
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the existing Root supplies the complete fenced transaction and immutable source callback"
    )]
    pub(crate) async fn permits_corpse(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: (
            &DurabilityRoot,
            &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        ),
        node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        callback: &RewardCallback,
        carried: &super::super::QuestCarriedUse,
        native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    ) -> Result<bool, DurabilityError> {
        let Some(corpse) = &self.corpse else {
            return Ok(false);
        };
        if !callback.fresh_corpse
            || self.fence.item_fence() != fence.item_fence()
            || request.cause != QuestCause::Command(self.command)
            || request.transition_key != self.callback
            || callback.creature_target.as_deref() != Some(self.key.as_str())
            || decode_use_intent_target(&[4, 11, 15], &carried.intent)
                .ok()
                .as_ref()
                != Some(&self.intent)
        {
            return Ok(false);
        }
        let mut profiles = native
            .creature_profiles()
            .records
            .iter()
            .filter(|r| r.profile.target.key == self.key);
        let Some(profile) = profiles.next() else {
            return Ok(false);
        };
        if profiles.next().is_some() {
            return Ok(false);
        }
        let authority = match assert_spell_item_authority_with_recovery(
            tx,
            corpse.permit,
            owner.0,
            owner.1,
            node,
            &fence.item_fence(),
            self.command,
            native.source_digest(),
        )
        .await
        {
            Ok(authority) => authority,
            Err(SpellItemError::Database(error)) => return Err(error.into()),
            Err(SpellItemError::Durability(error)) => return Err(error),
            Err(_) => return Ok(false),
        };
        match corpse
            .actor
            .read_in_transaction(
                tx,
                &authority,
                FreshCorpseTarget {
                    ground: &corpse.operand.ground,
                    item: corpse.operand.item,
                    race: &self.key,
                    race_revision: &profile.profile.target.revision,
                    // First-form/native expiry remains authoritative; no guessed quest age.
                    maximum_age_ms: std::num::NonZeroU32::MAX,
                },
            )
            .await
        {
            Ok(value) => Ok(value.is_some()),
            Err(SpellItemError::Database(error)) => Err(error.into()),
            Err(SpellItemError::Durability(error)) => Err(error),
            Err(_) => Ok(false),
        }
    }
}
