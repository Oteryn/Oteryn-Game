//! SHIELD-DEFENCE-1: what a player's hands defend a creature hit with (`player.cpp`
//! `getShieldAndWeapon`, `getDefense`, `getWeaponSkill`; `creature.cpp` `blockHit`).

use crate::attack::formulas::{
    DefenceSource, defence_formula, defence_mode_factor, zero_skill_defence_bounds,
};
use crate::attack::{AttackConstants, FightMode};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::spell_data::formula::FormulaInputs;
use oteryn_gameplay_content::content::{ReferenceItemField, ReferenceWeaponType};

/// The defence of a player's hands: the defence value, the formula's scaling source and the
/// owned skill index it scales with (`None` for a weapon with no skill, which defends as a
/// defending skill of 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandDefence {
    pub source: DefenceSource,
    pub value: u32,
    pub skill_index: Option<usize>,
}

/// The owned skill index of the shielding skill.
const SHIELDING: usize = 5;

struct HandItem {
    kind: ReferenceWeaponType,
    defense: i32,
    extra_defense: i32,
}

/// The defence of the hands `hand` resolves (`getShieldAndWeapon` over right (5) then left (6)):
/// the shield is the hand item of type shield (a spellbook too) with the higher defence, the
/// weapon the last other weapon. With a shield the value is the shield's defence plus the
/// weapon's extra defence, scaled by shielding; with a weapon only, its defence plus extra
/// defence, scaled by its skill; empty hands defend with the fist value and the fist skill.
/// `None` (fail closed) when a hand item's policy is unknown or its weapon facts are not Known.
pub fn hand_defence<'a>(
    content: &NativeGameplayState,
    constants: &AttackConstants,
    hand: impl Fn(u8) -> Option<(&'a str, &'a str)>,
) -> Option<HandDefence> {
    let mut shield: Option<HandItem> = None;
    let mut weapon: Option<HandItem> = None;
    for slot in [5_u8, 6] {
        let Some((production_key, revision_ref)) = hand(slot) else {
            continue;
        };
        let policy = content.item_policy(production_key, revision_ref)?;
        let facts = match &policy.record().semantics.weapon {
            ReferenceItemField::NotApplicable => continue,
            ReferenceItemField::Known(weapon) => weapon,
            ReferenceItemField::Unknown | ReferenceItemField::Conflict => return None,
        };
        let (
            ReferenceItemField::Known(kind),
            ReferenceItemField::Known(defense),
            ReferenceItemField::Known(extra_defense),
        ) = (&facts.weapon_type, &facts.defense, &facts.extra_defense)
        else {
            return None;
        };
        let item = HandItem {
            kind: *kind,
            defense: defense.0,
            extra_defense: extra_defense.0,
        };
        if matches!(
            *kind,
            ReferenceWeaponType::Shield | ReferenceWeaponType::Spellbook
        ) {
            if shield
                .as_ref()
                .is_none_or(|held| item.defense > held.defense)
            {
                shield = Some(item);
            }
        } else {
            weapon = Some(item);
        }
    }
    let extra = weapon.as_ref().map_or(0, |weapon| weapon.extra_defense);
    let (source, value, skill_index) = match (&shield, &weapon) {
        (Some(shield), _) => (
            DefenceSource::Shield,
            shield.defense.saturating_add(extra),
            Some(SHIELDING),
        ),
        (None, Some(weapon)) => (
            DefenceSource::Weapon,
            weapon.defense.saturating_add(weapon.extra_defense),
            weapon_skill(weapon.kind),
        ),
        (None, None) => (
            DefenceSource::Fist,
            i32::try_from(constants.fist.defence).ok()?,
            Some(0),
        ),
    };
    Some(HandDefence {
        source,
        value: u32::try_from(value).unwrap_or(0),
        skill_index,
    })
}

/// `getWeaponSkill`: the owned skill index a weapon defends with; none for wands, rods and
/// ammunition.
const fn weapon_skill(kind: ReferenceWeaponType) -> Option<usize> {
    match kind {
        ReferenceWeaponType::Fist => Some(0),
        ReferenceWeaponType::Club => Some(1),
        ReferenceWeaponType::Sword => Some(2),
        ReferenceWeaponType::Axe => Some(3),
        ReferenceWeaponType::Distance => Some(4),
        ReferenceWeaponType::Ammunition
        | ReferenceWeaponType::Wand
        | ReferenceWeaponType::Shield
        | ReferenceWeaponType::Spellbook => None,
    }
}

/// The defence draw range of a blocked creature hit (`creature.cpp` `blockHit`:
/// `[defense / 2, defense]`) for `hand` defended with the live `skill` in fight `mode`; a
/// defending skill of 0 uses the fight mode's fixed value. `None` when the formula does not
/// evaluate.
pub fn player_defence_bounds(
    constants: &AttackConstants,
    hand: HandDefence,
    skill: u32,
    mode: FightMode,
    swung_within_interval: bool,
) -> Option<(u32, u32)> {
    if hand.skill_index.is_none() || skill == 0 {
        return Some(zero_skill_defence_bounds(constants, mode));
    }
    let formula = defence_formula(
        constants,
        hand.source,
        defence_mode_factor(constants, mode, swung_within_interval),
        1.0,
    );
    let (minimum, maximum) = formula
        .bounds(&FormulaInputs {
            level: 1,
            magic_level: 0,
            base_power: None,
            attack_skill: 0,
            attack_value: 0,
            attack_factor: 1.0,
            shielding_skill: skill,
            shield_defense: Some(hand.value),
        })
        .ok()?;
    Some((
        u32::try_from(minimum.max(0)).ok()?,
        u32::try_from(maximum.max(0)).ok()?,
    ))
}

/// What a blocked hit's defence leaves of `damage`: `draw` picks the reduction in `bounds`.
#[must_use]
pub fn player_defence_mitigated(damage: u32, bounds: (u32, u32), draw: u64) -> u32 {
    let (minimum, maximum) = (bounds.0.min(bounds.1), bounds.1);
    let span = u64::from(maximum - minimum).saturating_add(1);
    let reduction = u32::try_from(draw % span).map_or(maximum, |offset| minimum + offset);
    damage.saturating_sub(reduction)
}

#[cfg(test)]
#[path = "player_defence_tests.rs"]
mod tests;
