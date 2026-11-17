//! The source returns first, then applies fire through the existing typed HP owner.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn punish_carrot_refusal(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        returned: MovementPositionSnapshot,
    ) {
        let known = self
            .states
            .quest_callbacks
            .carrot_refusal
            .lock()
            .is_ok_and(|mut refusal| refusal.take() == Some((actor, session, entered)));
        if !known || runtime.read_actor_position(actor) != Ok(returned) {
            return;
        }
        use oteryn_simulation_determinism::{
            DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
        };
        use sha2::{Digest, Sha256};
        let digest = Sha256::new()
            .chain_update(b"oteryn:dreamer-carrot-refusal:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(entered.revision().to_be_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let Ok(draw) = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "movement.trap.damage",
            0,
        ) else {
            return;
        };
        let raw = crate::spell::uniform_draw(draw, 10, 20);
        let damage = states
            .get(runtime, actor, session)
            .and_then(|state| {
                state.equipped_element_protection(crate::content::ReferenceResistanceKind::Fire)
            })
            .and_then(|protection| {
                oteryn_gameplay_items::equipment_effects::apply_protection(raw, protection)
            })
            .unwrap_or(raw);
        let Ok(damage) = u32::try_from(damage) else {
            return;
        };
        if states
            .apply_creature_damage_kind(
                runtime,
                actor,
                session,
                damage,
                crate::player_lethal::PlayerHitKind::Fire,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    self.owner_now().get(),
                ),
                oteryn_protocol_oteryn::analyser::AnalyserDamageSource::None,
            )
            .is_some()
            && let Some(position) = native(returned.position())
            && let Some(map) = self.bundle_map()
        {
            map.facts.emit_movement_effect(position, 16);
        }
    }
}
