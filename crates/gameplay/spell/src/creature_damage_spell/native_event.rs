//! Sealed native Think events use the normal Creature damage preparation/HP owner.
//! A Think event has no Ability identity and never enters the spell scheduler.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::ThinkOccurrence;

pub mod rotten_massive;

const MUSHROOM: &str = "oteryn:creature.mushroom";

#[derive(Debug, Clone)]
pub struct MushroomThinkSource {
    actor: ExactActorRef,
    sequence: u64,
    stamp: RuntimeWorkStamp,
    pins: ([u8; 32], [u8; 32], [u8; 32], [u8; 32], u64),
}
impl MushroomThinkSource {
    /// Called only by the reserved native Think consumer, before its source fuse
    /// is retained. Native source-body admission and the actual actor are proved.
    pub fn from_current_reserved_think(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        occurrence: ThinkOccurrence,
    ) -> Result<Self, AttackError> {
        if !runtime.contains_live_creature(occurrence.actor)
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
        {
            return Err(AttackError::InvalidPlan);
        }
        let snapshot = runtime
            .companion_snapshot(occurrence.actor)
            .map_err(|_| AttackError::StaleIssuer)?;
        if snapshot.state.policy.definition_key != MUSHROOM
            || snapshot.state.master.is_some()
            || runtime.native_summon_master(occurrence.actor).is_some()
        {
            return Err(AttackError::InvalidPlan);
        }
        let pin = runtime.content_pin();
        let pins = (
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            runtime.map_revision_digest(),
            pin.activation_sequence(),
        );
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| AttackError::StaleIssuer)?;
        Ok(Self {
            actor: occurrence.actor,
            sequence: occurrence.sequence,
            stamp,
            pins,
        })
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.binding_current(runtime)
            && runtime.source_creature_is_registered(self.actor).ok() == Some(true)
            && runtime
                .companion_snapshot_including_dead(self.actor)
                .is_ok_and(|snapshot| {
                    snapshot.state.policy.definition_key == MUSHROOM
                        && snapshot.state.master.is_none()
                })
            && runtime.native_summon_master(self.actor).is_none()
    }
    fn binding_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
            && self.pins
                == (
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    runtime.map_revision_digest(),
                    pin.activation_sequence(),
                )
    }
    fn occurrence(&self, target: ExactActorRef) -> String {
        format!(
            "native:mushroom-fuse:{}:{}:{}",
            hex(&self.actor.placement_identity()),
            self.sequence,
            hex(&target.placement_identity())
        )
    }
    pub fn normal_damage_draw(&self, target: ExactActorRef) -> Result<u32, AttackError> {
        let text = self.occurrence(target);
        let hash = Sha256::new()
            .chain_update(b"oteryn:native-mushroom-fuse:v1")
            .chain_update(text.as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(self.pins.0),
            DecisionOccurrenceId::from_bytes(id),
            "source_target_damage",
            0,
        )
        .map_err(|_| AttackError::InvalidPlan)?;
        oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(draw, 2500, 3000)
            .ok_or(AttackError::NumericOverflow)
    }
    pub fn prepare_player(
        &self,
        runtime: &ChannelRuntimeV1,
        reader: &mut dyn SpellWorldReader,
        target: ExactActorRef,
        session: GameSessionId,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
    ) -> Result<NativeCreatureEventDamagePlan, AttackError> {
        if !self.current(runtime)
            || !runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(stamp))
        {
            return Err(AttackError::StaleIssuer);
        }
        let from = runtime
            .read_actor_position(self.actor)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = runtime
            .read_actor_position(target)
            .map_err(|_| AttackError::StaleTarget)?;
        let a = from.position();
        let b = to.position();
        if from.context() != to.context()
            || a.floor != b.floor
            || i64::from(a.x).abs_diff(i64::from(b.x)) > 3
            || i64::from(a.y).abs_diff(i64::from(b.y)) > 3
        {
            return Err(AttackError::OutOfRange);
        }
        let facts = reader
            .combat(
                runtime,
                self.actor,
                target,
                session,
                Some("life_drain"),
                stamp,
            )
            .ok_or(AttackError::MissingCombatFacts)?;
        let actual = facts.attack;
        if actual.issuer != self.actor
            || actual.target != target
            || actual.session != session
            || actual.revision == 0
            || facts.multiplier_ppm > 10_000_000
        {
            return Err(AttackError::InvalidPlan);
        }
        if runtime
            .player_control_facts(target, session)
            .map_err(|_| AttackError::StaleTarget)?
            .control_loss
            .is_some()
        {
            return Err(AttackError::StaleTarget);
        }
        if actual.issuer_pz
            || actual.target_pz
            || actual.issuer_protected
            || actual.target_protected
        {
            return Err(AttackError::Protected);
        }
        let requested = reader
            .source_combat_component_damage(
                runtime,
                self.actor,
                target,
                self.normal_damage_draw(target)?,
                now.get(),
                stamp,
            )
            .ok_or(AttackError::MissingCombatFacts)?;
        let child = self.occurrence(target);
        let hash = Sha256::digest(child.as_bytes());
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        let hazard = (deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(self.pins.0),
            DecisionOccurrenceId::from_bytes(id),
            "hazard_critical",
            0,
        )
        .map_err(|_| AttackError::InvalidPlan)?
            % 1_000_000) as u32;
        let hit = prepare_native_creature_player_hit(
            runtime,
            &facts,
            requested,
            0,
            false,
            Some("life_drain"),
            hazard,
            false,
            now,
        )?;
        Ok(NativeCreatureEventDamagePlan {
            source: self.clone(),
            target,
            session,
            stamp,
            revision: facts.attack.revision,
            hit,
            occurrence: child,
            reflection_committed: false,
            completed: None,
            critical_at: None,
            retained: false,
        })
    }
}

/// A current, source-qualified normal hit. Its fields cannot be manufactured by
/// a caller; the normal preparation above and canonical HP owner remain joined.
#[derive(Debug)]
pub struct NativeCreatureEventDamagePlan {
    source: MushroomThinkSource,
    target: ExactActorRef,
    session: GameSessionId,
    stamp: RuntimeWorkStamp,
    revision: u64,
    hit: PreparedNativeCreaturePlayerHit,
    occurrence: String,
    reflection_committed: bool,
    completed: Option<Option<PlayerDamageReceipt>>,
    critical_at: Option<u64>,
    retained: bool,
}
impl NativeCreatureEventDamagePlan {
    pub fn target(&self) -> (ExactActorRef, GameSessionId) {
        (self.target, self.session)
    }

    /// Accept the complete source callback under its unchanged current facts,
    /// before publishing any member of its captured spectator cohort.
    pub fn retain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        vitals: &dyn PlayerLethalVitals,
    ) -> Result<(), AttackError> {
        if !self.source.current(runtime)
            || !runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
        {
            return Err(AttackError::StaleIssuer);
        }
        if vitals
            .source_player_health_including_dead(runtime, self.target, self.session)
            .is_none_or(|(_, _, revision)| revision != self.revision)
        {
            return Err(AttackError::StaleTarget);
        }
        self.retained = true;
        Ok(())
    }

    pub fn commit(
        mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        now: SemanticTimeMicros,
    ) -> Result<Option<PlayerDamageReceipt>, AttackError> {
        self.commit_inner(runtime, vitals, now, false)
    }

    /// The source callback owns this plan until every synchronous leaf has a
    /// known result. Its reflection receipt uses the original Think sequence.
    pub fn commit_retained(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        now: SemanticTimeMicros,
    ) -> Result<Option<PlayerDamageReceipt>, AttackError> {
        if !self.retained {
            return Err(AttackError::InvalidPlan);
        }
        self.commit_inner(runtime, vitals, now, true)
    }

    fn commit_inner(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        now: SemanticTimeMicros,
        retained: bool,
    ) -> Result<Option<PlayerDamageReceipt>, AttackError> {
        if !self.source.binding_current(runtime)
            || !runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
        {
            return Err(AttackError::StaleIssuer);
        }
        if self.completed.is_some() {
            return self.finish_receipt(runtime);
        }
        if !self.source.current(runtime) {
            return Err(AttackError::StaleIssuer);
        }
        let (health, _, revision) = if retained {
            vitals.source_player_health_including_dead(runtime, self.target, self.session)
        } else {
            vitals.source_player_health(runtime, self.target, self.session)
        }
        .ok_or(AttackError::StaleTarget)?;
        if health == 0 {
            self.completed = Some(None);
            return Ok(None);
        }
        if !self.reflection_committed {
            // The retained callback already validated every target before its
            // first published effect. It cannot recapture a later revision or
            // recompute modifiers after another member has committed.
            if !retained && revision != self.revision {
                return Err(AttackError::StaleTarget);
            }
            if !retained || runtime.contains_live_creature(self.source.actor) {
                let reflected = if retained {
                    vitals.reflect_equipment_damage_at_occurrence(
                        runtime,
                        self.source.actor,
                        self.target,
                        self.session,
                        Some("life_drain"),
                        self.hit.hazard.amount,
                        self.occurrence.as_bytes(),
                        self.source.sequence,
                        now,
                    )
                } else {
                    vitals.reflect_equipment_damage(
                        runtime,
                        self.source.actor,
                        self.target,
                        self.session,
                        Some("life_drain"),
                        self.hit.hazard.amount,
                        self.occurrence.as_bytes(),
                        now,
                    )
                };
                if !reflected {
                    return Err(AttackError::MissingVitals);
                }
            }
            self.reflection_committed = true;
        }
        if self.hit.blocked || self.hit.hazard.amount == 0 {
            self.completed = Some(None);
            return Ok(None);
        }
        let receipt = vitals
            .apply_source_typed_attack_damage(
                runtime,
                self.source.actor,
                self.target,
                self.session,
                self.hit.hazard.amount,
                crate::player_lethal::PlayerHitKind::LifeDrain,
                &self.occurrence,
                now,
            )
            .ok_or(AttackError::MissingVitals)?;
        self.completed = Some(Some(receipt));
        if self.hit.hazard.critical {
            self.critical_at = Some(now.get());
        }
        self.finish_receipt(runtime)
    }

    fn finish_receipt(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<Option<PlayerDamageReceipt>, AttackError> {
        if let Some(at) = self.critical_at {
            runtime
                .record_hazard_critical(self.target, self.session, at)
                .map_err(|_| AttackError::StaleTarget)?;
            self.critical_at = None;
        }
        self.completed.ok_or(AttackError::InvalidPlan)
    }
}

/// The same normal elemental response used by composed native spell hits.
/// Damage-derived healing remains an explicit separate canonical heal operation.
pub fn prepare_native_creature_damage(
    policy: &crate::content::creature_policy::CompiledCreaturePolicy,
    element: &str,
    amount: u32,
) -> Option<u32> {
    if amount == 0 {
        return Some(0);
    }
    if policy.damage_healing(element, i64::from(amount)).ok()? != 0 {
        return None;
    }
    u32::try_from(
        oteryn_gameplay_combat_rules::resolution::creature_damage_response(
            policy,
            i64::from(amount),
            element,
        )?,
    )
    .ok()
}
