//! Conditions consume the sole native actor-slot store. No transport condition collection.
//! Runtime lock precedes the existing ChannelSpellStates HP lock at every composition call.
use crate::foundation::{
    ActorConditionPlan, ConditionOwnerError, ConditionSource, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeRuntimeFence,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeConditionError {
    Owner(ConditionOwnerError),
    StaleOwner,
    StaleActor,
    ContentChanged,
    CombatRefused,
}
/// Independently current target/tile Combat facts from the existing map/Combat owner.
/// Missing policy refuses before preparing a native condition; no permissive default.
pub(crate) trait NativeConditionAdmission {
    fn current_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        source: ConditionSource,
    ) -> Option<bool>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pin {
    world: crate::foundation::WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    movement: crate::foundation::MovementPositionContext,
}
impl Pin {
    pub(crate) fn current(runtime: &ChannelRuntimeV1) -> Self {
        let p = runtime.content_pin();
        Self {
            world: p.world_id(),
            activation: p.activation_sequence(),
            server: p.server_artifact_digest(),
            client: p.client_artifact_digest(),
            frame: p.frame_binding_digest(),
            movement: runtime.pinned_movement_context(),
        }
    }
}
pub(crate) fn current_owner(
    runtime: &ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
) -> bool {
    let binding = runtime.binding();
    fence.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) && fence.accepts_stamp(stamp)
}
/// Immutable source-qualified value projection, not actor or content authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeConditionApplication {
    pub(crate) plan: ActorConditionPlan,
    pub(crate) pin: Pin,
    pub(crate) stamp: RuntimeWorkStamp,
    pub(crate) target: ExactActorRef,
    pub(crate) session: Option<GameSessionId>,
    pub(crate) source: ConditionSource,
}
impl NativeConditionApplication {
    pub(crate) fn native_plan(&self) -> &ActorConditionPlan {
        &self.plan
    }
}
pub(crate) fn creature_current(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
    runtime.contains_live_creature(actor)
        && runtime
            .read_actor_position(actor)
            .is_ok_and(|p| p.context() == runtime.pinned_movement_context())
}
