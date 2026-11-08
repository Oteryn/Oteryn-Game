//! Sealed exact live-creature admission for a carried quest tool.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::UseIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

/// A live-target proof cannot escape the runtime owner turn that observed it.
///
/// ```compile_fail
/// use oteryn_gameplay_durability::durability::{
///     reward_claim_mint::quest_creature::QuestCreatureUseProof,
///     character_progression::CurrentCharacterGameplayFence,
/// };
/// use oteryn_gameplay_entities::{ChannelRuntimeV1, ExactActorRef, CommandRef};
/// use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
/// use oteryn_protocol_oteryn::world_object::UseIntent;
/// fn escape(runtime: ChannelRuntimeV1, actor: ExactActorRef,
///     fence: CurrentCharacterGameplayFence, command: CommandRef,
///     callback: &RewardCallback, intent: &UseIntent) -> impl std::fmt::Debug + 'static {
///     QuestCreatureUseProof::read_current(&runtime, actor, fence, command, callback, intent)
///         .expect("current target")
/// }
/// ```
#[derive(Debug)]
pub struct QuestCreatureUseProof<'runtime> {
    _runtime: std::marker::PhantomData<&'runtime ChannelRuntimeV1>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: String,
    key: String,
    target: EntityRef,
    target_actor: ExactActorRef,
}
impl<'runtime> QuestCreatureUseProof<'runtime> {
    pub fn read_current(
        runtime: &'runtime ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        intent: &UseIntent,
    ) -> Option<Self> {
        let key = callback.creature_target.as_ref()?;
        let target = intent.use_with?;
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if actor.runtime_scope() != fence.runtime_scope
            || lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || command.game_session_id() != fence.game_session_id
        {
            return None;
        }
        let creature = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                c.actor.placement_identity() == target.identity && c.generation == target.generation
            })?
            .actor;
        runtime.check_mount_taming_subject(creature, key).ok()?;
        let own = runtime.read_actor_position(actor).ok()?.position();
        let at = runtime.read_actor_position(creature).ok()?.position();
        if at.floor != own.floor || at.x.abs_diff(own.x) > 1 || at.y.abs_diff(own.y) > 1 {
            return None;
        }
        Some(Self {
            _runtime: std::marker::PhantomData,
            fence,
            command,
            callback: callback.key.clone(),
            key: key.clone(),
            target,
            target_actor: creature,
        })
    }
    /// Exact handle observed in the retained runtime turn; used after a committed capture.
    pub fn target_actor(&self) -> ExactActorRef {
        self.target_actor
    }

    pub(super) async fn record_capture(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        carried: &super::QuestCarriedUse,
    ) -> Result<bool, crate::durability::DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = self.fence.runtime_scope
        else {
            return Ok(false);
        };
        Ok(sqlx::query("INSERT INTO game_quest_creature_captures(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,scope_generation,target_identity,target_generation,creature_key,source_item,ingress) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8,$9::text::numeric,$10,encode($11,'hex')::uuid,$12) ON CONFLICT DO NOTHING")
            .bind(self.fence.character_id.as_bytes().as_slice())
            .bind(self.command.game_session_id().as_bytes().as_slice())
            .bind(self.command.command_id().get().to_string()).bind(&self.callback)
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
            .bind(self.fence.scope_ownership_generation.get().to_string())
            .bind(self.target.identity.as_slice()).bind(self.target.generation.to_string())
            .bind(&self.key).bind(carried.source.as_slice()).bind(&carried.intent)
            .execute(&mut **tx).await?.rows_affected() == 1)
    }

    pub(super) fn permits(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        key: &str,
        carried: &super::QuestCarriedUse,
    ) -> bool {
        self.fence.item_fence() == fence.item_fence()
            && request.cause == QuestCause::Command(self.command)
            && request.transition_key == self.callback
            && self.key == key
            && oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 15],
                &carried.intent,
            )
            .is_ok_and(|intent| {
                intent.use_with == Some(self.target)
                    && intent.use_with_map_item.is_none()
                    && intent.use_with_position.is_none()
            })
    }
}
