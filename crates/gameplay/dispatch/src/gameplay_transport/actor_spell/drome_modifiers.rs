//! ARENA1 A4 current-HP hazards reuse the existing health/shield commit owner.
use super::*;
#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) enum DromeHealthLoss {
    GoingDown,
    ExplodingCorpse,
    Lava,
}
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn apply_drome_murmillion_paralyze(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        source: ExactActorRef,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<bool> {
        use oteryn_gameplay_domain::domain::condition::{ConditionValues, SpeedRange};
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition,
        };
        let base_speed = self
            .get(runtime, target, session)?
            .owned_base_speed()
            .try_into()
            .ok()?;
        if !runtime.drome_murmillion_target_due(source, target, now.get())? {
            return Some(false);
        }
        let definition = ConditionDefinition::new(
            "oteryn:condition.drome.murmillion.paralyze",
            1,
            ConditionValues::Speed {
                paralysis: true,
                range: SpeedRange {
                    a_min: 0,
                    a_max: 0,
                    b_min: 0,
                    b_max: 0,
                },
                duration_ms: 5000,
            },
        )?;
        let digest = Sha256::digest(format!("drome-murmillion:{source:?}:{}", now.get()));
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let occurrence = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: now.get(),
            base_speed,
            mana_shield_capacity: 0,
            target_reentry_protected: runtime
                .current_player_reentry_protection(target, session, now.get())
                .ok()?,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(id),
        };
        let stamp = runtime.issue_owner_work().ok()?;
        self.native_composite_conditions(
            runtime,
            target,
            session,
            &occurrence,
            source,
            &[definition],
            &facts,
            &[],
            None,
            crate::player_lethal::PlayerHitKind::Mitigated,
            None,
            stamp,
        )?;
        // The held owner and distinct source actor are unchanged by the player condition commit.
        runtime
            .drome_role_paralyze_applied(source, now.get())
            .ok()?;
        Some(true)
    }

    pub(in crate::gameplay_transport) fn apply_drome_health_loss(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        loss: DromeHealthLoss,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<CreatureHit> {
        let percent = match loss {
            DromeHealthLoss::GoingDown => 40,
            DromeHealthLoss::ExplodingCorpse => 25,
            DromeHealthLoss::Lava => 60,
        };
        if !runtime.permits_tibiadrome_health_loss(target, percent) {
            return None;
        }
        let health = self.get(runtime, target, session)?.vitals().health;
        // The accepted current-HP fraction rounds down, so a hazard alone never kills.
        let magnitude = u32::try_from(u64::from(health) * u64::from(percent) / 100).ok()?;
        self.apply_creature_damage_kind_inner(
            runtime,
            target,
            session,
            magnitude,
            crate::player_lethal::PlayerHitKind::Mitigated,
            now,
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
            true,
            // Current-HP arena hazards have no creature caster.
            None,
        )
    }
}
