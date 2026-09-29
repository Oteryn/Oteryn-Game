//! Reads a candidate Spell authoring bundle (`tools/content-schema/spell-authoring`, schema v1) into a
//! [`SpellDefinition`]. The bundle's own validator (`validate_spell.py`) owns structural validation;
//! this reader rejects anything it cannot represent instead of guessing. The admitted content route
//! (WorldProject/v2 lowering) replaces this reader in P3b.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde_json::Value;

use super::chain::{ChainShape, ChainSpec};
use super::formula::{Binary, Expression, Extremum, Formula, Input, MAX_EXPRESSION_DEPTH, Unary};
use super::party::{PartyBuffSpec, PartyMana};
use super::{Carrier, CooldownGroup, Execution, ManaCost, SpellDefinition, SpellEffect, Vocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoringError(pub(crate) String);

impl Display for AuthoringError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "spell authoring bundle: {}", self.0)
    }
}

impl Error for AuthoringError {}

fn fail<T>(message: impl Into<String>) -> Result<T, AuthoringError> {
    Err(AuthoringError(message.into()))
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, AuthoringError> {
    value
        .get(name)
        .ok_or_else(|| AuthoringError(format!("missing field {name}")))
}

fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, AuthoringError> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| AuthoringError(format!("{name} is not a string")))
}

fn number(value: &Value, name: &str) -> Result<u32, AuthoringError> {
    field(value, name)?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| AuthoringError(format!("{name} is not a u32")))
}

fn flag(value: &Value, name: &str) -> Result<bool, AuthoringError> {
    field(value, name)?
        .as_bool()
        .ok_or_else(|| AuthoringError(format!("{name} is not a boolean")))
}

fn reference_key<'a>(value: &'a Value, family: &str) -> Result<&'a str, AuthoringError> {
    if text(value, "family")? != family {
        return fail(format!("expected a {family} reference"));
    }
    text(value, "key")
}

fn item_id(reference: &Value) -> Result<u32, AuthoringError> {
    let key = reference_key(reference, "Item")?;
    key.rsplit('/')
        .next()
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| AuthoringError(format!("item key {key} does not end in a numeric item id")))
}

fn find<'a>(list: &'a Value, key: &str) -> Result<&'a Value, AuthoringError> {
    list.as_array()
        .and_then(|items| items.iter().find(|item| item["identity"]["key"] == key))
        .ok_or_else(|| AuthoringError(format!("no local payload for {key}")))
}

/// One `spell.json` plus its `dependencies.json`.
pub(crate) fn spell_from_bundle(
    bundle: &Value,
    dependencies: &Value,
) -> Result<SpellDefinition, AuthoringError> {
    let spell = field(bundle, "spell")?;
    let requirements = field(spell, "requirements")?;
    // S6/S16: a Wheel of Destiny revelation spell stays uncastable until a Wheel owner exists (fails closed).
    if requirements.get("wheel_unlock").is_some() && flag(requirements, "wheel_unlock")? {
        return fail("the spell is unlocked by the Wheel of Destiny, which has no owner yet");
    }
    // S26: a monk Harmony builder or spender needs the Harmony resource, which has no owner yet (fails closed).
    if spell.get("harmony_role").is_some() {
        return fail("the spell builds or spends monk Harmony, which has no owner yet");
    }
    let costs = field(spell, "costs")?;
    let targeting = field(spell, "targeting")?;
    // S20: a cast at a chosen position needs a position cast intent, which the cast wire does not carry yet.
    if targeting.get("cast_at_position").is_some() && flag(targeting, "cast_at_position")? {
        return fail(
            "the spell is cast at a chosen position, which the cast wire does not carry yet",
        );
    }
    let carrier = match text(spell, "carrier")? {
        "instant" => Carrier::Instant {
            words: text(spell, "words")?.to_owned(),
            takes_parameter: text(targeting, "parameter")? != "none",
        },
        "rune" => {
            let rune = field(spell, "rune")?;
            Carrier::Rune {
                item: item_id(field(rune, "item")?)?,
                charges: number(rune, "charges")?,
                magic_level: number(rune, "magic_level")?,
            }
        }
        other => return fail(format!("unknown carrier {other}")),
    };
    let vocations = field(requirements, "vocations")?
        .as_array()
        .ok_or_else(|| AuthoringError("vocations is not an array".into()))?
        .iter()
        .map(|v| {
            v.as_str()
                .and_then(Vocation::from_key)
                .ok_or_else(|| AuthoringError(format!("unknown vocation {v}")))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mana = match (costs.get("mana"), costs.get("mana_percent")) {
        (Some(_), None) => ManaCost::Fixed(number(costs, "mana")?),
        (None, Some(_)) => ManaCost::PercentOfMaximum(number(costs, "mana_percent")?),
        _ => return fail("exactly one of mana and mana_percent is required"),
    };
    let groups = field(spell, "groups")?
        .as_array()
        .ok_or_else(|| AuthoringError("groups is not an array".into()))?
        .iter()
        .map(|group| {
            Ok(CooldownGroup {
                key: text(group, "group")?.to_owned(),
                cooldown_micros: u64::from(number(group, "cooldown_ms")?) * 1000,
            })
        })
        .collect::<Result<Vec<_>, AuthoringError>>()?;
    let base_power = spell
        .get("base_power")
        .map(|v| {
            v.as_i64()
                .ok_or_else(|| AuthoringError("base_power".into()))
        })
        .transpose()?;
    let range_tiles = targeting
        .get("range_tiles")
        .map(|_| number(targeting, "range_tiles"))
        .transpose()?;
    let execution = field(spell, "execution")?;
    let mut chain = None;
    let execution = if let Some(conjure) = execution.get("conjure") {
        Execution::Conjure {
            reagent: conjure.get("reagent").map(item_id).transpose()?,
            result: item_id(field(conjure, "result")?)?,
            count: number(conjure, "count")?,
        }
    } else if let Some(ability) = execution.get("ability") {
        let (effects, ability_chain) =
            ability_effects(reference_key(ability, "Ability")?, dependencies)?;
        chain = ability_chain;
        Execution::Effects(effects)
    } else if let Some(native) = execution.get("native_behavior") {
        match text(native, "key")? {
            "party_buff" => {
                if mana != ManaCost::Fixed(0) {
                    return fail(
                        "a party_buff takes its mana from its parameters; costs.mana must be 0",
                    );
                }
                Execution::PartyBuff(party_buff(field(native, "parameters")?, dependencies)?)
            }
            // S7/D13: a key without an implementation is rejected.
            other => {
                return fail(format!(
                    "the native behaviour {other} has no implementation in this core (S7, D13)"
                ));
            }
        }
    } else {
        return fail("unknown execution");
    };
    Ok(SpellDefinition {
        key: text(field(spell, "identity")?, "key")?.to_owned(),
        name: text(spell, "name")?.to_owned(),
        carrier,
        vocations,
        level: number(requirements, "level")?,
        premium: flag(requirements, "premium")?,
        learning_required: flag(requirements, "learning_required")?,
        mana,
        soul: number(costs, "soul")?,
        cooldown_micros: u64::from(number(spell, "cooldown_ms")?) * 1000,
        groups,
        needs_target: flag(targeting, "needs_target")?,
        target_or_direction: targeting.get("target_or_direction").is_some()
            && flag(targeting, "target_or_direction")?,
        self_target: flag(targeting, "self_target")?,
        aggressive: flag(targeting, "aggressive")?,
        range_tiles,
        base_power,
        execution,
        chain,
    })
}

/// The effects of an Ability and its chain, if any.
type AbilityEffects = (Vec<SpellEffect>, Option<ChainSpec>);

fn ability_effects(key: &str, dependencies: &Value) -> Result<AbilityEffects, AuthoringError> {
    let ability = find(field(dependencies, "abilities")?, key)?;
    if ability.get("variants").is_some() {
        return fail(format!(
            "{key} picks random variants, which this core does not resolve"
        ));
    }
    let chain = ability.get("chain").map(chain_spec).transpose()?;
    let effects = field(ability, "effects")?
        .as_array()
        .ok_or_else(|| AuthoringError("effects is not an array".into()))?
        .iter()
        .map(|reference| spell_effect(reference, dependencies))
        .collect::<Result<_, AuthoringError>>()?;
    Ok((effects, chain))
}

/// The Effect an `EffectRef` names in the bundle's dependencies.
fn spell_effect(reference: &Value, dependencies: &Value) -> Result<SpellEffect, AuthoringError> {
    let key = reference_key(reference, "Effect")?;
    let effect = find(field(dependencies, "effects")?, key)?;
    Ok(match text(effect, "operation")? {
        "damage" => SpellEffect::Damage {
            damage_type: text(effect, "damage_type")?.to_owned(),
            formula: effect_formula(effect, dependencies)?,
        },
        "heal" => SpellEffect::Heal {
            formula: effect_formula(effect, dependencies)?,
        },
        "remove_condition" => SpellEffect::RemoveCondition {
            condition: text(effect, "removed_condition")?.to_owned(),
        },
        other => SpellEffect::Other {
            operation: other.to_owned(),
            effect: key.to_owned(),
        },
    })
}

/// `native_behavior` `party_buff` (part C.3). Only the accepted values are admitted: the area on
/// the caster's floor, a party required, rounding up; anything else fails closed.
fn party_buff(parameters: &Value, dependencies: &Value) -> Result<PartyBuffSpec, AuthoringError> {
    const FIELDS: [&str; 6] = [
        "area",
        "same_floor",
        "min_affected",
        "requires_party",
        "mana",
        "effect",
    ];
    let object = parameters
        .as_object()
        .ok_or_else(|| AuthoringError("party_buff parameters are not an object".into()))?;
    if let Some(unknown) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return fail(format!("unknown party_buff parameter {unknown}"));
    }
    if !flag(parameters, "same_floor")? {
        return fail("a party_buff across floors is not resolved by this core (C.3 Q13)");
    }
    if !flag(parameters, "requires_party")? {
        return fail("a party_buff without a party is not resolved by this core");
    }
    let min_affected = match number(parameters, "min_affected")? {
        0 => return fail("party_buff min_affected must be positive"),
        value => value,
    };
    Ok(PartyBuffSpec {
        area: area_matrix(field(parameters, "area")?)?,
        min_affected,
        mana: party_mana(field(parameters, "mana")?)?,
        effects: vec![spell_effect(field(parameters, "effect")?, dependencies)?],
    })
}

/// An areaMatrix (monster D12 cells) as offsets from its centre; it is not rotated.
fn area_matrix(matrix: &Value) -> Result<BTreeSet<(i32, i32)>, AuthoringError> {
    let rows = matrix
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| AuthoringError("area is not a non-empty array of rows".into()))?
        .iter()
        .map(|row| {
            row.as_str()
                .filter(|row| !row.is_empty())
                .ok_or_else(|| AuthoringError("an area row is not a non-empty string".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let width = rows[0].len();
    let mut centre = None;
    let mut cells = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        if row.len() != width {
            return fail("the area is not rectangular");
        }
        for (x, cell) in row.bytes().enumerate() {
            let position = (
                i32::try_from(x).map_err(|_| AuthoringError("area too wide".into()))?,
                i32::try_from(y).map_err(|_| AuthoringError("area too tall".into()))?,
            );
            match cell {
                b'.' => {}
                b'x' => cells.push(position),
                b'c' | b'C' => {
                    if centre.replace(position).is_some() {
                        return fail("the area has more than one centre");
                    }
                    if cell == b'C' {
                        cells.push(position);
                    }
                }
                _ => return fail("unknown area cell"),
            }
        }
    }
    let (cx, cy) = centre.ok_or_else(|| AuthoringError("the area has no centre".into()))?;
    Ok(cells.into_iter().map(|(x, y)| (x - cx, y - cy)).collect())
}

fn party_mana(mana: &Value) -> Result<PartyMana, AuthoringError> {
    let object = mana
        .as_object()
        .ok_or_else(|| AuthoringError("party_buff mana is not an object".into()))?;
    let allowed: &[&str] = match text(mana, "mode")? {
        "fixed" => &["mode", "base"],
        "scaled" => &["mode", "base", "falloff", "rounding"],
        other => return fail(format!("unknown party_buff mana mode {other}")),
    };
    if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return fail(format!("unknown party_buff mana field {unknown}"));
    }
    let base = number(mana, "base")?;
    if text(mana, "mode")? == "fixed" {
        return Ok(PartyMana::Fixed(base));
    }
    if text(mana, "rounding")? != "up" {
        return fail("party_buff mana rounding must be up");
    }
    // The falloff is a whole percent in (0, 1] (0.9 accepted), kept as an exact fraction.
    let falloff = field(mana, "falloff")?
        .as_f64()
        .ok_or_else(|| AuthoringError("party_buff falloff is not a number".into()))?;
    let percent = (falloff * 100.0).round();
    if !(1.0..=100.0).contains(&percent) || (falloff * 100.0 - percent).abs() > 1e-9 {
        return fail("party_buff falloff must be a whole percent in (0, 1]");
    }
    // `percent` is an integer in 1..=100 here, so the cast is exact.
    let percent = percent as u32;
    let divisor = gcd(percent, 100);
    Ok(PartyMana::Scaled {
        base,
        numerator: percent / divisor,
        denominator: 100 / divisor,
    })
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `Ability.chain` of a player spell (D12, S23). Backtracking and the target filters are not
/// resolved: no plain player chain spell uses them, and the support chains need a per-creature
/// behaviour of their own (chain §4.2).
fn chain_spec(chain: &Value) -> Result<ChainSpec, AuthoringError> {
    const FIELDS: [&str; 8] = [
        "max_targets",
        "range_tiles",
        "backtracking",
        "chain_asset_binding",
        "target_filter",
        "shape",
        "initial_range_tiles",
        "damage_step_percent",
    ];
    let object = chain
        .as_object()
        .ok_or_else(|| AuthoringError("chain is not an object".into()))?;
    if let Some(unknown) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return fail(format!("unknown chain field {unknown}"));
    }
    if flag(chain, "backtracking")? {
        return fail("a backtracking chain is not resolved by this core");
    }
    if chain.get("target_filter").is_some() {
        return fail(
            "a chain target filter needs a per-creature behaviour, which has no owner yet",
        );
    }
    let positive = |name: &str| match number(chain, name)? {
        0 => fail(format!("chain {name} must be positive")),
        value => Ok(value),
    };
    let range_tiles = positive("range_tiles")?;
    let shape = match chain
        .get("shape")
        .map(|_| text(chain, "shape"))
        .transpose()?
    {
        None | Some("sequential") => ChainShape::Sequential,
        Some("fork") => ChainShape::Fork,
        Some(other) => return fail(format!("unknown chain shape {other}")),
    };
    let damage_step_percent = match chain.get("damage_step_percent") {
        None => 0,
        Some(value) => value
            .as_i64()
            .filter(|percent| (-100..=100).contains(percent))
            .and_then(|percent| i32::try_from(percent).ok())
            .ok_or_else(|| {
                AuthoringError("chain damage_step_percent is not in -100..=100".into())
            })?,
    };
    Ok(ChainSpec {
        max_targets: positive("max_targets")?,
        range_tiles,
        initial_range_tiles: match chain.get("initial_range_tiles") {
            None => range_tiles,
            Some(_) => positive("initial_range_tiles")?,
        },
        shape,
        damage_step_percent,
        asset_binding: chain
            .get("chain_asset_binding")
            .map(|_| text(chain, "chain_asset_binding").map(str::to_owned))
            .transpose()?,
    })
}

fn effect_formula(effect: &Value, dependencies: &Value) -> Result<Formula, AuthoringError> {
    let formula = find(
        field(dependencies, "formulas")?,
        reference_key(field(effect, "formula")?, "Formula")?,
    )?;
    if text(formula, "kind")? != "player_expression" {
        return fail("a player cast needs a player_expression formula");
    }
    Ok(Formula {
        minimum: expression(field(formula, "minimum")?, 1)?,
        maximum: expression(field(formula, "maximum")?, 1)?,
    })
}

fn expression(value: &Value, depth: usize) -> Result<Expression, AuthoringError> {
    if depth > MAX_EXPRESSION_DEPTH {
        return fail("formula expression is too deep");
    }
    if let Some(constant) = value.get("const") {
        let literal = constant
            .as_str()
            .ok_or_else(|| AuthoringError("const is not a decimal string".into()))?;
        let parsed: f64 = literal
            .parse()
            .map_err(|_| AuthoringError(format!("bad decimal {literal}")))?;
        return Ok(Expression::Const(parsed));
    }
    if let Some(variable) = value.get("var") {
        let name = variable.as_str().unwrap_or_default();
        return Input::from_key(name)
            .map(Expression::Var)
            .ok_or_else(|| AuthoringError(format!("unknown input {name}")));
    }
    let args = field(value, "args")?
        .as_array()
        .ok_or_else(|| AuthoringError("args is not an array".into()))?
        .iter()
        .map(|arg| expression(arg, depth + 1))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(function) = value.get("fn") {
        return match (function.as_str(), <[Expression; 1]>::try_from(args)) {
            (Some("level_base_damage_healing"), Ok([level])) => {
                Ok(Expression::LevelBaseDamageHealing(Box::new(level)))
            }
            _ => fail("unknown formula function or arity"),
        };
    }
    let operation = text(value, "op")?;
    let unary = |op| match <[Expression; 1]>::try_from(args.clone()) {
        Ok([inner]) => Ok(Expression::Unary(op, Box::new(inner))),
        Err(_) => fail(format!("{operation} takes one argument")),
    };
    let binary = |op| match <[Expression; 2]>::try_from(args.clone()) {
        Ok([left, right]) => Ok(Expression::Binary(op, Box::new(left), Box::new(right))),
        Err(_) => fail(format!("{operation} takes two arguments")),
    };
    match operation {
        "neg" => unary(Unary::Neg),
        "floor" => unary(Unary::Floor),
        "ceil" => unary(Unary::Ceil),
        "sqrt" => unary(Unary::Sqrt),
        "abs" => unary(Unary::Abs),
        "add" => binary(Binary::Add),
        "sub" => binary(Binary::Sub),
        "mul" => binary(Binary::Mul),
        "div" => binary(Binary::Div),
        "min" | "max" if args.len() >= 2 => Ok(Expression::Extremum(
            if operation == "min" {
                Extremum::Min
            } else {
                Extremum::Max
            },
            args,
        )),
        other => fail(format!("unknown formula operation {other}")),
    }
}
