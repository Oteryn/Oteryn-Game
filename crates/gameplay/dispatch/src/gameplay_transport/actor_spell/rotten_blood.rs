//! Crystal game.cpp7212–7223: Agony bypasses combatBlockHit; creature.cpp842–845
//! excludes mitigation. The existing final HP/mana-shield/lethal owner remains.
use super::*;
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn apply_rotten_blood_agony(
        &mut self,
        runtime: &ChannelRuntimeV1,
        source: &oteryn_gameplay_entities::carrier::rotten_blood::RottenBloodIconSource,
        actor: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        if !runtime.rotten_blood_agony_source_current(source) {
            return None;
        }
        self.apply_player_damage_stage(
            runtime,
            actor,
            session,
            crate::player_lethal::PlayerHitKind::Mitigated,
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
            // Crystal rotten_blood_boss_mechanics.lua248 uses player:addHealth: the Lua caster is nil.
            None,
            |state| {
                crate::spell::actor_conditions::stage_preprocessed_player_hit(
                    state,
                    magnitude,
                    now.get(),
                )
                .ok()
            },
        )
    }
}
