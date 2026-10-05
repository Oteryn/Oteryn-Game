//! Spell adapters for Foundation-owned atomic transaction data.
//! Shared data never depends on a gameplay transport or authoring implementation.
pub(crate) use crate::foundation::runtime_actor_spell_types::*;

impl From<crate::ability::AbilityOccurrence> for SpellOccurrenceBinding {
    fn from(value: crate::ability::AbilityOccurrence) -> Self {
        let r = value.revisions();
        Self {
            id: value.id().as_str().to_owned(),
            revisions: [
                r.ruleset().to_owned(),
                r.content().to_owned(),
                r.world_policy().to_owned(),
                r.formula().to_owned(),
                r.simulation().to_owned(),
            ],
        }
    }
}

use super::native_combat::{Element, MagnitudePlan, NativeCombatPlan};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};

/// Resolved by the same live owner that produced the native target facts. Source IDs never
/// become actor authority: the physical owner independently checks the exact references.
#[derive(Debug, Clone)]
pub(crate) struct LiveActorBinding {
    pub(crate) source_id: u64,
    pub(crate) actor: ExactActorRef,
    pub(crate) target_atom: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TimedCombatEffect {
    pub(crate) delay_ms: u64,
    pub(crate) effect: OwnerCombatEffect,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CastOwnerChanges {
    pub(crate) cooldown_ms: Option<u64>,
    pub(crate) shared_cooldown: Option<(String, u64)>,
    pub(crate) reduce_all_spell_cooldowns_ms: u64,
    pub(crate) block_armor: bool,
    pub(crate) use_weapon_charges: bool,
    pub(crate) weapon_missile: bool,
    pub(crate) resolve_critical_and_fatal_once: bool,
}

#[derive(Debug)]
pub(crate) struct LoweredNativeCombat {
    pub(crate) batch: OwnerCombatBatch,
    pub(crate) delayed: Vec<TimedCombatEffect>,
    /// Must be consumed by the real caster/Item/magnitude owners before admission. These
    /// requirements are retained rather than silently discarded by the HP bridge.
    pub(crate) owner_changes: CastOwnerChanges,
    pub(crate) next_sub_ordinal: u16,
}

fn bound(bindings: &[LiveActorBinding], id: u64) -> Result<&LiveActorBinding, Error> {
    bindings
        .iter()
        .find(|b| b.source_id == id)
        .ok_or(Error::InvalidBatch)
}
fn append(
    batch: &mut OwnerCombatBatch,
    delayed: &mut Vec<TimedCombatEffect>,
    ordinal: &mut u16,
    actor: ExactActorRef,
    change: OwnerCombatChange,
    delay_ms: u64,
) -> Result<(), Error> {
    if usize::from(*ordinal) >= MAX_EFFECTS {
        return Err(Error::TooManyEffects);
    }
    let effect = OwnerCombatEffect {
        target: actor,
        sub_ordinal: *ordinal,
        change,
    };
    *ordinal = ordinal.checked_add(1).ok_or(Error::TooManyEffects)?;
    if delay_ms == 0 {
        batch.effects.push(effect);
    } else {
        delayed.push(TimedCombatEffect { delay_ms, effect });
    }
    Ok(())
}

/// Lower all twenty combat candidates into the existing physical/player transaction.
/// `finish` is the qualified shared magnitude pipeline, including Wheel bonus/side stages,
/// armor, resistance, extensions and rounding. There is deliberately no default mitigation
/// or critical policy here. It receives the complete plan, enabling one beam-wide extension
/// draw rather than accidentally rolling critical/fatal once per target.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_native(
    runtime: &ChannelRuntimeV1,
    mut batch: OwnerCombatBatch,
    caster_source_id: u64,
    plan: &NativeCombatPlan,
    bindings: &[LiveActorBinding],
    first_sub_ordinal: u16,
    finish: &mut dyn FnMut(
        &NativeCombatPlan,
        &MagnitudePlan,
        &LiveActorBinding,
    ) -> Result<i64, Error>,
) -> Result<LoweredNativeCombat, Error> {
    if !batch.effects.is_empty() || batch.deferred.is_some() || bindings.len() > MAX_EFFECTS {
        return Err(Error::InvalidBatch);
    }
    let caster = bound(bindings, caster_source_id)?;
    if caster.actor != batch.caster {
        return Err(Error::InvalidBatch);
    }
    runtime
        .player_control_facts(batch.caster, batch.command.game_session_id())
        .map_err(Error::Owner)?;
    for (index, binding) in bindings.iter().enumerate() {
        if binding.target_atom.is_empty()
            || binding.target_atom.contains('\0')
            || bindings[..index]
                .iter()
                .any(|other| other.source_id == binding.source_id || other.actor == binding.actor)
        {
            return Err(Error::InvalidBatch);
        }
        if !runtime.borrow_exact_actor_lookup().contains(binding.actor) {
            return Err(Error::Owner(
                crate::foundation::CarrierError::StaleActorGeneration,
            ));
        }
    }
    let caster_actor = batch.caster;
    let mut ordinal = first_sub_ordinal;
    let mut delayed = Vec::new();
    let mut owner_changes = CastOwnerChanges::default();
    let (hits, healing, dispels): (&[MagnitudePlan], bool, &[u64]) = match plan {
        NativeCombatPlan::Combat(combat) => {
            owner_changes = CastOwnerChanges {
                cooldown_ms: combat.cooldown_ms,
                shared_cooldown: combat.shared_cooldown.clone(),
                reduce_all_spell_cooldowns_ms: combat.reduce_all_spell_cooldowns_ms,
                block_armor: combat.block_armor,
                use_weapon_charges: combat.use_weapon_charges,
                weapon_missile: combat.weapon_missile,
                resolve_critical_and_fatal_once: combat.resolve_critical_and_fatal_once,
            };
            (
                &combat.hits,
                combat.element == Element::Healing,
                &combat.dispel_paralysis_targets,
            )
        }
        NativeCombatPlan::MassSpiritMend {
            heals,
            dispel_paralysis_targets,
            cooldown_ms,
            ..
        } => {
            owner_changes.cooldown_ms = Some(*cooldown_ms);
            (heals, true, dispel_paralysis_targets)
        }
        NativeCombatPlan::Avatar(avatar) => {
            owner_changes.cooldown_ms = Some(avatar.cooldown_ms);
            append(
                &mut batch,
                &mut delayed,
                &mut ordinal,
                caster_actor,
                OwnerCombatChange::Avatar(AvatarState {
                    expires_ms: avatar.expires_ms,
                    outfit_look_type: avatar.outfit_look_type,
                    incoming_reduction_percent: avatar.incoming_reduction_percent,
                    critical_chance_percent: avatar.critical_chance_percent,
                    critical_extra_percentage_points: avatar.critical_extra_percentage_points,
                }),
                0,
            )?;
            (&[], false, &[])
        }
        NativeCombatPlan::ManaShield {
            capacity,
            expires_ms,
            ..
        } => {
            append(
                &mut batch,
                &mut delayed,
                &mut ordinal,
                caster_actor,
                OwnerCombatChange::ManaShield(ManaShieldState {
                    capacity: *capacity,
                    expires_ms: *expires_ms,
                }),
                0,
            )?;
            (&[], false, &[])
        }
        NativeCombatPlan::MonsterAi {
            overrides,
            cooldown_ms,
        } => {
            owner_changes.cooldown_ms = *cooldown_ms;
            for ai in overrides {
                let target = bound(bindings, ai.target)?;
                let challenged_to = match (ai.challenged_to, ai.challenge_expires_ms) {
                    (Some(id), Some(expires)) => Some((bound(bindings, id)?.actor, expires)),
                    (None, None) => None,
                    _ => return Err(Error::InvalidCondition),
                };
                let forced_distance = match (ai.forced_distance, ai.forced_distance_expires_ms) {
                    (Some(distance), Some(expires)) => Some((distance, expires)),
                    (None, None) => None,
                    _ => return Err(Error::InvalidCondition),
                };
                append(
                    &mut batch,
                    &mut delayed,
                    &mut ordinal,
                    target.actor,
                    OwnerCombatChange::MonsterAi(MonsterAiState {
                        forced_distance,
                        challenged_to,
                    }),
                    ai.delay_ms,
                )?;
            }
            (&[], false, &[])
        }
    };
    for hit in hits {
        let target = bound(bindings, hit.target)?;
        let magnitude = finish(plan, hit, target)?;
        if magnitude < 0 {
            return Err(Error::InvalidMagnitude);
        }
        if magnitude == 0 {
            continue;
        }
        let change = if healing {
            OwnerCombatChange::Heal {
                target_atom: target.target_atom.clone(),
                magnitude,
            }
        } else {
            OwnerCombatChange::Damage {
                target_atom: target.target_atom.clone(),
                magnitude,
            }
        };
        append(
            &mut batch,
            &mut delayed,
            &mut ordinal,
            target.actor,
            change,
            hit.delay_ms,
        )?;
    }
    for id in dispels {
        append(
            &mut batch,
            &mut delayed,
            &mut ordinal,
            bound(bindings, *id)?.actor,
            OwnerCombatChange::DispelParalysis,
            0,
        )?;
    }
    Ok(LoweredNativeCombat {
        batch,
        delayed,
        owner_changes,
        next_sub_ordinal: ordinal,
    })
}
