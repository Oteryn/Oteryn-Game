//! The equipment-shaped profiles use the same owner payment and combat batches as other spells.
use super::*;
use crate::content::{
    ReferenceItemField as Field, ReferenceModifierElement, ReferenceModifierParameter,
    ReferenceSkillModifierKind, ReferenceWeaponType,
};
use crate::spell::native_combat::{AreaPlan, CombatPlan, Element, MagnitudePlan};
use oteryn_gameplay_spell_rules::native_actor_states::{
    ActorSkill, AttackArea, DamageElement, EquipmentAttackPlan, EquipmentFacts, HandItem, HandKind,
    MagnitudeBounds, equipment_attack,
};

pub(super) fn is_targeted(spell: &SpellDefinition) -> bool {
    spell.name == "Shield Bash"
}

pub(super) fn inputs(
    parameters: &Value,
    owned: &OwnedCastFacts,
    content: &NativeGameplayState,
    spell: &SpellDefinition,
    caster: &crate::spell::CasterState,
    now: SemanticTimeMicros,
) -> Result<EquipmentFacts, SpellCastDisposition> {
    let shield = parameters["input"]["kind"] == "wielded_shield";
    let hand = if shield {
        if caster.shield_defense.is_none() {
            return Err(SpellCastDisposition::NotAvailable);
        }
        caster.shield_defense.map(|defense| HandItem {
            kind: HandKind::Shield,
            attack: 0,
            defense,
            bond: None,
        })
    } else {
        let mut hand = None;
        for slot in [5, 6] {
            let Some(item) = owned
                .equipment()
                .items
                .iter()
                .find(|item| item.slot == slot)
            else {
                continue;
            };
            let policy = content
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .ok_or(SpellCastDisposition::Rejected)?;
            let semantics = &policy.record().semantics;
            let Field::Known(weapon) = &semantics.weapon else {
                continue;
            };
            let skill = match weapon.weapon_type {
                Field::Known(ReferenceWeaponType::Fist) => ActorSkill::Fist,
                Field::Known(ReferenceWeaponType::Club) => ActorSkill::Club,
                Field::Known(ReferenceWeaponType::Sword) => ActorSkill::Sword,
                Field::Known(ReferenceWeaponType::Axe) => ActorSkill::Axe,
                Field::Known(ReferenceWeaponType::Distance) => ActorSkill::Distance,
                _ => continue,
            };
            let bond = match &semantics.skill_modifiers {
                Field::Known(modifiers) => match &modifiers.modifiers {
                    Field::Known(modifiers) => modifiers.iter().find_map(|modifier| {
                        if modifier.kind != ReferenceSkillModifierKind::ElementalBond {
                            return None;
                        }
                        match modifier.parameter {
                            Field::Known(ReferenceModifierParameter::Element(
                                ReferenceModifierElement::Energy,
                            )) => Some(DamageElement::Energy),
                            Field::Known(ReferenceModifierParameter::Element(
                                ReferenceModifierElement::Earth,
                            )) => Some(DamageElement::Earth),
                            _ => None,
                        }
                    }),
                    _ => None,
                },
                _ => None,
            };
            // The caster projection already read the selected weapon's effective skill.
            hand = Some(HandItem {
                kind: HandKind::Weapon(skill),
                attack: caster.attack_value,
                defense: 0,
                bond,
            });
            break;
        }
        hand
    };
    let grade = if owned.has_current_wheel(now.get()) {
        owned
            .wheel_stage(&spell.name, now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?
    } else {
        0
    };
    let mut effective_skills = BTreeMap::from([(ActorSkill::Shielding, caster.shielding_skill)]);
    if let Some(HandItem {
        kind: HandKind::Weapon(skill),
        ..
    }) = hand
    {
        effective_skills.insert(skill, caster.attack_skill);
    }
    Ok(EquipmentFacts {
        level: caster.level,
        left: hand,
        right: None,
        effective_skills,
        flurry_enlarged: grade >= 1,
        shield_slam_grade: grade,
        harmony: Some(caster.harmony_multiplier),
    })
}

pub(super) struct EquipmentCast {
    pub(super) inputs: EquipmentFacts,
    pub(super) plan: EquipmentAttackPlan,
    damage_percent: i64,
}

pub(super) fn planned(
    parameters: &Value,
    equipment: &EquipmentFacts,
    owned: &OwnedCastFacts,
    player: &PlayerSpellState,
    now: SemanticTimeMicros,
) -> Result<EquipmentCast, SpellCastDisposition> {
    let mut plan =
        equipment_attack(parameters, equipment).map_err(|_| SpellCastDisposition::Rejected)?;
    let power = parameters["base_power"]
        .as_u64()
        .ok_or(SpellCastDisposition::Rejected)? as f64;
    let base = crate::spell::formula::level_base_damage_healing(equipment.level) as f64;
    let hand = equipment.left;
    let mut damage_percent = 100;
    match parameters["spell"].as_str() {
        Some(name @ ("shield_bash" | "shield_slam")) => {
            let defense = f64::from(hand.ok_or(SpellCastDisposition::NotAvailable)?.defense);
            let skill = f64::from(equipment.effective_skills[&ActorSkill::Shielding]);
            let (min_def, max_def, min_skill, max_skill) = if name == "shield_bash" {
                (1.5, 2.5, 0.6, 1.0)
            } else {
                (1.4, 2.3, 0.55, 0.95)
            };
            // Crystal shield_bash.lua and shield_slam.lua formula callbacks.
            plan.components[0].bounds = damage_bounds(
                base + power + defense * min_def + skill * min_skill,
                base + power + defense * max_def + skill * max_skill,
            )?;
        }
        Some(name @ ("flurry_of_blows" | "sweeping_takedown")) => {
            let (skill, attack) = hand.map_or((0, 7), |item| {
                let skill = match item.kind {
                    HandKind::Weapon(skill) => equipment.effective_skills[&skill],
                    _ => 0,
                };
                (skill, item.attack)
            });
            let sweeping = name == "sweeping_takedown";
            // Crystal register_spells.lua: calculateMonkSpellDamage and calculateAttackValue.
            let weapon = (1.2 * f64::from(attack)).floor() * ((f64::from(skill) + 4.0) / 28.0);
            let attack_value = base + weapon;
            let factor = if sweeping { 1.0 } else { 0.6 };
            let total = power * attack_value / 100.0 + factor * attack_value;
            let bounds = damage_bounds(total * 0.9, total * 1.1)?;
            plan.blocked_by_armor = false;
            if sweeping {
                // Crystal sweeping_takedown.lua runs two distinct Combat calls with the same formula.
                plan.components[0].area = directional(&["xxx", "xCx", "x.x"]);
                plan.components[1].area =
                    directional(&[".xxx.", "xxxxx", "x...x", "x.c.x", "x...x"]);
                for component in &mut plan.components {
                    component.bounds = bounds;
                }
                let monk = player.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
                if let Some(exponent) = monk.harmony().checked_sub(1) {
                    use oteryn_gameplay_spell_rules::native_actor_states::StandardStance;
                    let virtue = if player.standard_stance() == Some(StandardStance::Harmony) {
                        if monk.serene() { 6 } else { 3 }
                    } else {
                        0
                    };
                    let ascetic = if owned.has_current_wheel(now.get()) {
                        owned
                            .wheel_stage("ascetic", now.get())
                            .map_err(|_| SpellCastDisposition::Rejected)?
                    } else {
                        0
                    };
                    // Crystal combat.cpp: applyImproveMonkAttackSpender scales each rolled hit.
                    damage_percent += (7 + virtue + i64::from(ascetic)) << exponent;
                }
            } else {
                plan.components[0].bounds = bounds;
                if equipment.flurry_enlarged {
                    plan.components[0].area =
                        directional(&["..x..", ".xxx.", "xxxxx", "xxCxx", "xx.xx"]);
                }
            }
        }
        _ => return reject(),
    }
    Ok(EquipmentCast {
        inputs: equipment.clone(),
        plan,
        damage_percent,
    })
}

fn damage_bounds(minimum: f64, maximum: f64) -> Result<MagnitudeBounds, SpellCastDisposition> {
    if minimum < 0.0 || maximum < minimum || !maximum.is_finite() || maximum > f64::from(i32::MAX) {
        return reject();
    }
    Ok(MagnitudeBounds {
        minimum: minimum as i64,
        maximum: maximum as i64,
    })
}

fn directional(rows: &[&str]) -> AttackArea {
    AttackArea::Matrix {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        directional: true,
    }
}

pub(super) fn footprint(
    area: &AttackArea,
    origin: TilePosition,
    facing: Direction,
) -> Result<BTreeSet<TilePosition>, SpellCastDisposition> {
    match area {
        AttackArea::Target => Ok(BTreeSet::from([origin])),
        AttackArea::Matrix { rows, directional } => {
            let matrix =
                cast_plans::source_matrix(rows).map_err(|_| SpellCastDisposition::Rejected)?;
            area_candidates(
                &json!({"area":{"directional":directional,"orthogonal":matrix}}),
                origin,
                facing,
            )
        }
    }
}

fn damage_element(element: DamageElement) -> Element {
    match element {
        DamageElement::Physical => Element::Physical,
        DamageElement::Energy => Element::Energy,
        DamageElement::Earth => Element::Earth,
        DamageElement::Fire => Element::Fire,
        DamageElement::Ice => Element::Ice,
        DamageElement::Holy => Element::Holy,
        DamageElement::Death => Element::Death,
    }
}

pub(super) fn combat(
    equipment: &EquipmentCast,
    facts: &NativeCombatFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeCombatPlan, SpellCastDisposition> {
    let mut hits = Vec::new();
    let mut tiles = BTreeSet::new();
    let mut geometry = BTreeSet::new();
    let origin = facts.area_origin;
    for component in &equipment.plan.components {
        let footprint = footprint(&component.area, origin, facts.direction)?;
        geometry.extend(footprint.iter().copied());
        let valid = |position| {
            footprint.contains(&position)
                && !facts.floor_change_tiles.contains(&position)
                && (facts.area_sight_origin == position
                    || facts
                        .clear_sight
                        .contains(&(facts.area_sight_origin, position)))
        };
        tiles.extend(footprint.iter().copied().filter(|p| valid(*p)));
        let amount = draw(component.bounds.minimum, component.bounds.maximum);
        if !(component.bounds.minimum..=component.bounds.maximum).contains(&amount) {
            return reject();
        }
        for target in &facts.targets {
            if target.id != facts.caster
                && target.legal
                && valid(target.position)
                && (component.area != AttackArea::Target
                    || facts.explicit_target == Some(target.id))
            {
                hits.push(MagnitudePlan {
                    target: target.id,
                    magnitude: amount * equipment.damage_percent / 100,
                    bonus_percent: 0,
                    side_percent: None,
                    delay_ms: 0,
                });
            }
        }
    }
    let area = facts.explicit_target.is_none().then_some(AreaPlan {
        origin,
        central_tiles: tiles,
        central_geometry_tiles: geometry,
        side_tiles: BTreeSet::new(),
    });
    Ok(NativeCombatPlan::Combat(CombatPlan {
        element: damage_element(equipment.plan.element),
        area,
        hits,
        cooldown_ms: None,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor: equipment.plan.blocked_by_armor,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: Vec::new(),
        resolve_critical_and_fatal_once: true,
    }))
}

pub(super) fn append_shield_conditions(
    runtime: &ChannelRuntimeV1,
    lowered: &mut crate::spell::combat_batch::LoweredNativeCombat,
    equipment: &EquipmentAttackPlan,
    plan: &NativeCombatPlan,
    bindings: &[LiveActorBinding],
    now: SemanticTimeMicros,
) -> Result<(), SpellCastDisposition> {
    use crate::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind,
    };
    use crate::foundation::runtime_actor_spell_types::{
        CompanionConditionUpdate, MAX_EFFECTS, OwnerCombatChange, OwnerCombatEffect,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

    let Some(debuff) = equipment
        .components
        .first()
        .and_then(|component| component.next_auto_attack)
    else {
        return Ok(());
    };
    let NativeCombatPlan::Combat(plan) = plan else {
        return reject();
    };
    let duration_ms =
        u32::try_from(debuff.duration_ms).map_err(|_| SpellCastDisposition::Rejected)?;
    let percent = u32::try_from(100 - debuff.reduction_percent - debuff.wheel_bonus_percent)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let definition = ConditionDefinition::shield_attack_reduction(duration_ms, percent)
        .ok_or(SpellCastDisposition::Rejected)?;
    let batch = &mut lowered.batch;
    let seed = nonce(
        b"oteryn:shield-attack-condition:v1",
        batch.caster,
        batch.command.game_session_id(),
        batch.command.command_id().get(),
        &[],
    );
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let facts = ApplicationFacts {
        now: now.get(),
        base_speed: 0,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: false,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes(seed),
    };
    for hit in &plan.hits {
        let binding = bindings
            .iter()
            .find(|binding| binding.source_id == hit.target)
            .ok_or(SpellCastDisposition::Rejected)?;
        let expected = runtime
            .companion_snapshot(binding.actor)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let mut next = expected.state.clone();
        match next.conditions.apply(
            &definition,
            Some(batch.caster),
            ConditionSourceKind::Player,
            &next.policy.condition_immunities,
            &facts,
        ) {
            Ok(_) => (),
            Err(crate::domain::condition::ConditionRefusal::Immune) => continue,
            Err(_) => return reject(),
        }
        let ordinal = lowered.next_sub_ordinal;
        if usize::from(ordinal) >= MAX_EFFECTS {
            return reject();
        }
        batch.effects.push(OwnerCombatEffect {
            target: binding.actor,
            sub_ordinal: ordinal,
            change: OwnerCombatChange::CompanionConditions(Box::new(CompanionConditionUpdate {
                expected,
                next,
            })),
        });
        lowered.next_sub_ordinal += 1;
    }
    Ok(())
}
