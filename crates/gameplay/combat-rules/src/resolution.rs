//! Pure weapon selection, deterministic swing draws and creature mitigation.

use crate::attack::{AttackConstants, armor_reduction_bounds};
use oteryn_gameplay_content::content::creature_policy::{
    CompiledCreaturePolicy, CreatureExactRatio,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::{ReferenceItemField, ReferenceWeaponType};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub mod player_defence;

/// How often the attacker's own connection re-reads its swing weapon while it holds a target.
pub const WEAPON_REFRESH_MICROS: u64 = 1_000_000;

/// The weapon a swing is drawn with (ATTACK-0 §4): fists, or a melee weapon's attack value. A
/// distance weapon, a throwing weapon, a wand or a rod is no weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwingWeapon {
    Fists,
    Melee { attack: u32, skill: MeleeSkill },
}

/// The skill a melee weapon trains and swings with (`player.cpp` `getWeaponSkill`): a club,
/// sword, axe or fist weapon uses its own skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeSkill {
    Fist,
    Club,
    Sword,
    Axe,
}

impl MeleeSkill {
    /// The index of this skill in the seven owned skills (fist, club, sword, axe, distance,
    /// shielding, fishing).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Fist => 0,
            Self::Club => 1,
            Self::Sword => 2,
            Self::Axe => 3,
        }
    }
}

/// The swing weapon of `items` (`player.cpp:405-464` `getWeapon`): the first hand, left (6) then
/// right (5), that holds a weapon other than a shield, ammunition or a spellbook. A club, sword,
/// axe or fist weapon swings with its attack; any other weapon, or none, swings with fists.
/// `None` (fail closed) when a hand item's policy is unknown or its weapon facts are not Known.
pub fn swing_weapon<'a>(
    content: &NativeGameplayState,
    hand: impl Fn(u8) -> Option<(&'a str, &'a str)>,
) -> Option<SwingWeapon> {
    for slot in [6_u8, 5] {
        let Some((production_key, revision_ref)) = hand(slot) else {
            continue;
        };
        let policy = content.item_policy(production_key, revision_ref)?;
        let weapon = match &policy.record().semantics.weapon {
            ReferenceItemField::NotApplicable => continue,
            ReferenceItemField::Known(weapon) => weapon,
            ReferenceItemField::Unknown | ReferenceItemField::Conflict => return None,
        };
        let ReferenceItemField::Known(kind) = weapon.weapon_type else {
            return None;
        };
        match kind {
            ReferenceWeaponType::Shield
            | ReferenceWeaponType::Ammunition
            | ReferenceWeaponType::Spellbook => continue,
            ReferenceWeaponType::Distance | ReferenceWeaponType::Wand => {
                return Some(SwingWeapon::Fists);
            }
            ReferenceWeaponType::Club
            | ReferenceWeaponType::Sword
            | ReferenceWeaponType::Axe
            | ReferenceWeaponType::Fist => {
                let ReferenceItemField::Known(attack) = weapon.attack else {
                    return None;
                };
                let skill = match kind {
                    ReferenceWeaponType::Club => MeleeSkill::Club,
                    ReferenceWeaponType::Sword => MeleeSkill::Sword,
                    ReferenceWeaponType::Axe => MeleeSkill::Axe,
                    _ => MeleeSkill::Fist,
                };
                return Some(SwingWeapon::Melee {
                    attack: u32::try_from(attack.0).ok()?,
                    skill,
                });
            }
        }
    }
    Some(SwingWeapon::Fists)
}

/// The decision occurrence of one swing event: unique per attacker, lineage, ordinal and tag.
pub fn swing_occurrence(
    attacker: [u8; 16],
    lineage: ([u8; 16], u64),
    ordinal: u16,
    tag: &[u8],
) -> DecisionOccurrenceId {
    let hash = Sha256::new()
        .chain_update(b"oteryn:auto-attack-swing:v1")
        .chain_update(attacker)
        .chain_update(lineage.0)
        .chain_update(lineage.1.to_be_bytes())
        .chain_update(ordinal.to_be_bytes())
        .chain_update(tag)
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

/// One draw in `[minimum, maximum]` of the swing's occurrence `tag` under `key`; a degenerate
/// range is its bound without a draw. `None` on an RNG fault.
#[allow(clippy::too_many_arguments)]
pub fn swing_draw(
    root: &GameplayDecisionRoot,
    attacker: [u8; 16],
    lineage: ([u8; 16], u64),
    ordinal: u16,
    tag: &[u8],
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Option<i64> {
    if maximum <= minimum {
        return Some(minimum);
    }
    let draw = deterministic_decision_u64(
        root,
        swing_occurrence(attacker, lineage, ordinal, tag),
        key,
        0,
    )
    .ok()?;
    let span = u64::try_from(maximum - minimum).ok()?.saturating_add(1);
    Some(minimum.saturating_add(i64::try_from(draw % span).ok()?))
}

/// What the creature's block (when its budget allows), defence and armor leave of `damage`
/// (`creature.cpp:958-990`): defence draws in `[defense / 2, defense]` and, when it stops the
/// whole hit, armor is not reached; armor draws in [`armor_reduction_bounds`].
#[allow(clippy::too_many_arguments)]
pub fn creature_mitigated(
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
    attacker: [u8; 16],
    lineage: ([u8; 16], u64),
    ordinal: u16,
    damage: i64,
    blocked: bool,
    defense: u32,
    armor: u32,
) -> Option<i64> {
    swing_mitigated(
        root,
        constants,
        attacker,
        lineage,
        ordinal,
        damage,
        blocked.then_some((defense / 2, defense)),
        armor,
    )
}

/// PVP-DEFENCE-1: what a blocked hit's `defence` draw range (`None` when the target did not
/// block) and the target's armor leave of a swing (`Creature::blockHit`): when defence stops the
/// whole hit, armor is not reached; armor draws in [`armor_reduction_bounds`]. A creature target
/// passes its `[defense / 2, defense]`, a player target the bounds of the hand it defends with.
#[allow(clippy::too_many_arguments)]
pub fn swing_mitigated(
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
    attacker: [u8; 16],
    lineage: ([u8; 16], u64),
    ordinal: u16,
    damage: i64,
    defence: Option<(u32, u32)>,
    armor: u32,
) -> Option<i64> {
    let mut remaining = damage;
    if let Some((minimum, maximum)) = defence {
        let defence = swing_draw(
            root,
            attacker,
            lineage,
            ordinal,
            b"defence-draw",
            "attack.defence",
            i64::from(minimum),
            i64::from(maximum),
        )?;
        remaining = remaining.saturating_sub(defence);
        if remaining <= 0 {
            return Some(0);
        }
    }
    let (minimum, maximum) = armor_reduction_bounds(constants, armor);
    let reduction = swing_draw(
        root,
        attacker,
        lineage,
        ordinal,
        b"armor-draw",
        "attack.armor",
        i64::from(minimum),
        i64::from(maximum),
    )?;
    Some(remaining.saturating_sub(reduction).max(0))
}

/// The damage key a melee swing deals (`COMBAT_PHYSICALDAMAGE`).
const PHYSICAL: &str = "physical";

/// CREATURE-RESIST-MELEE-1 (`Creature::blockHit` `isImmune`): a creature immune to physical
/// damage takes nothing from a swing, before its block budget or defence is reached.
#[must_use]
pub fn creature_physically_immune(policy: &CompiledCreaturePolicy) -> bool {
    policy.damage_immunities.iter().any(|v| v == PHYSICAL)
}

/// CREATURE-RESIST-MELEE-1: what a creature's mitigation and physical response leave of a swing
/// that passed its defence and armor (`Creature::mitigateDamage`, then `Monster::blockHit`'s
/// elementMap), by the arithmetic the creature spell path uses: mitigation (capped at 30%)
/// truncates, the response rounds and never goes below 0. A creature without mitigation
/// mitigates nothing (a monster type's mitigation defaults to 0); `None` when a ratio is
/// malformed (fail closed).
#[must_use]
pub fn creature_physical_response(policy: &CompiledCreaturePolicy, damage: i64) -> Option<i64> {
    let ratio = |r: CreatureExactRatio| {
        (r.denominator != 0).then(|| r.numerator as f64 / r.denominator as f64)
    };
    let mitigation = match policy.mitigation {
        Some(m) => ratio(m)?.min(30.0),
        None => 0.0,
    };
    let resistance = match policy
        .resistances
        .iter()
        .find(|v| v.damage_type == PHYSICAL)
    {
        Some(v) => ratio(v.percent)?,
        None => 0.0,
    };
    let damage = damage as f64;
    let mitigated = (damage - damage * mitigation / 100.0).trunc();
    let landed = (mitigated * (100.0 - resistance) / 100.0).round();
    if !landed.is_finite() || landed > f64::from(i32::MAX) || landed < f64::from(i32::MIN) {
        return None;
    }
    Some((landed as i64).max(0))
}

/// ARMOR-MITIGATION-1: what a player's worn armor leaves of a creature hit (`creature.cpp`
/// `Creature::blockHit`, `checkArmor`): `draw` picks the reduction in
/// [`armor_reduction_bounds`]. A hit is never raised and never goes below 0.
#[must_use]
pub fn player_armor_mitigated(
    constants: &AttackConstants,
    damage: u32,
    armor: u32,
    draw: u64,
) -> u32 {
    let (minimum, maximum) = armor_reduction_bounds(constants, armor);
    let span = u64::from(maximum.saturating_sub(minimum)).saturating_add(1);
    let reduction = u32::try_from(draw % span).map_or(maximum, |offset| minimum + offset);
    damage.saturating_sub(reduction)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn mitigated(ordinal: u16, damage: i64, blocked: bool, defense: u32, armor: u32) -> i64 {
        let constants = AttackConstants::checked_in().expect("constants");
        creature_mitigated(
            &GameplayDecisionRoot::from_bytes([7; 32]),
            constants,
            [1; 16],
            ([3; 16], 1),
            ordinal,
            damage,
            blocked,
            defense,
            armor,
        )
        .expect("draws")
    }

    #[test]
    fn a_player_target_defends_by_its_hand_bounds_and_a_creature_by_half_to_full() {
        let constants = AttackConstants::checked_in().expect("constants");
        let swing = |ordinal: u16, damage: i64, defence: Option<(u32, u32)>, armor: u32| {
            swing_mitigated(
                &GameplayDecisionRoot::from_bytes([7; 32]),
                constants,
                [1; 16],
                ([3; 16], 1),
                ordinal,
                damage,
                defence,
                armor,
            )
            .expect("draws")
        };
        for ordinal in 0..64 {
            assert_eq!(
                swing(ordinal, 100, Some((5, 10)), 10),
                mitigated(ordinal, 100, true, 10, 10),
                "a creature's block is the [defense / 2, defense] range"
            );
            assert_eq!(
                swing(ordinal, 100, None, 10),
                mitigated(ordinal, 100, false, 10, 10),
                "no block, armor only"
            );
            let defended = swing(ordinal, 100, Some((3, 7)), 0);
            assert!((93..=97).contains(&defended), "defence draws in [3, 7]");
            assert_eq!(swing(ordinal, 6, Some((7, 7)), 10), 0, "defence stops it");
            assert_eq!(
                swing(ordinal, 0, Some((0, 0)), 0),
                0,
                "nothing stays nothing"
            );
        }
    }

    #[test]
    fn defence_applies_only_to_a_blocked_hit_and_armor_draws_in_its_bounds() {
        for ordinal in 0..64 {
            assert_eq!(
                mitigated(ordinal, 10, false, 1_000, 0),
                10,
                "no block, no armor"
            );
            assert_eq!(
                mitigated(ordinal, 10, true, 1_000, 0),
                0,
                "defence stops the hit"
            );
            assert_eq!(
                mitigated(ordinal, 10, false, 0, 2),
                9,
                "low armor reduces by 1"
            );
            let defended = mitigated(ordinal, 100, true, 10, 0);
            assert!((90..=95).contains(&defended), "defence draws in [5, 10]");
            let armored = mitigated(ordinal, 100, false, 0, 10);
            assert!((91..=95).contains(&armored), "armor 10 draws in [5, 9]");
            assert_eq!(mitigated(ordinal, 3, false, 0, 10), 0, "armor never heals");
        }
        assert_eq!(
            mitigated(3, 100, true, 10, 10),
            mitigated(3, 100, true, 10, 10),
            "every draw is a pure function of the swing"
        );
    }

    #[test]
    fn player_armor_reduces_a_creature_hit_within_its_bounds() {
        let constants = AttackConstants::checked_in().expect("constants");
        for draw in 0..64 {
            assert_eq!(
                player_armor_mitigated(constants, 10, 0, draw),
                10,
                "no armor"
            );
            assert_eq!(
                player_armor_mitigated(constants, 10, 3, draw),
                9,
                "armor 3 takes 1"
            );
            let armored = player_armor_mitigated(constants, 100, 10, draw);
            assert!((91..=95).contains(&armored), "armor 10 takes [5, 9]");
            let odd = player_armor_mitigated(constants, 100, 11, draw);
            assert!((90..=95).contains(&odd), "armor 11 takes [5, 9]");
            assert_eq!(
                player_armor_mitigated(constants, 4, 10, draw),
                0,
                "never below 0"
            );
        }
        assert_eq!(
            player_armor_mitigated(constants, 100, 10, 0),
            95,
            "draw 0 takes the minimum"
        );
        assert_eq!(
            player_armor_mitigated(constants, 100, 10, 4),
            91,
            "draw 4 takes the maximum"
        );
        assert_eq!(player_armor_mitigated(constants, 100, 10, u64::MAX), 95);
    }

    fn policy(
        mitigation: Option<(i64, u64)>,
        physical: Option<(i64, u64)>,
    ) -> CompiledCreaturePolicy {
        use oteryn_gameplay_content::content::creature_policy::{
            CreatureFlags, CreatureResistance,
        };
        let ratio = |(numerator, denominator)| CreatureExactRatio {
            numerator,
            denominator,
        };
        CompiledCreaturePolicy {
            definition_key: "creature".into(),
            definition_revision: "1".into(),
            display_name: "creature".into(),
            maximum_health: 100,
            base_speed: 100,
            outfit_look_type: 1,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(0),
            armor: Some(0),
            mitigation: mitigation.map(ratio),
            resistances: physical
                .map(|p| CreatureResistance {
                    damage_type: PHYSICAL.into(),
                    percent: ratio(p),
                })
                .into_iter()
                .collect(),
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: None,
        }
    }

    #[test]
    fn a_creatures_mitigation_and_physical_response_shape_a_landed_swing() {
        let response = |m, p, damage| creature_physical_response(&policy(m, p), damage);
        assert_eq!(
            response(Some((0, 1)), None, 15),
            Some(15),
            "nothing to mitigate"
        );
        assert_eq!(
            response(Some((10, 1)), None, 15),
            Some(13),
            "10% truncates 13.5"
        );
        assert_eq!(
            response(Some((50, 1)), None, 10),
            Some(7),
            "mitigation caps at 30%"
        );
        assert_eq!(
            response(Some((0, 1)), Some((100, 1)), 40),
            Some(0),
            "100% physical stops it"
        );
        assert_eq!(
            response(Some((0, 1)), Some((25, 1)), 10),
            Some(8),
            "25% rounds 7.5 up"
        );
        assert_eq!(
            response(Some((10, 1)), Some((-10, 1)), 15),
            Some(14),
            "weakness raises 13"
        );
        assert_eq!(
            response(Some((3, 2)), None, 200),
            Some(197),
            "an exact ratio of 1.5%"
        );
        assert_eq!(
            response(None, None, 15),
            Some(15),
            "no mitigation mitigates nothing"
        );
        assert_eq!(
            response(Some((1, 0)), None, 15),
            None,
            "a 0 denominator fails closed"
        );
        assert_eq!(response(Some((0, 1)), Some((1, 0)), 15), None);
    }

    #[test]
    fn only_a_physical_immunity_stops_a_swing() {
        let mut immune = policy(Some((0, 1)), None);
        assert!(!creature_physically_immune(&immune));
        immune.damage_immunities = vec!["fire".into()];
        assert!(!creature_physically_immune(&immune));
        immune.damage_immunities.push(PHYSICAL.into());
        assert!(creature_physically_immune(&immune));
    }
}
