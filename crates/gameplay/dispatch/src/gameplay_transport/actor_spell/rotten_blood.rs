//! Crystal game.cpp7212–7223: Agony bypasses combatBlockHit; creature.cpp842–845
//! excludes mitigation. The existing final HP/mana-shield/lethal owner remains.
use super::*;
impl ChannelSpellStates {
    /// Consumes a closed portal callback plan under the same runtime guard. All
    /// arithmetic and allocations precede reflection and the existing HP owner.
    pub(in crate::gameplay_transport) fn commit_rotten_massive_hit(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        plan: crate::creature_damage_spell::native_event::rotten_massive::RottenMassiveDamagePlan,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<
        (u32, Option<crate::player_lethal::PlayerDamageReceipt>),
        crate::creature_auto_attack::AttackError,
    > {
        use crate::creature_auto_attack::AttackError;
        use crate::creature_damage_spell::native_event::rotten_massive::RottenMassiveKind;
        use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalReceipt};
        if !plan.validate(runtime, self) {
            return Err(AttackError::StaleTarget);
        }
        let source = plan.source();
        let (target, session) = plan.target();
        let requested = plan.requested();
        let kind = match plan.kind() {
            RottenMassiveKind::Energy => crate::player_lethal::PlayerHitKind::Energy,
            RottenMassiveKind::Agony => crate::player_lethal::PlayerHitKind::Mitigated,
        };
        self.prune_source_memos(runtime);
        if let Some(previous) = self.source_damage.iter().find(|memo| {
            memo.actor == target && memo.session == session && memo.occurrence == plan.occurrence()
        }) {
            return if previous.source_primary == Some((source, requested))
                && previous.kind == Some(kind)
            {
                Ok((requested, Some(previous.receipt)))
            } else {
                Err(AttackError::OccurrenceConflict)
            };
        }
        let text =
            Self::source_occurrence_text(plan.occurrence()).ok_or(AttackError::LedgerFull)?;
        let position = runtime
            .read_actor_position(target)
            .map_err(|_| AttackError::StaleTarget)?
            .position();
        let old = self
            .source_damage
            .iter()
            .position(|memo| memo.actor == target && memo.session == session);
        if old.is_none() {
            self.source_damage
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
        }
        let staged = if plan.amount() == 0 {
            None
        } else {
            let state = self
                .get(runtime, target, session)
                .ok_or(AttackError::MissingVitals)?;
            let revision = state.revision();
            let next = match plan.kind() {
                RottenMassiveKind::Energy => {
                    let amount = runtime
                        .tibiadrome_incoming_damage(target, plan.amount(), now.get())
                        .map_err(|_| AttackError::StaleTarget)?;
                    crate::spell::actor_conditions::stage_typed_hit_in_world(
                        runtime,
                        target,
                        state,
                        amount,
                        kind,
                        now.get(),
                    )
                }
                RottenMassiveKind::Agony => {
                    crate::spell::actor_conditions::stage_preprocessed_player_hit(
                        state,
                        plan.amount(),
                        now.get(),
                    )
                }
            }
            .map_err(|_| AttackError::MissingVitals)?;
            if next.1.health_after == 0 {
                self.deaths
                    .try_reserve(1)
                    .map_err(|_| AttackError::LedgerFull)?;
            }
            Some((revision, next))
        };
        // Retain the real source before reflection can lethally damage the boss.
        let (key, display) = plan.source_identity();
        let cause =
            crate::durability::character_death::PublicDeathCause::native_creature(key, display);
        if plan.reflects_equipment()
            && runtime.contains_live_creature(source)
            && !self.reflect_equipment_for_source_occurrence(
                runtime,
                source,
                target,
                session,
                Some("energy"),
                plan.reflection_amount(),
                plan.occurrence().as_bytes(),
                plan.source_sequence(),
                now,
            )
        {
            return Err(AttackError::MissingVitals);
        }
        let Some((revision, next)) = staged else {
            return Ok((requested, None));
        };
        let hit = self
            .apply_player_damage_stage(
                runtime,
                target,
                session,
                kind,
                oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
                Some(cause),
                |current| (current.revision() == revision).then_some(next),
            )
            .ok_or(AttackError::MissingVitals)?;
        let receipt = PlayerDamageReceipt {
            applied: hit.damage.applied,
            health_after: hit.damage.health_after,
            vitals_revision: hit.vitals_revision,
            death: hit.death.map(|bytes| {
                PlayerLethalReceipt::from_native_commit(
                    runtime,
                    target,
                    session,
                    hit.vitals_revision,
                    bytes,
                    position,
                )
            }),
        };
        let memo = NativeSourceMemo {
            actor: target,
            session,
            occurrence: text,
            magnitude: plan.amount(),
            kind: Some(kind),
            source_primary: Some((source, requested)),
            receipt,
        };
        if let Some(index) = old {
            self.source_damage[index] = memo;
        } else {
            self.source_damage.push(memo);
        }
        if plan.critical() {
            runtime
                .record_hazard_critical(target, session, now.get())
                .map_err(|_| AttackError::StaleTarget)?;
        }
        Ok((requested, Some(receipt)))
    }

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
