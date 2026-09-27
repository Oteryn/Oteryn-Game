//! Player damage/heal formulas (`player_expression`, spell schema S5).
//!
//! A formula is the source expression tree over declared inputs. It is evaluated in IEEE-754 double
//! in authored operation order, each bound is truncated toward zero, and the world damage
//! distribution draws the value between the bounds. The level contribution is the one world curve
//! of the official 2022 scaling table.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Deepest expression tree an admitted formula may carry (authoring limit, `validate_spell.py`).
pub(crate) const MAX_EXPRESSION_DEPTH: usize = 32;

/// Damage and healing bonus of a character level (official scaling since patch 13.05.12657):
/// +1 every 5 levels up to 500, every 6 up to 1100, every 7 up to 1800, and so on. Exact integer
/// form of `S = floor((sqrt(2L + 2025) + 5) / 10)`, `B = floor((L + 1000) / S) + 50S - 450`.
pub(crate) fn level_base_damage_healing(level: u32) -> i64 {
    let level = u64::from(level);
    // floor((sqrt(x) + 5) / 10) == floor((isqrt(x) + 5) / 10): adding 5 and dividing by 10 only
    // crosses an integer at an integer square root value.
    let step = (isqrt(2 * level + 2025) + 5) / 10;
    let bonus = (level + 1000) / step + 50 * step;
    i64::try_from(bonus).unwrap_or(i64::MAX) - 450
}

fn isqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut low = 1_u64;
    let mut high = value.min(u64::from(u32::MAX));
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if mid.checked_mul(mid).is_some_and(|square| square <= value) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Input {
    Level,
    MagicLevel,
    BasePower,
    AttackSkill,
    AttackValue,
    AttackFactor,
    ShieldingSkill,
}

impl Input {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "level" => Self::Level,
            "magic_level" => Self::MagicLevel,
            "base_power" => Self::BasePower,
            "attack_skill" => Self::AttackSkill,
            "attack_value" => Self::AttackValue,
            "attack_factor" => Self::AttackFactor,
            "shielding_skill" => Self::ShieldingSkill,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unary {
    Neg,
    Floor,
    Ceil,
    Sqrt,
    Abs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Binary {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Extremum {
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expression {
    Const(f64),
    Var(Input),
    Unary(Unary, Box<Expression>),
    Binary(Binary, Box<Expression>, Box<Expression>),
    Extremum(Extremum, Vec<Expression>),
    LevelBaseDamageHealing(Box<Expression>),
}

/// Formula inputs of one cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FormulaInputs {
    pub(crate) level: u32,
    pub(crate) magic_level: u32,
    pub(crate) base_power: Option<i64>,
    pub(crate) attack_skill: u32,
    pub(crate) attack_value: u32,
    pub(crate) attack_factor: f64,
    pub(crate) shielding_skill: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormulaError {
    MissingBasePower,
    DivisionByZero,
    NegativeSquareRoot,
    NonFinite,
    LevelOutOfRange,
    NegativeMagnitude,
    InvertedBounds,
}

impl Display for FormulaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingBasePower => "the formula reads base_power but the spell has none",
            Self::DivisionByZero => "division by zero",
            Self::NegativeSquareRoot => "square root of a negative value",
            Self::NonFinite => "non-finite formula value",
            Self::LevelOutOfRange => "level argument outside the level curve domain",
            Self::NegativeMagnitude => "a formula bound is negative",
            Self::InvertedBounds => "the minimum bound exceeds the maximum bound",
        })
    }
}

impl Error for FormulaError {}

impl Expression {
    pub(crate) fn depth(&self) -> usize {
        match self {
            Self::Const(_) | Self::Var(_) => 1,
            Self::Unary(_, inner) | Self::LevelBaseDamageHealing(inner) => 1 + inner.depth(),
            Self::Binary(_, left, right) => 1 + left.depth().max(right.depth()),
            Self::Extremum(_, args) => 1 + args.iter().map(Self::depth).max().unwrap_or(0),
        }
    }

    pub(crate) fn evaluate(&self, inputs: &FormulaInputs) -> Result<f64, FormulaError> {
        let value = match self {
            Self::Const(value) => *value,
            Self::Var(input) => match input {
                Input::Level => f64::from(inputs.level),
                Input::MagicLevel => f64::from(inputs.magic_level),
                Input::BasePower => {
                    let power = inputs.base_power.ok_or(FormulaError::MissingBasePower)?;
                    i32::try_from(power)
                        .map(f64::from)
                        .map_err(|_| FormulaError::NonFinite)?
                }
                Input::AttackSkill => f64::from(inputs.attack_skill),
                Input::AttackValue => f64::from(inputs.attack_value),
                Input::AttackFactor => inputs.attack_factor,
                Input::ShieldingSkill => f64::from(inputs.shielding_skill),
            },
            Self::Unary(op, inner) => {
                let value = inner.evaluate(inputs)?;
                match op {
                    Unary::Neg => -value,
                    Unary::Floor => value.floor(),
                    Unary::Ceil => value.ceil(),
                    Unary::Abs => value.abs(),
                    Unary::Sqrt if value < 0.0 => return Err(FormulaError::NegativeSquareRoot),
                    Unary::Sqrt => value.sqrt(),
                }
            }
            Self::Binary(op, left, right) => {
                let (left, right) = (left.evaluate(inputs)?, right.evaluate(inputs)?);
                match op {
                    Binary::Add => left + right,
                    Binary::Sub => left - right,
                    Binary::Mul => left * right,
                    Binary::Div if right == 0.0 => return Err(FormulaError::DivisionByZero),
                    Binary::Div => left / right,
                }
            }
            Self::Extremum(op, args) => {
                let mut values = args.iter().map(|arg| arg.evaluate(inputs));
                let first = values.next().ok_or(FormulaError::NonFinite)??;
                values.try_fold(first, |best, next| {
                    let next = next?;
                    Ok(match op {
                        Extremum::Min => best.min(next),
                        Extremum::Max => best.max(next),
                    })
                })?
            }
            Self::LevelBaseDamageHealing(level) => {
                let level = level.evaluate(inputs)?;
                if !(0.0..=f64::from(u32::MAX)).contains(&level) {
                    return Err(FormulaError::LevelOutOfRange);
                }
                // The argument is a level; a fractional level is truncated like the engine's integer level.
                let bonus = level_base_damage_healing(level.trunc() as u32);
                i32::try_from(bonus)
                    .map(f64::from)
                    .map_err(|_| FormulaError::NonFinite)?
            }
        };
        if value.is_finite() {
            Ok(value)
        } else {
            Err(FormulaError::NonFinite)
        }
    }
}

/// Magnitude range of a damage or heal effect.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Formula {
    pub(crate) minimum: Expression,
    pub(crate) maximum: Expression,
}

impl Formula {
    /// Truncated `(minimum, maximum)` magnitude for these inputs.
    pub(crate) fn bounds(&self, inputs: &FormulaInputs) -> Result<(i64, i64), FormulaError> {
        let low = truncate(self.minimum.evaluate(inputs)?)?;
        let high = truncate(self.maximum.evaluate(inputs)?)?;
        if low < 0 || high < 0 {
            return Err(FormulaError::NegativeMagnitude);
        }
        if low > high {
            return Err(FormulaError::InvertedBounds);
        }
        Ok((low, high))
    }
}

/// Truncation toward zero, as `LuaScriptInterface::getNumber<int32_t>` does with the callback values.
fn truncate(value: f64) -> Result<i64, FormulaError> {
    let value = value.trunc();
    if value.abs() > f64::from(i32::MAX) {
        return Err(FormulaError::NonFinite);
    }
    // In range after the check above, so the cast is exact.
    Ok(value as i64)
}
