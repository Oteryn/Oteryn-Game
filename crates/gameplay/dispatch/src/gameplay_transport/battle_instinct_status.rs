//! C11 own-player read from the same physical census used by combat.
use super::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, capabilities::OfferedCapability,
    connection::AdmittedSession,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::battle_instinct_status::{
    BattleInstinctStatus, CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_BATTLE_INSTINCT_STATUS_V1)
        .collect()
}

macro_rules! authority_methods {
    () => {
        fn battle_instinct_status_available(&self) -> bool {
            false
        }
        fn battle_instinct_status_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

pub(in crate::gameplay_transport) fn project(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    now_us: u64,
) -> Option<BattleInstinctStatus> {
    if runtime
        .player_control_facts(actor, session)
        .ok()?
        .control_loss
        .is_some()
    {
        return None;
    }
    let state = states.get(runtime, actor, session)?;
    let active = state
        .wheel_effects_at(now_us)
        .is_some_and(|effects| effects.active_unique_perks.contains("battle_instinct"));
    let bonuses = state.wheel_battle_instinct(runtime, actor, now_us)?;
    Some(BattleInstinctStatus {
        active,
        melee: bonuses.melee,
        shielding: bonuses.shielding,
    })
}

pub(in crate::gameplay_transport) fn actor_is_current(
    runtime: &ChannelRuntimeV1,
    admitted: AdmittedSession,
) -> bool {
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    let binding = runtime.binding();
    fence.game_session_id == admitted.game_session_id
        && fence.connection_generation.get() == admitted.continuity.connection_generation
        && binding.world_id() == admitted.world_id
        && binding.channel_id() == Some(admitted.channel_id)
        && binding.runtime_scope() == fence.runtime_scope
        && binding.scope_generation() == fence.scope_ownership_generation
        && runtime.owner_fence().is_ok()
        && runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        && runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn battle_instinct_status_page(
        &self,
        admitted: AdmittedSession,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_BATTLE_INSTINCT_STATUS_V1)
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        {
            let runtime = self.runtime.lock().await;
            if !actor_is_current(&runtime, admitted) {
                return None;
            }
        }
        if !self
            .refresh_wheel_vitals(actor, admitted.game_session_id)
            .await
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        if !actor_is_current(&runtime, admitted) {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        let now = self.owner_now().get();
        project(&runtime, &states, actor, admitted.game_session_id, now)?.encode()
    }
}

macro_rules! composed_methods {
    () => {
        fn battle_instinct_status_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn battle_instinct_status_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.battle_instinct_status_page(admitted).await
        }
    };
}
pub(super) use composed_methods;
