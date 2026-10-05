//! ATTACK-0 §5 damage, defence and armor formulas as `player_expression` trees for the spell
//! formula engine (`spell/formula.rs`). No second formula engine: the trees are built from the
//! constants table and evaluated by the engine, which truncates each bound toward zero.
//!
//! Engine inputs: `level` (through the official 13.05 curve, equal to `level / 5` up to level
//! 500), `attack_skill`, `attack_value` and `attack_factor` for damage. The engine has no
//! defence inputs of its own, so a defence evaluation passes the defending skill as
//! `shielding_skill` and the defence value as `shield_defense`.

use super::constants::{AttackConstants, FightMode};
use crate::spell::formula::{Binary, Expression, Formula, Input, Unary};

fn constant(value: f64) -> Expression {
    Expression::Const(value)
}

fn var(input: Input) -> Expression {
    Expression::Var(input)
}

fn binary(op: Binary, left: Expression, right: Expression) -> Expression {
    Expression::Binary(op, Box::new(left), Box::new(right))
}

fn floor(inner: Expression) -> Expression {
    Expression::Unary(Unary::Floor, Box::new(inner))
}

/// `round((coefficient * attack_factor * attack * attack_skill) + level / 5)` of
/// `getMaxWeaponDamage` (`weapons.cpp:94-100`), with the level term on the official curve. The
/// level term is an integer, so `floor(x + level_term + 0.5)` is the half-away-from-zero round
/// of a nonnegative `x + level_term`.
fn player_melee_maximum(coefficient: f64, attack: Expression) -> Expression {
    let product = binary(
        Binary::Mul,
        binary(
            Binary::Mul,
            binary(Binary::Mul, constant(coefficient), var(Input::AttackFactor)),
            attack,
        ),
        var(Input::AttackSkill),
    );
    let level = Expression::LevelBaseDamageHealing(Box::new(var(Input::Level)));
    floor(binary(
        Binary::Add,
        binary(Binary::Add, product, level),
        constant(0.5),
    ))
}

/// Player melee weapon damage: `[0, max]`, the weapon's attack in `attack_value`.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn player_melee_formula(constants: &AttackConstants) -> Formula {
    Formula {
        minimum: constant(0.0),
        maximum: player_melee_maximum(
            constants.player_melee.damage_coefficient,
            var(Input::AttackValue),
        ),
    }
}

/// Player fist damage: the melee formula with the fist attack value (`weapons.cpp:226`); it
/// does not read `attack_value`.
pub(crate) fn player_fist_formula(constants: &AttackConstants) -> Formula {
    Formula {
        minimum: constant(0.0),
        maximum: player_melee_maximum(
            constants.player_melee.damage_coefficient,
            constant(f64::from(constants.fist.attack)),
        ),
    }
}

/// Creature melee damage `[0, ceil(skill * (attack * 0.05) + attack * 0.5)]`
/// (`getMaxMeleeDamage`, `weapons.cpp:88-91`) over the creature's `attack_skill` and
/// `attack_value`.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn creature_melee_formula(constants: &AttackConstants) -> Formula {
    let melee = constants.creature_melee;
    let skill_term = binary(
        Binary::Mul,
        var(Input::AttackSkill),
        binary(
            Binary::Mul,
            var(Input::AttackValue),
            constant(melee.skill_attack_coefficient),
        ),
    );
    let attack_term = binary(
        Binary::Mul,
        var(Input::AttackValue),
        constant(melee.attack_coefficient),
    );
    Formula {
        minimum: constant(0.0),
        maximum: Expression::Unary(
            Unary::Ceil,
            Box::new(binary(Binary::Add, skill_term, attack_term)),
        ),
    }
}

/// What the defender defends with; it selects the defence scaling (`player.cpp:776-818`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) enum DefenceSource {
    Shield,
    Weapon,
    Fist,
    Creature,
}

/// The defence mode factor (`getDefenseFactor`, `player.cpp:853-872`): the fight mode's factor
/// while the defender swung within its attack interval, 1.0 otherwise. Creatures use 1.0.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn defence_mode_factor(
    constants: &AttackConstants,
    mode: FightMode,
    swung_within_interval: bool,
) -> f64 {
    if swung_within_interval {
        constants.fight_mode(mode).defence_factor_after_swing
    } else {
        1.0
    }
}

/// Defence draw range `[defence / 2, defence]` (`creature.cpp:966-967`) with
/// `defence = trunc((skill / 4 + 2.23) * value * scaling * mode_factor * vocation_multiplier)`,
/// reading the defending skill from `shielding_skill` and the defence value from
/// `shield_defense`. A defending skill of 0 uses [`zero_skill_defence_bounds`] instead.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn defence_formula(
    constants: &AttackConstants,
    source: DefenceSource,
    mode_factor: f64,
    vocation_multiplier: f64,
) -> Formula {
    let defence = constants.defence;
    let scaling = match source {
        DefenceSource::Shield => defence.scaling_shield,
        DefenceSource::Weapon => defence.scaling_weapon,
        DefenceSource::Fist => defence.scaling_fist,
        DefenceSource::Creature => defence.scaling_creature,
    };
    let skill = binary(
        Binary::Add,
        binary(
            Binary::Div,
            var(Input::ShieldingSkill),
            constant(defence.skill_divisor),
        ),
        constant(defence.skill_offset),
    );
    let maximum = floor(binary(
        Binary::Mul,
        binary(
            Binary::Mul,
            binary(
                Binary::Mul,
                binary(Binary::Mul, skill, var(Input::ShieldDefense)),
                constant(scaling),
            ),
            constant(mode_factor),
        ),
        constant(vocation_multiplier),
    ));
    Formula {
        minimum: floor(binary(Binary::Div, maximum.clone(), constant(2.0))),
        maximum,
    }
}

/// Defence draw range when the defending skill is 0 (`player.cpp:776-818`): the fight mode's
/// fixed defence value `v`, drawn from `[v / 2, v]`.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn zero_skill_defence_bounds(
    constants: &AttackConstants,
    mode: FightMode,
) -> (u32, u32) {
    let value = constants.fight_mode(mode).zero_skill_defence;
    (value / 2, value)
}

/// Armor reduction draw range (`creature.cpp:976-982`): `[armor / 2, armor - (armor % 2 + 1)]`
/// above the flat threshold, exactly 1 from 1 up to it, nothing at 0.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
pub(crate) fn armor_reduction_bounds(constants: &AttackConstants, armor: u32) -> (u32, u32) {
    if armor > constants.armor.flat_reduction_max_armor {
        (armor / 2, armor - (armor % 2 + 1))
    } else if armor > 0 {
        (1, 1)
    } else {
        (0, 0)
    }
}
