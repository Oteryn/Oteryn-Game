//! Prepared source events inside the existing periodic owner. No second HP store.
use super::*;
use crate::carrier::soul_war_health::{
    HealthSourceKind as S, apparition_vocation, source_rule_draw,
};
use crate::carrier::soul_war_player_source::SoulWarPlayerSourceOrigin;
use oteryn_gameplay_content::content::native_soul_war_callbacks::{
    QualifiedSoulWarHealthPolicy, SoulWarHealthCallbackKind as K,
};
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct PeriodicHealthEvents {
    callback: Option<Arc<QualifiedSoulWarHealthPolicy>>,
    pub(super) native: Option<super::super::native_health_change::NativeHealthChangeObservation>,
    pub(super) target_identity: Option<Arc<[u8]>>,
    pub(super) conditions: Option<runtime_actor_conditions::ActorConditionState>,
    hatred_defense: Option<u32>,
    stamp: RuntimeWorkStamp,
    pub(super) blood: Vec<super::super::soul_war_native_health_events::CommittedCloakBloodEvent>,
}
impl PeriodicHealthEvents {
    pub(super) fn new(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        stamp: RuntimeWorkStamp,
        observations: &[super::super::native_health_change::NativeHealthChangeObservation],
    ) -> Result<Self, CarrierError> {
        let mut matching = observations
            .iter()
            .filter(|observation| observation.target() == actor);
        let native = matching.next().cloned();
        if matching.next().is_some() {
            return Err(CarrierError::PlanConflict);
        }
        Ok(Self {
            native,
            callback: runtime
                .soul_war_health_callback(actor)?
                .cloned()
                .map(Arc::new),
            target_identity: None,
            conditions: None,
            hatred_defense: None,
            stamp,
            blood: Vec::new(),
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing periodic owner supplies the same tick, source, native type and HP successor for one atomic slot publication"
    )]
    pub(super) fn apply(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        after: &mut CompanionState,
        health: &mut i64,
        maximum: i64,
        tick: &ConditionTick<ExactActorRef>,
        tick_index: usize,
        damage: i64,
        now_us: u64,
    ) -> Result<i64, CarrierError> {
        if damage <= 0 {
            return Ok(damage);
        }
        // ConditionDamage invokes HEALTHCHANGE with ORIGIN_CONDITION. Transform
        // its original signed primary after target mitigation, once, before HP.
        // The direct Lua ORIGIN_NONE path does not enter this periodic owner.
        if self.native.is_none() {
            let unknown_names = |_: ExactActorRef, _: MovementPositionSnapshot| None;
            self.native = runtime.observe_native_health_change(actor, &unknown_names)?;
        }
        let damage = if let Some(observation) = &self.native {
            runtime.validate_native_health_change_observation(observation)?;
            observation
                .primary(damage.checked_neg().ok_or(CarrierError::DamageOverflow)?)?
                .checked_neg()
                .ok_or(CarrierError::DamageOverflow)?
        } else {
            damage
        };
        let Some(policy) = self.callback.as_ref() else {
            return Ok(damage);
        };
        let content = runtime.content_pin().server_artifact_digest();
        if !policy.matches(
            content,
            &after.policy.definition_key,
            &after.policy.definition_revision,
        ) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let source = runtime.carrier.soul_war_health_context(
            &runtime.continuity,
            tick.provenance.source,
            None,
            Some(now_us / 1000),
            SoulWarPlayerSourceOrigin::Condition,
            runtime.content_pin(),
        )?;
        use sha2::{Digest, Sha256};
        let occurrence = Sha256::new()
            .chain_update(b"oteryn:cloak-periodic-health-event:v1")
            .chain_update(self.stamp.generation().get().to_be_bytes())
            .chain_update(self.stamp.ordinal().get().to_be_bytes())
            .chain_update(actor.placement_identity())
            .chain_update(
                u64::try_from(tick_index)
                    .map_err(|_| CarrierError::CapacityArithmeticOverflow)?
                    .to_be_bytes(),
            )
            .chain_update(tick.sequence.to_be_bytes())
            .chain_update(tick.due.to_be_bytes())
            .chain_update(tick.provenance.definition_key.as_bytes())
            .chain_update(tick.provenance.definition_revision.to_be_bytes())
            .finalize()
            .into();
        if let Some(mut event) = runtime.carrier.stage_cloak_blood_event(
            &runtime.continuity,
            runtime.content_pin(),
            actor,
            &source,
            damage.checked_neg().ok_or(CarrierError::InvalidDamage)?,
            0,
            occurrence,
        )? {
            event.stamp = Some(self.stamp);
            self.blood
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            self.blood.push(event);
        }
        let mut binding = Vec::with_capacity(40);
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(&tick.sequence.to_be_bytes());
        binding.extend_from_slice(&tick.due.to_be_bytes());
        match policy.kind() {
            K::MirrorImage => match source.kind {
                S::Null | S::NonPlayer => Ok(damage.min(health.saturating_sub(1))),
                S::Player(None) => Err(CarrierError::PlanConflict),
                S::Player(Some(facts)) => {
                    let Some(apparition) = apparition_vocation(facts.vocation) else {
                        return Ok(damage);
                    };
                    let index = runtime.carrier.validate_ref(&runtime.continuity, actor.0)?;
                    let Slot::CreatureOccupied { spell_combat, .. } = &runtime.carrier.slots[index]
                    else {
                        return Err(CarrierError::NotCreature);
                    };
                    let native = spell_combat
                        .mirror_births
                        .as_deref()
                        .ok_or(CarrierError::PlanConflict)?
                        .policy(content, apparition)?;
                    if runtime.companion_policy(&native.definition_key)?.as_ref() != native.as_ref()
                    {
                        return Err(CarrierError::ContentPinWorldMismatch);
                    }
                    let epoch = after
                        .lifecycle_epoch
                        .checked_add(1)
                        .ok_or(CarrierError::CapacityArithmeticOverflow)?;
                    let identity =
                        Arc::from(copy_bounded_binding(native.definition_key.as_bytes())?);
                    // Lua setType(false) replaces type/look/speed, retains this exact
                    // physical actor, old health AND old maximum, and its conditions.
                    after.outfit_look_type = native.outfit_look_type;
                    after.object_look_type = native.object_look_type;
                    after.policy = native;
                    after.creation_speed_delta = 0;
                    after.lifecycle_epoch = epoch;
                    self.callback = runtime
                        .companion_policies
                        .as_ref()
                        .and_then(|table| {
                            table.soul_war_health_policy(
                                &after.policy.definition_key,
                                &after.policy.definition_revision,
                            )
                        })
                        .cloned()
                        .map(Arc::new);
                    self.target_identity = Some(identity);
                    Ok(damage)
                }
            },
            K::FourthTaint => {
                use oteryn_gameplay_domain::domain::soul_war_fourth_taint::{
                    self as fourth, Killer,
                };
                let killer = match source.kind {
                    S::Null => Killer::Null,
                    S::NonPlayer => Killer::NonPlayer,
                    S::Player(Some(facts)) => Killer::Player {
                        taints: facts.taints,
                        in_realm: facts.realm.is_some(),
                    },
                    S::Player(None) if damage < *health => Killer::Player {
                        taints: 0,
                        in_realm: false,
                    },
                    S::Player(None) => return Err(CarrierError::PlanConflict),
                };
                let mut invalid = false;
                let result =
                    fourth::resolve(*health, maximum, damage, killer, || match source_rule_draw(
                        &binding,
                        actor.0,
                        b"fourth-taint",
                        10,
                    ) {
                        Ok(draw) => draw + 1,
                        Err(_) => {
                            invalid = true;
                            0
                        }
                    })
                    .map_err(|_| CarrierError::InvalidDamage)?;
                if invalid {
                    return Err(CarrierError::InvalidCommitBinding);
                }
                *health = result.damage_before;
                Ok(result.damage)
            }
            K::HirintrorHeal | K::WhitePaleHeal if damage >= *health => {
                let index = runtime.carrier.validate_ref(&runtime.continuity, actor.0)?;
                let Slot::CreatureOccupied { committed, .. } = &runtime.carrier.slots[index] else {
                    return Err(CarrierError::NotCreature);
                };
                let current = self.conditions.as_ref().unwrap_or(&committed.conditions);
                if let Some(next) =
                    current.prepare_w01_lethal_heal(actor, content, &binding, now_us / 1000)?
                {
                    let healed = health
                        .checked_add(if policy.kind() == K::HirintrorHeal {
                            1000
                        } else {
                            400
                        })
                        .ok_or(CarrierError::DamageOverflow)?
                        .min(maximum);
                    self.conditions = Some(next);
                    *health = healed;
                    Ok(0)
                } else {
                    Ok(damage)
                }
            }
            K::PassiveHealth if policy.is_cruelty() && matches!(source.kind, S::Player(_)) => {
                // soul_war_mechanics.lua:1091-1098 runs on the already
                // mitigated component, including Player-origin condition ticks.
                let snapshot = runtime.companion_snapshot(actor)?;
                let drain = runtime.soul_war_cruelty_drain(&snapshot)?.unwrap_or(2);
                let scaled = damage as f64 * (1.0 - drain as f64 / 100.0).max(0.0);
                if !scaled.is_finite() || scaled < 0.0 || scaled >= i64::MAX as f64 {
                    return Err(CarrierError::DamageOverflow);
                }
                Ok(scaled.trunc() as i64)
            }
            K::PassiveHealth
                if policy.is_empowered_boss() && matches!(source.kind, S::Player(_)) =>
            {
                let index = runtime.carrier.validate_ref(&runtime.continuity, actor.0)?;
                let Slot::CreatureOccupied { spell_combat, .. } = &runtime.carrier.slots[index]
                else {
                    return Err(CarrierError::NotCreature);
                };
                // The existing prepared turn fences the entire original Slot.
                // Each consumed health tick stages its callback before HP clamp;
                // commit publishes only this successor delta with that same Slot.
                let mut staged = spell_combat.clone();
                if let Some(delta) = self.hatred_defense {
                    staged.soul_war_defense_delta = delta;
                }
                staged.stage_hatred_incoming_health(&after.policy, &source)?;
                self.hatred_defense = Some(staged.soul_war_defense_delta);
                Ok(damage)
            }
            K::HirintrorHeal | K::WhitePaleHeal | K::PassiveHealth => Ok(damage),
        }
    }
    pub(super) fn install(
        self,
        spell_combat: &mut runtime_actor_spell::ActorCombatState,
        committed: &mut runtime_actor_conditions::CreatureCommitState,
        target_identity: &mut Arc<[u8]>,
    ) -> Vec<super::super::soul_war_native_health_events::CommittedCloakBloodEvent> {
        if let Some(identity) = self.target_identity {
            *target_identity = identity;
            spell_combat.soul_war_health = self.callback;
            spell_combat.mirror_births = None;
        }
        if let Some(conditions) = self.conditions {
            committed.conditions = conditions;
        }
        if let Some(delta) = self.hatred_defense {
            spell_combat.soul_war_defense_delta = delta;
        }
        self.blood
    }
}
