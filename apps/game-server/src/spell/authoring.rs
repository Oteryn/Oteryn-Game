//! Reads a candidate Spell authoring bundle (`tools/content-schema/spell-authoring`, schema v1) into a
//! [`SpellDefinition`]. The bundle's own validator (`validate_spell.py`) owns structural validation;
//! this reader rejects anything it cannot represent instead of guessing. The admitted content route
//! (WorldProject/v2 lowering) replaces this reader in P3b.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde_json::Value;

use super::formula::{Binary, Expression, Extremum, Formula, Input, MAX_EXPRESSION_DEPTH, Unary};
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
    let execution = if let Some(conjure) = execution.get("conjure") {
        Execution::Conjure {
            reagent: conjure.get("reagent").map(item_id).transpose()?,
            result: item_id(field(conjure, "result")?)?,
            count: number(conjure, "count")?,
        }
    } else if let Some(ability) = execution.get("ability") {
        Execution::Effects(ability_effects(
            reference_key(ability, "Ability")?,
            dependencies,
        )?)
    } else {
        return fail("the execution is a native behaviour, which this core does not resolve");
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
        self_target: flag(targeting, "self_target")?,
        aggressive: flag(targeting, "aggressive")?,
        range_tiles,
        base_power,
        execution,
    })
}

fn ability_effects(key: &str, dependencies: &Value) -> Result<Vec<SpellEffect>, AuthoringError> {
    let ability = find(field(dependencies, "abilities")?, key)?;
    if ability.get("variants").is_some() {
        return fail(format!(
            "{key} picks random variants, which this core does not resolve"
        ));
    }
    // A chain would otherwise be cast on the first creature only (OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md).
    if ability.get("chain").is_some() {
        return fail(format!(
            "{key} hits a chain of creatures, which this core does not resolve yet"
        ));
    }
    field(ability, "effects")?
        .as_array()
        .ok_or_else(|| AuthoringError("effects is not an array".into()))?
        .iter()
        .map(|reference| {
            let effect = find(
                field(dependencies, "effects")?,
                reference_key(reference, "Effect")?,
            )?;
            let operation = text(effect, "operation")?;
            Ok(match operation {
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
                },
            })
        })
        .collect()
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
