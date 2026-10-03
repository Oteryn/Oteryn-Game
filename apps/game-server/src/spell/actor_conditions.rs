//! Mutators of the real PlayerSpellState condition/vitals successor. The owning
//! Channel transaction qualifies actors and commits its final revision once.
use super::cast::PlayerSpellState;
use super::combat_batch::OwnerCombatChange;
use super::executable_catalog::EffectProfile;
use crate::ability::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionTick, ConditionType,
    ConditionValues, TickFacts, TickKind,
};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

pub(crate) fn has_periodic_damage(state: &PlayerSpellState) -> bool {
    state.conditions.instances().iter().any(|instance| {
        matches!(
            instance.definition().condition_type(),
            ConditionType::DamageOverTime(_)
        )
    })
}

fn rejected<T>() -> Result<T, SpellCastDisposition> {
    Err(SpellCastDisposition::Rejected)
}
fn now_us(now_ms: u64) -> Result<u64, SpellCastDisposition> {
    now_ms
        .checked_mul(1000)
        .ok_or(SpellCastDisposition::Rejected)
}
fn apply_definition(
    state: &mut PlayerSpellState,
    definition: ConditionDefinition,
    now_ms: u64,
    shield_capacity: u32,
) -> Result<(), SpellCastDisposition> {
    // All definitions admitted here are fixed and make no random decision. This
    // inert context cannot grant authority; the outer owner already proves it.
    let root = GameplayDecisionRoot::from_bytes([0; 32]);
    let facts = ApplicationFacts {
        now: now_us(now_ms)?,
        base_speed: u16::try_from(state.base_speed).map_err(|_| SpellCastDisposition::Rejected)?,
        mana_shield_capacity: shield_capacity,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
    };
    match state
        .conditions
        .apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts)
    {
        Ok(_) | Err(crate::ability::condition::ConditionRefusal::KeptCurrent) => Ok(()),
        Err(_) => Err(SpellCastDisposition::Rejected),
    }
}
fn definition(
    key: &str,
    values: ConditionValues,
) -> Result<ConditionDefinition, SpellCastDisposition> {
    ConditionDefinition::new(key, 1, values).ok_or(SpellCastDisposition::Rejected)
}
pub(crate) fn supports_condition_change(change: &OwnerCombatChange) -> bool {
    match change {
        OwnerCombatChange::Avatar(_)
        | OwnerCombatChange::ManaShield(_)
        | OwnerCombatChange::ConsumeManaShield { .. }
        | OwnerCombatChange::DispelParalysis => true,
        OwnerCombatChange::PlayerConditions { .. }
        | OwnerCombatChange::Damage { .. }
        | OwnerCombatChange::Heal { .. }
        | OwnerCombatChange::MonsterAi(_)
        | OwnerCombatChange::CompanionMaster(_)
        | OwnerCombatChange::CompanionConditions(_) => false,
    }
}
pub(crate) fn apply_combat_change(
    state: &mut PlayerSpellState,
    change: &OwnerCombatChange,
    now_ms: u64,
) -> Result<(), SpellCastDisposition> {
    let mut staged = state.clone();
    match change {
        OwnerCombatChange::Avatar(v) => {
            let duration_ms = u32::try_from(
                v.expires_ms
                    .checked_sub(now_ms)
                    .ok_or(SpellCastDisposition::Rejected)?,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            apply_definition(
                &mut staged,
                definition(
                    "spell.avatar.attributes",
                    ConditionValues::Attributes {
                        duration_ms,
                        critical_chance_percent: v.critical_chance_percent,
                        critical_extra_percentage_points: v.critical_extra_percentage_points,
                        damage_dealt_percent: 100,
                        incoming_reduction_percent: v.incoming_reduction_percent,
                    },
                )?,
                now_ms,
                0,
            )?;
            apply_definition(
                &mut staged,
                definition(
                    "spell.avatar.outfit",
                    ConditionValues::Outfit {
                        duration_ms,
                        look_type: v.outfit_look_type,
                    },
                )?,
                now_ms,
                0,
            )?;
        }
        OwnerCombatChange::ManaShield(v) => {
            if v.capacity == 0 || v.capacity > staged.facts.max_mana {
                return rejected();
            }
            let duration_ms = u32::try_from(
                v.expires_ms
                    .checked_sub(now_ms)
                    .ok_or(SpellCastDisposition::Rejected)?,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            apply_definition(
                &mut staged,
                definition(
                    "spell.mana_shield",
                    ConditionValues::ManaShield { duration_ms },
                )?,
                now_ms,
                v.capacity,
            )?;
        }
        OwnerCombatChange::ConsumeManaShield {
            expected_capacity,
            amount,
        } => {
            let mana = staged
                .mana
                .checked_sub(*amount)
                .ok_or(SpellCastDisposition::NotEnoughMana)?;
            staged
                .conditions
                .consume_mana_shield(now_us(now_ms)?, *expected_capacity, *amount)
                .map_err(|_| SpellCastDisposition::Rejected)?;
            staged.mana = mana;
            if mana == 0 {
                staged.conditions.remove_type(ConditionType::ManaShield);
            }
        }
        OwnerCombatChange::DispelParalysis => {
            staged.conditions.remove_type(ConditionType::Paralysis);
        }
        OwnerCombatChange::PlayerConditions { .. }
        | OwnerCombatChange::Damage { .. }
        | OwnerCombatChange::Heal { .. }
        | OwnerCombatChange::MonsterAi(_)
        | OwnerCombatChange::CompanionMaster(_)
        | OwnerCombatChange::CompanionConditions(_) => return rejected(),
    }
    *state = staged;
    Ok(())
}
/// The compiler supplies the complete immutable EffectProfile. Refuse every
/// field outside these supported condition shapes before changing actor state.
pub(crate) fn apply_effect(
    state: &mut PlayerSpellState,
    profile: &EffectProfile,
    now_ms: u64,
) -> Result<(), SpellCastDisposition> {
    let dependencies = super::executable_catalog::DependencyProfile {
        abilities: Vec::new(),
        effects: Vec::new(),
        formulas: Vec::new(),
    };
    let definition = condition_definition(profile, &dependencies)?;
    let mut staged = state.clone();
    apply_definition(&mut staged, definition, now_ms, 0)?;
    *state = staged;
    Ok(())
}

fn source_condition_kind(
    key: &str,
) -> Result<crate::ability::condition::ConditionType, SpellCastDisposition> {
    use crate::ability::condition::{ConditionType as T, DotElement as E};
    Ok(match key {
        "paralyze" => T::Paralysis,
        "manashield" => T::ManaShield,
        "poison" => T::DamageOverTime(E::Poison),
        "fire" => T::DamageOverTime(E::Fire),
        "energy" => T::DamageOverTime(E::Energy),
        "bleeding" => T::DamageOverTime(E::Bleeding),
        "cursed" => T::DamageOverTime(E::Cursed),
        "dazzled" => T::DamageOverTime(E::Dazzled),
        "invisible" => T::Invisible,
        _ => return rejected(),
    })
}
pub(crate) fn condition_definition(
    profile: &EffectProfile,
    dependencies: &crate::spell::executable_catalog::DependencyProfile,
) -> Result<crate::ability::condition::ConditionDefinition, SpellCastDisposition> {
    use crate::ability::condition::{
        ConditionDefinition, ConditionValues as V, DotSequenceStep, MAX_DOT_SEQUENCE_STEPS,
        SpeedRange,
    };
    if profile.operation != "condition"
        || profile.formula.is_some()
        || profile.damage_type.is_some()
        || profile.created_item.is_some()
        || profile.removed_condition.is_some()
        || profile.mitigated_by.is_some()
        || profile.pvp_safe_item.is_some()
        || profile.duration_range_ms.is_some()
        || profile.duration_selection.is_some()
        || profile.safe_world_type.is_some()
        || profile.refuse_on.is_some()
        || profile.description_template.is_some()
    {
        return rejected();
    }
    let c = profile
        .condition
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let shapes = [
        c.attribute_modifiers.is_some(),
        c.regeneration.is_some(),
        c.light.is_some(),
        c.speed_formula.is_some(),
        c.damage_over_time.is_some(),
    ];
    if shapes.iter().filter(|v| **v).count() != usize::from(c.kind != "invisible") {
        return rejected();
    }
    let duration = || profile.duration_ms.ok_or(SpellCastDisposition::Rejected);
    let values = if let Some(dot) = &c.damage_over_time {
        if c.lifetime != "damage_schedule"
            || profile.duration_ms.is_some()
            || dot.tick_profile != "fixed"
            || dot.first_tick != "after_interval"
            || dot.fixed_ticks.len() > MAX_DOT_SEQUENCE_STEPS
        {
            return rejected();
        }
        let crate::ability::condition::ConditionType::DamageOverTime(element) =
            source_condition_kind(&c.kind)?
        else {
            return rejected();
        };
        let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
        for (slot, tick) in steps.iter_mut().zip(&dot.fixed_ticks) {
            *slot = DotSequenceStep {
                amount: u32::try_from(tick.amount).map_err(|_| SpellCastDisposition::Rejected)?,
                interval_ms: tick.interval_ms,
                repetitions: tick.count,
            };
        }
        V::DamageSequence {
            element,
            steps,
            len: u8::try_from(dot.fixed_ticks.len()).map_err(|_| SpellCastDisposition::Rejected)?,
            delayed: true,
        }
    } else {
        if c.lifetime != "fixed_duration" {
            return rejected();
        }
        match c.kind.as_str() {
            "invisible" => V::Invisible {
                duration_ms: duration()?,
            },
            "light" => {
                let l = c.light.as_ref().ok_or(SpellCastDisposition::Rejected)?;
                V::SpellLight {
                    duration_ms: duration()?,
                    level: u8::try_from(l.level).map_err(|_| SpellCastDisposition::Rejected)?,
                    color: u8::try_from(l.color).map_err(|_| SpellCastDisposition::Rejected)?,
                }
            }
            "regeneration" => {
                let r = c
                    .regeneration
                    .as_ref()
                    .ok_or(SpellCastDisposition::Rejected)?;
                V::SpellRegeneration {
                    sub_id: qualified_regeneration_sub_id(profile)?,
                    duration_ms: duration()?,
                    health_gain: r.health_gain.unwrap_or(0),
                    health_interval_ms: r
                        .health_interval_ms
                        .or(r.mana_interval_ms)
                        .ok_or(SpellCastDisposition::Rejected)?,
                    mana_gain: r.mana_gain.unwrap_or(0),
                    mana_interval_ms: r
                        .mana_interval_ms
                        .or(r.health_interval_ms)
                        .ok_or(SpellCastDisposition::Rejected)?,
                }
            }
            "attributes" => {
                let sub_id = qualified_skill_sub_id(profile)?;
                let mut magic_level = 0;
                let mut fist = 0;
                let mut melee = 0;
                let mut distance = 0;
                let mut shielding = 0;
                let mut seen = std::collections::BTreeSet::new();
                for modifier in c
                    .attribute_modifiers
                    .as_ref()
                    .ok_or(SpellCastDisposition::Rejected)?
                {
                    if modifier.mode != "add" || !seen.insert(modifier.attribute.as_str()) {
                        return rejected();
                    }
                    let value = i32::try_from(modifier.value)
                        .map_err(|_| SpellCastDisposition::Rejected)?;
                    match modifier.attribute.as_str() {
                        "stat_magicpoints" => magic_level = value,
                        "skill_fist" => fist = value,
                        "skill_melee" => melee = value,
                        "skill_distance" => distance = value,
                        "skill_shield" => shielding = value,
                        _ => return rejected(),
                    }
                }
                V::SpellSkills {
                    duration_ms: duration()?,
                    sub_id,
                    magic_level,
                    fist,
                    melee,
                    distance,
                    shielding,
                }
            }
            "paralyze" => {
                let reference = c
                    .speed_formula
                    .as_ref()
                    .ok_or(SpellCastDisposition::Rejected)?;
                if reference.family != "Formula" {
                    return rejected();
                }
                let f = dependencies
                    .formulas
                    .iter()
                    .find(|f| {
                        f.identity.key == reference.key && f.identity.revision == reference.revision
                    })
                    .filter(|f| {
                        f.kind == "speed_modifier"
                            && f.inputs.is_none()
                            && f.minimum.is_none()
                            && f.maximum.is_none()
                    })
                    .and_then(|f| f.speed.as_ref())
                    .ok_or(SpellCastDisposition::Rejected)?;
                let coeff=|ratio:&crate::spell::executable_catalog::Fraction| -> Result<i32,SpellCastDisposition> {
                    if ratio.denominator==0 { return rejected(); }
                    let scaled=i128::from(ratio.numerator)*1000;
                    let denominator=i128::from(ratio.denominator);
                    if scaled%denominator!=0 { return rejected(); }
                    i32::try_from(scaled/denominator).map_err(|_|SpellCastDisposition::Rejected)
                };
                V::Speed {
                    paralysis: true,
                    duration_ms: duration()?,
                    range: SpeedRange {
                        a_min: coeff(&f.minimum_multiplier)?,
                        a_max: coeff(&f.maximum_multiplier)?,
                        b_min: i32::try_from(f.minimum_offset)
                            .map_err(|_| SpellCastDisposition::Rejected)?,
                        b_max: i32::try_from(f.maximum_offset)
                            .map_err(|_| SpellCastDisposition::Rejected)?,
                    },
                }
            }
            _ => return rejected(),
        }
    };
    ConditionDefinition::new(&profile.identity.key, 1, values).ok_or(SpellCastDisposition::Rejected)
}

// Frozen Canary party files explicitly set attributes subid1/2/3. The compiler's
// admitted effects omit that field, so qualify the captured identity, not modifier shape.
fn qualified_skill_sub_id(profile: &EffectProfile) -> Result<u8, SpellCastDisposition> {
    if profile.identity.revision != "spell-p2-r20" {
        return rejected();
    }
    match profile.identity.key.as_str() {
        "candidate:spell/train_party/effect-attributes" => Ok(1),
        "candidate:spell/protect_party/effect-attributes" => Ok(2),
        "candidate:spell/enchant_party/effect-attributes" => Ok(3),
        // Other attributes need a source-qualified subid; absence never guesses coexistence.
        _ => rejected(),
    }
}
fn qualified_regeneration_sub_id(profile: &EffectProfile) -> Result<u8, SpellCastDisposition> {
    match profile.identity.key.as_str() {
        "candidate:spell/heal_party/effect-regeneration"
        | "candidate:spell/enlighten_party/effect-regeneration" => {
            if profile.identity.revision == "spell-p2-r20" {
                Ok(1)
            } else {
                rejected()
            }
        }
        _ => Ok(0),
    }
}

pub(crate) fn apply_appearance(
    state: &mut PlayerSpellState,
    selection: crate::domain::appearance::AppearanceSelection<String>,
    look_type: u32,
    duration_ms: u32,
    now_ms: u64,
) -> Result<(), SpellCastDisposition> {
    let definition = definition(
        "spell.temporary.outfit",
        ConditionValues::Outfit {
            duration_ms,
            look_type,
        },
    )?
    .with_appearance(selection)
    .ok_or(SpellCastDisposition::Rejected)?;
    let mut staged = state.clone();
    apply_definition(&mut staged, definition, now_ms, 0)?;
    *state = staged;
    Ok(())
}
pub(crate) fn remove_condition(
    state: &mut PlayerSpellState,
    name: &str,
) -> Result<bool, SpellCastDisposition> {
    use crate::ability::condition::DotElement;
    let kind = match name {
        "poison" => ConditionType::DamageOverTime(DotElement::Poison),
        "fire" => ConditionType::DamageOverTime(DotElement::Fire),
        "energy" => ConditionType::DamageOverTime(DotElement::Energy),
        "bleeding" => ConditionType::DamageOverTime(DotElement::Bleeding),
        "drown" => ConditionType::DamageOverTime(DotElement::Drown),
        "freezing" => ConditionType::DamageOverTime(DotElement::Freezing),
        "dazzled" => ConditionType::DamageOverTime(DotElement::Dazzled),
        "cursed" => ConditionType::DamageOverTime(DotElement::Cursed),
        "paralysis" | "paralyze" => ConditionType::Paralysis,
        _ => return rejected(),
    };
    Ok(state.conditions.remove_type(kind))
}
/// Returns every non-regeneration tick, including refused DOT ticks, to the
/// actual combat owner. It never directly applies periodic damage to player HP.
pub(crate) fn tick(
    state: &mut PlayerSpellState,
    now_ms: u64,
    facts: TickFacts,
) -> Result<Vec<ConditionTick<String>>, SpellCastDisposition> {
    tick_inner(state, now_ms, facts, true)
}
fn tick_inner(
    state: &mut PlayerSpellState,
    now_ms: u64,
    facts: TickFacts,
    damage: bool,
) -> Result<Vec<ConditionTick<String>>, SpellCastDisposition> {
    let now = now_us(now_ms)?;
    if !state.conditions.accepts_time(now)
        || state.health > state.facts.max_health
        || state.mana > state.facts.max_mana
    {
        return rejected();
    }
    let mut staged = state.clone();
    let mut combat = Vec::new();
    let ticks = if damage {
        staged.conditions.take_due(now, facts)
    } else {
        staged.conditions.take_due_non_damage(now, facts)
    };
    for tick in ticks {
        match tick.kind {
            TickKind::SpellRegeneration {
                health_gain,
                mana_gain,
                suppressed,
            } => {
                if !suppressed && staged.health > 0 {
                    staged.health = staged
                        .health
                        .saturating_add(health_gain)
                        .min(staged.facts.max_health);
                    staged.mana = staged
                        .mana
                        .saturating_add(mana_gain)
                        .min(staged.facts.max_mana);
                }
            }
            TickKind::Damage { .. } | TickKind::Regeneration { .. } => combat.push(tick),
        }
    }
    *state = staged;
    Ok(combat)
}
pub(crate) fn clear_on_lifecycle(state: &mut PlayerSpellState) {
    state.field_attack_history.clear_on_lifecycle();
    state.conditions.clear_on_death();
    state.movement_pacing.clear_pending();
}

/// The existing creature-bite vitals owner applies its already qualified damage to the
/// actual player conditions. The HP floor remains D54; shield mana and capacity change
/// together with HP and exactly one player revision.
pub(crate) fn stage_creature_hit(
    state: &PlayerSpellState,
    magnitude: u32,
    now: u64,
) -> Result<
    (
        PlayerSpellState,
        crate::ability::creature_bite::FlooredDamage,
    ),
    SpellCastDisposition,
> {
    use crate::ability::condition::ConflictKey;
    if !state.conditions.accepts_time(now)
        || state.health == 0
        || state.health > state.facts.max_health
        || state.mana > state.facts.max_mana
    {
        return rejected();
    }
    let mut next = state.clone();
    let mut remaining = magnitude;
    if let Some(attributes) = next.conditions.active_at(ConflictKey::Attributes, now)
        && let ConditionValues::Attributes {
            incoming_reduction_percent,
            ..
        } = attributes.definition().values()
    {
        if magnitude > i32::MAX as u32 || incoming_reduction_percent > 100 {
            return rejected();
        }
        // Pinned Avatar removes ceil(hit * percent / 100), then retains the remainder.
        let removed = (u64::from(magnitude) * u64::from(incoming_reduction_percent)).div_ceil(100);
        remaining = magnitude
            .checked_sub(u32::try_from(removed).map_err(|_| SpellCastDisposition::Rejected)?)
            .ok_or(SpellCastDisposition::Rejected)?;
    }
    if let Some(capacity) = next.conditions.mana_shield_at(now) {
        let absorbed = remaining.min(capacity).min(next.mana);
        if absorbed > 0 {
            next.conditions
                .consume_mana_shield(now, capacity, absorbed)
                .map_err(|_| SpellCastDisposition::Rejected)?;
            next.mana -= absorbed;
            remaining -= absorbed;
        }
        if next.mana == 0 {
            next.conditions.remove_type(ConditionType::ManaShield);
        }
    }
    let damage = crate::ability::creature_bite::floor_creature_damage(next.health, remaining);
    next.health = damage.health_after;
    if next != *state {
        next.advance_batch_revision()?;
    }
    Ok((next, damage))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::items_after_test_module)]
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::super::Vocation;
    use super::super::cast::CharacterCastFacts;
    use super::super::combat_batch::{AvatarState, ManaShieldState};
    use super::*;
    fn actor() -> PlayerSpellState {
        PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap()
    }
    #[test]
    fn all_qualified_catalog_condition_shapes_convert_to_actual_store_definitions() {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))
        .unwrap();
        let mut checked = 0;
        for bundle in catalog["bundles"].as_array().unwrap() {
            let dependencies: crate::spell::executable_catalog::DependencyProfile =
                serde_json::from_value(bundle["dependencies"].clone()).unwrap();
            for effect in &dependencies.effects {
                if effect.condition.is_some() {
                    assert!(
                        condition_definition(effect, &dependencies).is_ok(),
                        "{}",
                        effect.identity.key
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 19);
    }
    #[test]
    fn enchant_live_modifier_expires_without_rewriting_base_magic_or_training() {
        let mut state = actor();
        let build = crate::durability::character_build::DurableBuildState::new(
            "knight",
            (10, 123),
            [(50, 456); 7],
        )
        .unwrap();
        state.training =
            Some(super::super::mana_training::LiveManaTraining::from_owned_build(&build));
        let original_facts = state.character_facts();
        let original_training = state.training.clone();
        let profile:EffectProfile=serde_json::from_value(serde_json::json!({"identity":{"key":"candidate:spell/enchant_party/effect-attributes","revision":"spell-p2-r20"},"operation":"condition","duration_ms":120000,"condition":{"type":"attributes","lifetime":"fixed_duration","buff_spell":true,"attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"add","value":1}]}})).unwrap();
        apply_effect(&mut state, &profile, 0).unwrap();
        assert_eq!(state.owned_effective_magic_level(119999000), Ok(11));
        assert_eq!(state.owned_effective_magic_level(120000000), Ok(10));
        assert_eq!(state.character_facts(), original_facts);
        assert_eq!(state.training, original_training);
        let mut ambiguous = profile.clone();
        ambiguous.identity.key = "candidate:spell/unqualified/effect-attributes".into();
        let before = state.clone();
        assert_eq!(
            apply_effect(&mut state, &ambiguous, 0),
            Err(SpellCastDisposition::Rejected)
        );
        assert_eq!(state, before);
    }
    #[test]
    fn shield_actual_store_and_correlated_mana_are_one_staged_write() {
        let mut state = actor();
        apply_combat_change(
            &mut state,
            &OwnerCombatChange::ManaShield(ManaShieldState {
                capacity: 300,
                expires_ms: 2000,
            }),
            0,
        )
        .unwrap();
        let before = state.clone();
        assert_eq!(
            apply_combat_change(
                &mut state,
                &OwnerCombatChange::ConsumeManaShield {
                    expected_capacity: 299,
                    amount: 50
                },
                1000
            ),
            Err(SpellCastDisposition::Rejected)
        );
        assert_eq!(state, before);
        apply_combat_change(
            &mut state,
            &OwnerCombatChange::ConsumeManaShield {
                expected_capacity: 300,
                amount: 50,
            },
            1000,
        )
        .unwrap();
        assert_eq!(state.mana, 450);
        assert_eq!(state.conditions.mana_shield_at(1000000), Some(250));
        assert_eq!(state.conditions.mana_shield_at(2000000), None);
        assert_eq!(state.revision, before.revision);
    }
    #[test]
    fn light_reads_real_level_color_and_linear_expiry_without_draining_ticks() {
        let mut state = actor();
        apply_definition(
            &mut state,
            definition(
                "spell.light",
                ConditionValues::SpellLight {
                    level: 6,
                    color: 215,
                    duration_ms: 60000,
                },
            )
            .unwrap(),
            0,
            0,
        )
        .unwrap();
        assert_eq!(state.conditions.light_at(0), Some((6, Some(215))));
        assert_eq!(state.conditions.light_at(10000000), Some((5, Some(215))));
        assert_eq!(state.conditions.light_at(59999999), Some((1, Some(215))));
        assert_eq!(state.conditions.light_at(60000000), None);
        let before = state.clone();
        assert!(
            apply_combat_change(
                &mut state,
                &OwnerCombatChange::ManaShield(ManaShieldState {
                    capacity: 0,
                    expires_ms: 1000
                }),
                0
            )
            .is_err()
        );
        assert_eq!(state, before);
    }
    #[test]
    fn avatar_outfit_and_attributes_expire_and_swift_does_not_erase_outfit() {
        let mut state = actor();
        apply_combat_change(
            &mut state,
            &OwnerCombatChange::Avatar(AvatarState {
                expires_ms: 15000,
                outfit_look_type: 1590,
                incoming_reduction_percent: 80,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
            }),
            0,
        )
        .unwrap();
        assert_eq!(state.conditions.outfit_at(14999999), Some(1590));
        assert!(state.conditions.attributes_at(15000000).is_none());
        assert!(state.conditions.outfit_at(15000000).is_none());
        apply_definition(
            &mut state,
            definition(
                "spell.swift.attributes",
                ConditionValues::Attributes {
                    duration_ms: 10000,
                    critical_chance_percent: 0,
                    critical_extra_percentage_points: 0,
                    damage_dealt_percent: 70,
                    incoming_reduction_percent: 0,
                },
            )
            .unwrap(),
            6000,
            0,
        )
        .unwrap();
        assert_eq!(state.conditions.outfit_at(14000000), Some(1590));
        assert!(matches!(
            state.conditions.attributes_at(14000000),
            Some(ConditionValues::Attributes {
                damage_dealt_percent: 70,
                critical_chance_percent: 0,
                ..
            })
        ));
    }
    #[test]
    fn real_regeneration_dual_clocks_pz_suppression_and_bounded_catchup() {
        let mut state = actor();
        state.health = 100;
        state.mana = 100;
        apply_definition(
            &mut state,
            definition(
                "spell.recovery",
                ConditionValues::SpellRegeneration {
                    sub_id: 0,
                    duration_ms: 10000,
                    health_gain: 20,
                    health_interval_ms: 3000,
                    mana_gain: 7,
                    mana_interval_ms: 2000,
                },
            )
            .unwrap(),
            0,
            0,
        )
        .unwrap();
        assert!(
            tick(&mut state, 3000, TickFacts::default())
                .unwrap()
                .is_empty()
        );
        assert_eq!((state.health, state.mana), (120, 107));
        tick(
            &mut state,
            6000,
            TickFacts {
                in_protection_zone: true,
                standing_on_field: None,
            },
        )
        .unwrap();
        assert_eq!((state.health, state.mana), (120, 107));
        tick(&mut state, 10000, TickFacts::default()).unwrap();
        assert_eq!((state.health, state.mana), (140, 121));
        assert!(state.conditions.instances().is_empty());
        let revision = state.revision;
        clear_on_lifecycle(&mut state);
        assert_eq!(state.revision, revision);
    }
}

pub(crate) fn pacing_snapshot(state: &PlayerSpellState) -> crate::movement::speed::StepPacing {
    state.movement_pacing.clone()
}
pub(crate) fn replace_pacing(
    state: &mut PlayerSpellState,
    pacing: crate::movement::speed::StepPacing,
) {
    state.movement_pacing = pacing;
}
pub(crate) fn has_speed_condition(state: &PlayerSpellState, now: u64) -> bool {
    state
        .conditions
        .active_at(crate::ability::condition::ConflictKey::Speed, now)
        .is_some()
}
pub(crate) fn movement_speed(state: &PlayerSpellState, now: u64, equipment_delta: i32) -> u16 {
    crate::movement::speed::effective_speed(
        state.base_speed,
        state.conditions.speed_delta_at(now),
        equipment_delta,
    )
}

/// The actual actor owner must lower every combat tick before installing this
/// complete successor. The struct itself grants no periodic damage authority.
pub(crate) struct ConditionCycle {
    pub(crate) next: PlayerSpellState,
    pub(crate) combat_ticks: Vec<ConditionTick<String>>,
    pub(crate) publish_vitals: bool,
}
pub(crate) fn stage_owner_cycle(
    state: &PlayerSpellState,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    facts: Option<TickFacts>,
) -> Result<Option<ConditionCycle>, SpellCastDisposition> {
    stage_cycle(state, now, facts, true, true, None)
}
pub(crate) fn stage_owner_cycle_without_damage(
    state: &PlayerSpellState,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    facts: TickFacts,
) -> Result<Option<ConditionCycle>, SpellCastDisposition> {
    stage_cycle(state, now, Some(facts), false, true, None)
}
/// Source candidates require a genuine current party/no-party owner. Unknown
/// party facts skip Serene evaluation; they never manufacture SoloParty.
pub(crate) fn stage_source_owner_cycle(
    state: &PlayerSpellState,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    facts: Option<TickFacts>,
    world: Option<&dyn super::harmony::SereneWorld>,
) -> Result<Option<ConditionCycle>, SpellCastDisposition> {
    stage_cycle(state, now, facts, true, false, world)
}
pub(crate) fn stage_source_owner_cycle_without_damage(
    state: &PlayerSpellState,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    facts: TickFacts,
    world: Option<&dyn super::harmony::SereneWorld>,
) -> Result<Option<ConditionCycle>, SpellCastDisposition> {
    stage_cycle(state, now, Some(facts), false, false, world)
}
fn stage_cycle(
    state: &PlayerSpellState,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    facts: Option<TickFacts>,
    damage: bool,
    legacy_solo: bool,
    world: Option<&dyn super::harmony::SereneWorld>,
) -> Result<Option<ConditionCycle>, SpellCastDisposition> {
    let mut next = state.clone();
    if let Some(world) = world {
        next.tick_with_world(now, world)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    } else if legacy_solo {
        next.tick(now).map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let combat_ticks = if let Some(facts) = facts {
        tick_inner(&mut next, now.get() / 1000, facts, damage)?
    } else {
        next.conditions
            .expire_non_ticking_checked(now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?;
        Vec::new()
    };
    if &next == state && combat_ticks.is_empty() {
        return Ok(None);
    }
    let publish_vitals = next.vitals() != state.vitals()
        || next.conditions.instances() != state.conditions.instances();
    next.revision = if publish_vitals {
        state
            .revision
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?
    } else {
        state.revision
    };
    Ok(Some(ConditionCycle {
        next,
        combat_ticks,
        publish_vitals,
    }))
}

/// A qualified source presentation operation carries no actor mutation. Its
/// complete cue is still staged by the actual presentation owner before payment.
pub(crate) fn is_presentation_only(profile: &EffectProfile) -> bool {
    profile.operation == "presentation_only"
        && profile.presentation.is_some()
        && profile.damage_type.is_none()
        && profile.formula.is_none()
        && profile.condition.is_none()
        && profile.mitigated_by.is_none()
        && profile.removed_condition.is_none()
        && profile.duration_ms.is_none()
        && profile.created_item.is_none()
        && profile.pvp_safe_item.is_none()
        && profile.duration_range_ms.is_none()
        && profile.duration_selection.is_none()
        && profile.safe_world_type.is_none()
        && profile.refuse_on.is_none()
        && profile.description_template.is_none()
}

impl PlayerSpellState {
    pub(crate) fn owned_skill_adjustments(
        &self,
        now_us: u64,
    ) -> Result<crate::ability::condition::SkillAdjustments, SpellCastDisposition> {
        self.conditions
            .skill_adjustments(now_us)
            .ok_or(SpellCastDisposition::Rejected)
    }
    pub(crate) fn owned_effective_magic_level(
        &self,
        now_us: u64,
    ) -> Result<u32, SpellCastDisposition> {
        self.owned_skill_adjustments(now_us)?
            .adjust_magic_level(self.character_facts().magic_level)
            .ok_or(SpellCastDisposition::Rejected)
    }
    pub(crate) fn owned_invisible_at(&self, now_us: u64) -> Result<bool, SpellCastDisposition> {
        if !self.conditions.accepts_time(now_us) {
            return rejected();
        }
        Ok(self.conditions.invisible_at(now_us))
    }
}
