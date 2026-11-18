//! Crystal 201189f8 data-global/scripts/quests/the_pits_of_inferno_quest/movements_fire_tile.lua:1-31,47-54.
use super::*;
use crate::{
    content::{ReferenceResistanceKind, spell_data::Vocation},
    foundation::owner_timer::SemanticTimeMicros,
    player_lethal::PlayerHitKind,
};
use oteryn_gameplay_quests::quest::world_callbacks::{
    StepDamage, StepDamageKind, StepDamageVocation, VocationStepDamage,
};
use oteryn_protocol_oteryn::analyser::AnalyserDamageSource;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "rechecks the exact movement and map source under the Channel turn"
    )]
    pub(super) async fn apply_vocation_step_damage(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        damage: VocationStepDamage,
    ) {
        let mut states;
        let runtime = loop {
            let runtime = self.runtime.lock().await;
            if let Ok(guard) = self.states.spell_states.try_lock() {
                states = guard;
                break runtime;
            }
            drop(runtime);
            drop(self.states.spell_states.lock().await);
        };
        if runtime.read_actor_position(actor) != Ok(entered)
            || map.facts.base_hidden(key)
            || !matches(map, callback, key)
            || states.is_dead(actor)
        {
            return;
        }
        let Some(state) = states.get(&runtime, actor, session) else {
            return;
        };
        let base = match state.source_party_vocation() {
            Vocation::Sorcerer | Vocation::MasterSorcerer => Some(StepDamageVocation::Sorcerer),
            Vocation::Druid | Vocation::ElderDruid => Some(StepDamageVocation::Druid),
            Vocation::Paladin | Vocation::RoyalPaladin => Some(StepDamageVocation::Paladin),
            Vocation::Knight | Vocation::EliteKnight => Some(StepDamageVocation::Knight),
            Vocation::Monk | Vocation::ExaltedMonk => Some(StepDamageVocation::Monk),
            Vocation::None => None,
        };
        let hit = if base == Some(damage.vocation) {
            damage.matching
        } else {
            damage.other
        };
        self.apply_quest_hit(&runtime, &mut states, actor, session, entered, map, hit);
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "damage and presentation use the same current actor snapshot"
    )]
    pub(super) fn apply_quest_hit(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        map: &BundleMap,
        hit: StepDamage,
    ) {
        let Some(state) = states.get(runtime, actor, session) else {
            return;
        };
        let (kind, resistance) = match hit.kind {
            StepDamageKind::Fire => (PlayerHitKind::Fire, ReferenceResistanceKind::Fire),
            StepDamageKind::Physical => {
                (PlayerHitKind::Physical, ReferenceResistanceKind::Physical)
            }
        };
        let raw = i64::from(hit.amount);
        let amount = state
            .equipped_element_protection(resistance)
            .and_then(|protection| {
                oteryn_gameplay_items::equipment_effects::apply_protection(raw, protection)
            })
            .unwrap_or(raw);
        let Ok(amount) = u32::try_from(amount) else {
            return;
        };
        if states
            .apply_creature_damage_kind(
                runtime,
                actor,
                session,
                amount,
                kind,
                SemanticTimeMicros::from_micros(self.owner_now().get()),
                AnalyserDamageSource::None,
            )
            .is_some()
            && let Some(position) = native(entered.position())
        {
            map.facts
                .emit_movement_effect(position, u16::from(hit.effect));
        }
    }
}
