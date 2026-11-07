//! W05d source-only damage owner; existing callers retain their original shield policy.
use super::*;
impl ChannelSpellStates {
    #[allow(
        clippy::too_many_arguments,
        reason = "existing final damage owner plus explicit closed source shield policy"
    )]
    pub(super) fn apply_creature_damage_kind_policy(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
        analyser_source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource,
        skip_mana_shield: bool,
    ) -> Option<CreatureHit> {
        // Durable unknown outcomes retain their exact player before-state. A bite
        // must not invalidate that state, including another caster's reserved target.
        if self.is_dead(target)
            || self.has_pending_spell_commit(target, target_session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        let state = self.get(runtime, target, target_session)?;
        let health_before = state.vitals().health;
        let staged = if skip_mana_shield {
            crate::spell::actor_conditions::stage_cake_undefined_hit_in_world(
                runtime,
                target,
                state,
                magnitude,
                now.get(),
            )
        } else {
            crate::spell::actor_conditions::stage_typed_hit_in_world(
                runtime,
                target,
                state,
                magnitude,
                kind,
                now.get(),
            )
        };
        let (next, damage) = staged.ok()?;
        if next.revision() == state.revision() {
            return Some(CreatureHit {
                damage,
                vitals_revision: state.revision(),
                death: None,
            });
        }
        let death = if damage.health_after == 0 {
            let cell = runtime.read_actor_position(target).ok()?.position();
            self.deaths.try_reserve(1).ok()?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)()?,
                cell,
            })
        } else {
            None
        };
        let vitals_revision = next.revision();
        if !self.commit(runtime, target, target_session, next) {
            return None;
        }
        if let Some(value) =
            std::num::NonZeroU32::new(health_before.saturating_sub(damage.health_after))
        {
            self.analyser.push(
                target,
                target_session,
                oteryn_protocol_oteryn::analyser::AnalyserFact::DamageInput {
                    value,
                    element: crate::gameplay_transport::analyser::element(kind),
                    source: analyser_source,
                },
            );
        }
        if let Some(death) = death {
            self.deaths.push((target, target_session, death));
        }
        Some(CreatureHit {
            damage,
            vitals_revision,
            death: death.map(|death| *death.occurrence.as_bytes()),
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "existing final damage owner plus explicit closed source shield policy"
    )]
    pub(super) fn apply_typed_attack_damage_from_policy(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
        analyser_source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource,
        skip_mana_shield: bool,
    ) -> Option<crate::player_lethal::PlayerDamageReceipt> {
        use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalReceipt};
        if magnitude == 0
            || runtime
                .player_control_facts(target, session)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        if self
            .source_secondary_memos
            .iter()
            .any(|m| m.actor == target && m.session == session && m.occurrence == occurrence)
            && !self
                .source_damage
                .iter()
                .any(|m| m.actor == target && m.session == session && m.occurrence == occurrence)
        {
            return None;
        }
        let text = Self::source_occurrence_text(occurrence)?;
        self.prune_source_memos(runtime);
        let old = self
            .source_damage
            .iter()
            .position(|m| m.actor == target && m.session == session);
        if let Some(i) = old {
            let m = &self.source_damage[i];
            if m.occurrence == occurrence {
                return (m.magnitude == magnitude && m.kind == Some(kind)).then_some(m.receipt);
            }
        }
        if self.is_dead(target) {
            return None;
        }
        // Position/allocation failures precede native HP. Existing native owner rechecks position,
        // reserves and mints its one PlayerDeathOccurrence before its own compare-commit.
        let position = runtime.read_actor_position(target).ok()?.position();
        if old.is_none() {
            self.source_damage.try_reserve(1).ok()?;
        }
        let hit = self.apply_creature_damage_kind_policy(
            runtime,
            target,
            session,
            magnitude,
            kind,
            now,
            analyser_source,
            skip_mana_shield,
        )?;
        // PREY-DIFF: the composite source boundary records the race-qualified activity;
        // this lower-level method also serves source-less condition damage.
        let death = hit.death.map(|bytes| {
            PlayerLethalReceipt::from_native_commit(
                runtime,
                target,
                session,
                hit.vitals_revision,
                bytes,
                position,
            )
        });
        let receipt = PlayerDamageReceipt {
            applied: hit.damage.applied,
            health_after: hit.damage.health_after,
            vitals_revision: hit.vitals_revision,
            death,
        };
        let memo = NativeSourceMemo {
            actor: target,
            session,
            occurrence: text,
            magnitude,
            kind: Some(kind),
            receipt,
        };
        if let Some(i) = old {
            self.source_damage[i] = memo
        } else {
            self.source_damage.push(memo)
        }
        Some(receipt)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "closed source damage retains exact actor, session and occurrence fences"
    )]
    pub(super) fn apply_cake_undefined_source(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        source: ExactActorRef,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<crate::player_lethal::PlayerDamageReceipt> {
        if magnitude > 40
            || !runtime.matches_live_creature_identity(source, b"oteryn:creature.cake_golem")
        {
            return None;
        }
        self.apply_typed_attack_damage_from_policy(
            runtime,
            target,
            session,
            magnitude,
            crate::player_lethal::PlayerHitKind::Mitigated,
            occurrence,
            now,
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::None,
            true,
        )
    }
}
