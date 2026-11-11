//! The accepted Blue area keeps its original native source proof across a
//! setType/removal; each captured Player still uses the canonical HP owner.
use super::*;

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn apply_retained_soul_war_blue_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        area: &super::super::soul_war_callbacks::PreparedSoulWarArea,
        target: ExactActorRef,
        session: GameSessionId,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        if !area.permits_blue_target(runtime, native, target, session) {
            return None;
        }
        self.apply_creature_damage_kind_inner(
            runtime,
            target,
            session,
            6000,
            crate::player_lethal::PlayerHitKind::Death,
            now,
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
            true,
            None,
        )
    }
}

#[cfg(test)]
impl ChannelSpellStates {
    /// Canonical committed HP memo, without a second test damage/heal owner.
    pub(in crate::gameplay_transport) fn source_damage_receipt_for_test(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(String, u32, crate::player_lethal::PlayerDamageReceipt)> {
        self.source_damage
            .iter()
            .find(|memo| memo.actor == actor && memo.session == session)
            .map(|memo| (memo.occurrence.clone(), memo.magnitude, memo.receipt))
    }

    pub(in crate::gameplay_transport) fn source_heal_receipt_for_test(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(String, u32, crate::player_lethal::PlayerHealReceipt)> {
        self.source_heal
            .iter()
            .find(|memo| memo.actor == actor && memo.session == session)
            .map(|memo| (memo.occurrence.clone(), memo.magnitude, memo.receipt))
    }
}
