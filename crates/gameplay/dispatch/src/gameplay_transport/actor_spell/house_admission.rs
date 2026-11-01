//! A recovered House resident keeps the same native PlayerSpellState owner.
//! Generic Channel actor access remains closed; publication proves named custody.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;

fn matches_house(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
) -> Option<()> {
    let house = runtime.house_actor_state(actor, session).ok()?;
    if fence.game_session_id != session
        || house.scope != fence.runtime_scope
        || house.generation != fence.scope_ownership_generation
        || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
        || house.lease.generation() != fence.character_lease_generation
        || runtime
            .house_player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
    {
        return None;
    }
    Some(())
}

impl ChannelSpellStates {
    /// Admission snapshot only. Generic combat Charm reads still require their
    /// original Channel owner, while House custody retains this Character data.
    pub(in crate::gameplay_transport) fn install_house_charm_state(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
        state: Option<crate::durability::charm_state::CombatCharmState>,
    ) -> Option<()> {
        matches_house(runtime, actor, session, fence)?;
        self.charm_states.retain(|(present, held, _)| {
            (runtime.player_control_facts(*present, *held).is_ok()
                || runtime.house_player_control_facts(*present, *held).is_ok())
                && !(*present == actor && *held == session)
        });
        if let Some(state) = state {
            self.charm_states.push((actor, session, state));
        }
        Some(())
    }

    /// Installs a fully prepared native state only when this exact House actor has
    /// none. A retry preserves current health, cooldowns and remaining buff ticks.
    pub(in crate::gameplay_transport) fn install_house_admitted(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
        prepared: PlayerSpellState,
    ) -> Option<bool> {
        matches_house(runtime, actor, session, fence)?;
        self.initialize_owner_bookkeeping(runtime)?;
        if self.index(actor, session).is_some() {
            return Some(false);
        }
        self.actors.try_reserve(1).ok()?;
        self.actors.push((actor, session, prepared));
        Some(true)
    }

    /// Explicit scope-qualified access for House admission effects and vitals.
    pub(in crate::gameplay_transport) fn house_state_mut(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
    ) -> Option<&mut PlayerSpellState> {
        matches_house(runtime, actor, session, fence)?;
        let index = self.index(actor, session)?;
        self.actors.get_mut(index).map(|(_, _, state)| state)
    }

    pub(in crate::gameplay_transport) fn house_vitals(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
    ) -> Option<(u64, ActorVitals)> {
        matches_house(runtime, actor, session, fence)?;
        let (_, _, state) = self.actors.get(self.index(actor, session)?)?;
        Some((state.revision(), state.vitals()))
    }
}
