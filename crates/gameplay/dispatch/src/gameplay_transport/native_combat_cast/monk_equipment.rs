//! Lower the two Monk equipment attacks through the existing native combat owner.
use super::*;
use crate::spell::native_combat::{AreaPlan, CombatPlan, Element, MagnitudePlan};
use oteryn_gameplay_spell_rules::native_actor_states::{
    ActorSkill, AttackArea, DamageElement, EquipmentAttackPlan, EquipmentFacts, HandItem, HandKind,
};

pub(super) fn applies(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution, Execution::NativeProfile(profile)
        if profile.spell()["execution"]["native_behavior"]["key"] == "equipment_attack"
            && matches!(spell.key.as_str(),
                "candidate:spell/flurry_of_blows" | "candidate:spell/sweeping_takedown"))
}

pub(super) fn facts(
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    caster: &crate::spell::CasterState,
    now: SemanticTimeMicros,
) -> Result<EquipmentFacts, SpellCastDisposition> {
    use crate::content::{ReferenceItemField as F, ReferenceWeaponType as W};
    let mut left = None;
    let mut effective_skills = BTreeMap::new();
    if let Some(item) = owned.equipment().items.iter().find(|item| item.slot == 5) {
        let item = content
            .item_policy(
                &item.definition.production_key,
                &item.definition.revision_ref,
            )
            .ok_or(SpellCastDisposition::Rejected)?;
        if let F::Known(weapon) = &item.record().semantics.weapon {
            let skill = match weapon.weapon_type {
                F::Known(W::Fist) => Some(ActorSkill::Fist),
                F::Known(W::Sword) => Some(ActorSkill::Sword),
                F::Known(W::Axe) => Some(ActorSkill::Axe),
                F::Known(W::Club) => Some(ActorSkill::Club),
                F::Known(W::Distance) => Some(ActorSkill::Distance),
                _ => None,
            };
            if let Some(skill) = skill {
                let bond = match ordinary_combat::monk_weapon_element(content, owned.equipment())? {
                    Element::Earth => Some(DamageElement::Earth),
                    Element::Energy => Some(DamageElement::Energy),
                    _ => None,
                };
                left = Some(HandItem {
                    kind: HandKind::Weapon(skill),
                    attack: caster.attack_value,
                    defense: 0,
                    bond,
                });
                effective_skills.insert(skill, caster.attack_skill);
            }
        }
    }
    let flurry_enlarged = spell.key == "candidate:spell/flurry_of_blows"
        && owned.has_current_wheel(now.get())
        && owned
            .wheel_stage(&spell.name, now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?
            > 0;
    Ok(EquipmentFacts {
        level: caster.level,
        left,
        right: None,
        effective_skills,
        flurry_enlarged,
        shield_slam_grade: 0,
        harmony: Some(caster.harmony_multiplier),
    })
}

pub(super) fn apply_parity(
    spell: &SpellDefinition,
    caster: &crate::spell::CasterState,
    facts: &EquipmentFacts,
    plan: &mut EquipmentAttackPlan,
) -> Result<(), SpellCastDisposition> {
    let area = |rows: &[&str]| AttackArea::Matrix {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        directional: true,
    };
    if spell.key == "candidate:spell/flurry_of_blows" && facts.flurry_enlarged {
        let [flurry] = plan.components.as_mut_slice() else {
            return reject();
        };
        // Crystal 201189f8 flurry_of_blows.lua:47-53.
        flurry.area = area(&["..x..", ".xxx.", "xxxxx", "xxCxx", "xx.xx"]);
    }
    if spell.key == "candidate:spell/sweeping_takedown" {
        let [inner, outer] = plan.components.as_mut_slice() else {
            return reject();
        };
        // Global uses connected 3x2 boxes, with two side tiles and 75% outer damage.
        inner.area = area(&["xxx", "xCx", "x.x"]);
        outer.area = area(&["xxx", "xxx", "...", ".c."]);
        let power = spell.base_power.ok_or(SpellCastDisposition::Rejected)? as f64;
        let level =
            oteryn_gameplay_content::content::spell_data::formula::level_base_damage_healing(
                caster.level,
            );
        let weapon = (1.2 * f64::from(caster.attack_value)).floor();
        let skill = (f64::from(caster.attack_skill) + 4.0) / 28.0;
        let total = (level as f64 + weapon * skill) * (power / 100.0 + 1.0);
        inner.bounds.minimum = caster.harmony_multiplier.apply((total * 0.9) as i64);
        inner.bounds.maximum = caster.harmony_multiplier.apply((total * 1.1) as i64);
        outer.bounds.minimum = inner.bounds.minimum * 3 / 4;
        outer.bounds.maximum = inner.bounds.maximum * 3 / 4;
    }
    Ok(())
}

pub(super) fn geometry(
    plan: &EquipmentAttackPlan,
    origin: TilePosition,
    facing: Direction,
) -> Result<Vec<BTreeSet<TilePosition>>, SpellCastDisposition> {
    plan.components
        .iter()
        .map(|component| {
            let AttackArea::Matrix { rows, directional } = &component.area else {
                return reject();
            };
            let matrix = rows
                .iter()
                .map(|row| {
                    row.bytes()
                        .map(|cell| match cell {
                            b'x' => 1,
                            b'c' => 2,
                            b'C' => 3,
                            _ => 0,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            area_candidates(
                &json!({"area": {"orthogonal": matrix, "directional": directional}}),
                origin,
                facing,
            )
        })
        .collect()
}

pub(super) fn combat(
    equipment: &EquipmentAttackPlan,
    geometry: &[BTreeSet<TilePosition>],
    world: &NativeCombatFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeCombatPlan, SpellCastDisposition> {
    let element = match equipment.element {
        DamageElement::Physical => Element::Physical,
        DamageElement::Earth => Element::Earth,
        DamageElement::Energy => Element::Energy,
        _ => return reject(),
    };
    let mut hits = Vec::new();
    let mut central_tiles = BTreeSet::new();
    let mut central_geometry_tiles = BTreeSet::new();
    for (component, geometry) in equipment.components.iter().zip(geometry) {
        let minimum = component.bounds.minimum;
        let maximum = component.bounds.maximum;
        let magnitude = draw(minimum, maximum);
        if !(minimum..=maximum).contains(&magnitude) {
            return reject();
        }
        let tiles = geometry
            .iter()
            .copied()
            .filter(|position| {
                !world.floor_change_tiles.contains(position)
                    && world
                        .clear_sight
                        .contains(&(world.area_sight_origin, *position))
            })
            .collect::<BTreeSet<_>>();
        hits.extend(
            world
                .targets
                .iter()
                .filter(|target| {
                    target.id != world.caster && target.legal && tiles.contains(&target.position)
                })
                .map(|target| MagnitudePlan {
                    target: target.id,
                    magnitude,
                    bonus_percent: 0,
                    side_percent: None,
                    delay_ms: 0,
                }),
        );
        central_tiles.extend(tiles);
        central_geometry_tiles.extend(geometry);
    }
    Ok(NativeCombatPlan::Combat(CombatPlan {
        element,
        area: Some(AreaPlan {
            origin: world.area_origin,
            central_tiles,
            central_geometry_tiles,
            side_tiles: BTreeSet::new(),
        }),
        hits,
        cooldown_ms: None,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor: false,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: Vec::new(),
        resolve_critical_and_fatal_once: true,
    }))
}
