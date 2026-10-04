//! Pure plans for the twelve hash-qualified actor/equipment authoring profiles.
//!
//! The profile reader must first compare the entire payload to its canonical
//! qualified profile. These functions also reject missing numeric facts and
//! malformed required fields. They authorize no cast, mutate no actor and own
//! no persistence: the existing fenced PRIMARY COMMIT applies a returned plan.
//! Equipment rolls are requests to the existing world distribution. Sweeping
//! shares its input/bounds snapshot but has two distinct random draw requests.

use std::collections::{BTreeMap, BTreeSet};

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::Value;

use super::Vocation;
use super::formula::level_base_damage_healing;
use super::harmony::HarmonyMultiplier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActorPlanError {
    InvalidParameters(&'static str),
    InvalidHarmony,
    MissingOwnedFact(&'static str),
    IneligibleVocation,
    TimeOverflow,
    InvalidBounds,
    MissingShield { message: String, effect: String },
}

fn required<'a>(value: &'a Value, key: &'static str) -> Result<&'a Value, ActorPlanError> {
    value.get(key).ok_or(ActorPlanError::InvalidParameters(key))
}

fn text<'a>(value: &'a Value, key: &'static str) -> Result<&'a str, ActorPlanError> {
    required(value, key)?
        .as_str()
        .ok_or(ActorPlanError::InvalidParameters(key))
}

fn flag(value: &Value, key: &'static str) -> Result<bool, ActorPlanError> {
    required(value, key)?
        .as_bool()
        .ok_or(ActorPlanError::InvalidParameters(key))
}

fn integer(value: &Value, key: &'static str) -> Result<i64, ActorPlanError> {
    required(value, key)?
        .as_i64()
        .ok_or(ActorPlanError::InvalidParameters(key))
}

fn percent(value: &Value, key: &'static str) -> Result<i32, ActorPlanError> {
    let result = integer(value, key)?;
    if !(-100..=100).contains(&result) {
        return Err(ActorPlanError::InvalidParameters(key));
    }
    Ok(result as i32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MagnitudeBounds {
    pub(crate) minimum: i64,
    pub(crate) maximum: i64,
}

fn magnitude(value: f64) -> Result<i64, ActorPlanError> {
    if !value.is_finite() || value < 0.0 || value >= i64::MAX as f64 {
        return Err(ActorPlanError::InvalidBounds);
    }
    Ok(value.trunc() as i64)
}

fn bounds(low: f64, high: f64) -> Result<MagnitudeBounds, ActorPlanError> {
    let result = MagnitudeBounds {
        minimum: magnitude(low)?,
        maximum: magnitude(high)?,
    };
    if result.minimum > result.maximum {
        return Err(ActorPlanError::InvalidBounds);
    }
    Ok(result)
}

// A small native expression evaluator is necessary for the two additional
// local variables gained_charges and sweeping_skill_bonus. Only the closed
// canonical AST is admitted; there is no arbitrary function or Lua execution.
fn evaluate(expr: &Value, vars: &BTreeMap<&str, f64>, depth: usize) -> Result<f64, ActorPlanError> {
    if depth > 32 {
        return Err(ActorPlanError::InvalidParameters("expression_depth"));
    }
    let result = if let Some(value) = expr.get("const") {
        value
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or(ActorPlanError::InvalidParameters("const"))?
    } else if let Some(value) = expr.get("var") {
        let key = value
            .as_str()
            .ok_or(ActorPlanError::InvalidParameters("var"))?;
        *vars
            .get(key)
            .ok_or(ActorPlanError::MissingOwnedFact("expression_input"))?
    } else {
        let args = required(expr, "args")?
            .as_array()
            .ok_or(ActorPlanError::InvalidParameters("args"))?;
        if let Some(function) = expr.get("fn") {
            if function.as_str() != Some("level_base_damage_healing") || args.len() != 1 {
                return Err(ActorPlanError::InvalidParameters("fn"));
            }
            let level = evaluate(&args[0], vars, depth + 1)?;
            if level < 1.0 || level > f64::from(u32::MAX) || level.fract() != 0.0 {
                return Err(ActorPlanError::MissingOwnedFact("level"));
            }
            level_base_damage_healing(level as u32) as f64
        } else {
            let operation = text(expr, "op")?;
            let expected = match operation {
                "floor" | "ceil" => 1,
                "add" | "sub" | "mul" | "div" | "max" | "min" => 2,
                _ => return Err(ActorPlanError::InvalidParameters("op")),
            };
            if args.len() != expected {
                return Err(ActorPlanError::InvalidParameters("args"));
            }
            let a = evaluate(&args[0], vars, depth + 1)?;
            if expected == 1 {
                if operation == "floor" {
                    a.floor()
                } else {
                    a.ceil()
                }
            } else {
                let b = evaluate(&args[1], vars, depth + 1)?;
                match operation {
                    "add" => a + b,
                    "sub" => a - b,
                    "mul" => a * b,
                    "div" if b != 0.0 => a / b,
                    "max" => a.max(b),
                    "min" => a.min(b),
                    _ => return Err(ActorPlanError::InvalidBounds),
                }
            }
        }
    };
    if !result.is_finite() {
        return Err(ActorPlanError::InvalidBounds);
    }
    Ok(result)
}

fn expression_bounds(
    value: &Value,
    vars: &BTreeMap<&str, f64>,
) -> Result<MagnitudeBounds, ActorPlanError> {
    bounds(
        evaluate(required(value, "minimum")?, vars, 1)?,
        evaluate(required(value, "maximum")?, vars, 1)?,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CooldownResetRule {
    Preserve,
    Clear,
    SpenderOnly,
}

impl CooldownResetRule {
    fn read(value: &Value, key: &'static str) -> Result<Self, ActorPlanError> {
        match text(value, key)? {
            "none" | "preserve" => Ok(Self::Preserve),
            "clear" => Ok(Self::Clear),
            "spender_only" => Ok(Self::SpenderOnly),
            _ => Err(ActorPlanError::InvalidParameters(key)),
        }
    }
    pub(crate) fn clears(self, is_spender: bool) -> bool {
        self == Self::Clear || (self == Self::SpenderOnly && is_spender)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FocusFacts {
    pub(crate) harmony: u8,
    pub(crate) serene: bool,
    pub(crate) forced_until: Option<SemanticTimeMicros>,
    pub(crate) now: SemanticTimeMicros,
    pub(crate) level: u32,
    pub(crate) sustain_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HealingRollPlan {
    /// Draw once from the unchanged world healing distribution over these bounds.
    pub(crate) bounds: MagnitudeBounds,
    /// Apply Sustain after drawing, preserving the support/distribution of the roll.
    pub(crate) sustain_percent_after_draw: i32,
}

impl HealingRollPlan {
    pub(crate) fn finish_draw(&self, roll: i64) -> Result<i64, ActorPlanError> {
        if !(self.bounds.minimum..=self.bounds.maximum).contains(&roll) {
            return Err(ActorPlanError::InvalidBounds);
        }
        i64::try_from(i128::from(roll) * i128::from(100 + self.sustain_percent_after_draw) / 100)
            .map_err(|_| ActorPlanError::InvalidBounds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FocusPlan {
    pub(crate) harmony_after: u8,
    pub(crate) gained_charges: u8,
    pub(crate) serene_after: bool,
    pub(crate) forced_until: Option<SemanticTimeMicros>,
    pub(crate) arm_forced_serene: bool,
    pub(crate) effect_asset_binding: String,
    pub(crate) healing: Option<HealingRollPlan>,
    pub(crate) individual_reset: CooldownResetRule,
    pub(crate) group_without_spell_reset: CooldownResetRule,
    pub(crate) group_with_spell_reset: CooldownResetRule,
    pub(crate) rearm_cast_cooldowns: bool,
}

pub(crate) fn plan_focus(
    parameters: &Value,
    facts: FocusFacts,
) -> Result<FocusPlan, ActorPlanError> {
    if !flag(parameters, "fill_harmony")? || integer(parameters, "harmony_max")? != 5 {
        return Err(ActorPlanError::InvalidParameters("harmony_max"));
    }
    if facts.harmony > 5 {
        return Err(ActorPlanError::InvalidHarmony);
    }
    let serene_ms = required(parameters, "serene_ms")?;
    let forced_until = if serene_ms.is_null() {
        facts.forced_until
    } else {
        let millis = serene_ms
            .as_u64()
            .ok_or(ActorPlanError::InvalidParameters("serene_ms"))?;
        if millis != 7000 {
            return Err(ActorPlanError::InvalidParameters("serene_ms"));
        }
        Some(
            facts
                .now
                .checked_add(millis * 1000)
                .map_err(|_| ActorPlanError::TimeOverflow)?,
        )
    };
    let serene_after = !serene_ms.is_null() || facts.serene;
    let gained_charges = 5 - facts.harmony;
    let healing = if gained_charges == 0 {
        None
    } else {
        let heal = required(parameters, "harmony_gain_healing")?;
        if text(heal, "sustain_application")? != "after_world_healing_roll"
            || text(heal, "sustain_quantization")? != "truncate_toward_zero"
        {
            return Err(ActorPlanError::InvalidParameters("sustain_application"));
        }
        let vars = BTreeMap::from([
            ("level", f64::from(facts.level)),
            ("gained_charges", f64::from(gained_charges)),
        ]);
        let raw = expression_bounds(required(heal, "bounds")?, &vars)?;
        let boost = if facts.sustain_active {
            percent(
                heal,
                if serene_after {
                    "serene_sustain_percent"
                } else {
                    "sustain_percent"
                },
            )?
        } else {
            0
        };
        Some(HealingRollPlan {
            bounds: raw,
            sustain_percent_after_draw: boost,
        })
    };
    let filter = required(parameters, "cooldown_reset_filter")?;
    Ok(FocusPlan {
        harmony_after: 5,
        gained_charges,
        serene_after,
        forced_until,
        arm_forced_serene: flag(parameters, "forced_serene_prevents_automatic_clear")?,
        effect_asset_binding: text(
            required(parameters, "presentation")?,
            "effect_asset_binding",
        )?
        .to_owned(),
        healing,
        individual_reset: CooldownResetRule::read(filter, "individual")?,
        group_without_spell_reset: CooldownResetRule::read(filter, "group_without_resolved_spell")?,
        group_with_spell_reset: CooldownResetRule::read(filter, "group_with_resolved_spell")?,
        rearm_cast_cooldowns: flag(parameters, "rearm_cast_cooldowns")?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StandardStance {
    BloodRage,
    Protector,
    Sharpshooter,
    Harmony,
    Justice,
    Sustain,
}

impl StandardStance {
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::BloodRage => "blood_rage",
            Self::Protector => "protector",
            Self::Sharpshooter => "sharpshooter",
            Self::Harmony => "virtue_of_harmony",
            Self::Justice => "virtue_of_justice",
            Self::Sustain => "virtue_of_sustain",
        }
    }
    pub(crate) const fn eligible(self, vocation: Vocation) -> bool {
        match self {
            Self::BloodRage | Self::Protector => matches!(vocation, Vocation::EliteKnight),
            Self::Sharpshooter => matches!(vocation, Vocation::RoyalPaladin),
            Self::Harmony | Self::Justice | Self::Sustain => {
                matches!(vocation, Vocation::Monk | Vocation::ExaltedMonk)
            }
        }
    }
    pub(crate) fn read(value: &str) -> Result<Self, ActorPlanError> {
        match value {
            "blood_rage" => Ok(Self::BloodRage),
            "protector" => Ok(Self::Protector),
            "sharpshooter" => Ok(Self::Sharpshooter),
            "virtue_of_harmony" => Ok(Self::Harmony),
            "virtue_of_justice" => Ok(Self::Justice),
            "virtue_of_sustain" => Ok(Self::Sustain),
            _ => Err(ActorPlanError::InvalidParameters("stance")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ActorSkill {
    Sword,
    Axe,
    Club,
    Distance,
    Shielding,
    Fist,
}

impl ActorSkill {
    fn read(key: &str) -> Result<Self, ActorPlanError> {
        match key {
            "sword" => Ok(Self::Sword),
            "axe" => Ok(Self::Axe),
            "club" => Ok(Self::Club),
            "distance" => Ok(Self::Distance),
            "shielding" => Ok(Self::Shielding),
            "fist" => Ok(Self::Fist),
            _ => Err(ActorPlanError::InvalidParameters("skill")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StanceModifier {
    FinalSkills {
        skills: Vec<ActorSkill>,
        percent: i32,
    },
    BaseFist {
        percent: i32,
        serene_percent: i32,
    },
    DamageTaken(i32),
    DamageDealt(i32),
    HealingDone {
        percent: i32,
        serene_percent: i32,
    },
    HarmonyBaseScale {
        percent: i32,
        serene_percent: i32,
        refund_charges: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StancePlan {
    pub(crate) before: Option<StandardStance>,
    pub(crate) after: Option<StandardStance>,
    pub(crate) modifiers: Vec<StanceModifier>,
    pub(crate) keep_on_death: bool,
    pub(crate) persist_across_sessions: bool,
    pub(crate) debit_mana_and_start_cooldowns: bool,
}

pub(crate) fn plan_stance(
    parameters: &Value,
    active: Option<StandardStance>,
    vocation: Vocation,
) -> Result<StancePlan, ActorPlanError> {
    if text(parameters, "slot")? != "standard" {
        return Err(ActorPlanError::InvalidParameters("slot"));
    }
    let eligible = required(parameters, "eligible_vocations")?
        .as_array()
        .ok_or(ActorPlanError::InvalidParameters("eligible_vocations"))?;
    if !eligible
        .iter()
        .any(|v| v.as_str().and_then(Vocation::from_key) == Some(vocation))
    {
        return Err(ActorPlanError::IneligibleVocation);
    }
    let requested = StandardStance::read(text(parameters, "stance")?)?;
    let toggling_off = active == Some(requested) && flag(parameters, "toggle_same_stance_off")?;
    if active.is_some()
        && !toggling_off
        && active != Some(requested)
        && !flag(parameters, "replace_existing")?
    {
        return Err(ActorPlanError::InvalidParameters("replace_existing"));
    }
    let after = if toggling_off { None } else { Some(requested) };
    let mut modifiers = Vec::new();
    for value in required(parameters, "modifiers")?
        .as_array()
        .ok_or(ActorPlanError::InvalidParameters("modifiers"))?
    {
        let modifier = match text(value, "kind")? {
            "skill_percent_of_final" => StanceModifier::FinalSkills {
                skills: required(value, "skills")?
                    .as_array()
                    .ok_or(ActorPlanError::InvalidParameters("skills"))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or(ActorPlanError::InvalidParameters("skill"))
                            .and_then(ActorSkill::read)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                percent: percent(value, "percent")?,
            },
            "fist_bonus_percent" if text(value, "basis")? == "base" => StanceModifier::BaseFist {
                percent: percent(value, "percent")?,
                serene_percent: percent(value, "serene_percent")?,
            },
            "damage_taken_percent" if flag(value, "exclude_healing")? => {
                StanceModifier::DamageTaken(percent(value, "percent")?)
            }
            "damage_dealt_percent" if flag(value, "exclude_healing")? => {
                StanceModifier::DamageDealt(percent(value, "percent")?)
            }
            "healing_done_percent" => StanceModifier::HealingDone {
                percent: percent(value, "percent")?,
                serene_percent: percent(value, "serene_percent")?,
            },
            "harmony_base_bonus_scale" => {
                let refund = integer(value, "spender_refund_charges")?;
                if !(0..=5).contains(&refund) {
                    return Err(ActorPlanError::InvalidHarmony);
                }
                StanceModifier::HarmonyBaseScale {
                    percent: percent(value, "percent")?,
                    serene_percent: percent(value, "serene_percent")?,
                    refund_charges: refund as u8,
                }
            }
            _ => return Err(ActorPlanError::InvalidParameters("modifier_kind")),
        };
        if after.is_some() {
            modifiers.push(modifier);
        }
    }
    Ok(StancePlan {
        before: active,
        after,
        modifiers,
        keep_on_death: flag(parameters, "keep_on_death")?,
        persist_across_sessions: flag(parameters, "persist_across_sessions")?,
        debit_mana_and_start_cooldowns: !toggling_off
            || (flag(parameters, "toggle_off_costs_mana")?
                && flag(parameters, "toggle_off_starts_cooldowns")?),
    })
}

pub(crate) fn stance_skill(
    plan: &StancePlan,
    skill: ActorSkill,
    base: u32,
    effective: u32,
    serene: bool,
) -> Result<u32, ActorPlanError> {
    let mut result = i128::from(effective);
    for modifier in &plan.modifiers {
        match modifier {
            StanceModifier::FinalSkills { skills, percent } if skills.contains(&skill) => {
                result = result * i128::from(100 + percent) / 100
            }
            StanceModifier::BaseFist {
                percent,
                serene_percent,
            } if skill == ActorSkill::Fist => {
                result += i128::from(base)
                    * i128::from(if serene { *serene_percent } else { *percent })
                    / 100
            }
            _ => {}
        }
    }
    u32::try_from(result).map_err(|_| ActorPlanError::InvalidBounds)
}

/// Sustain's qualified scope excludes item healing and unrelated sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HealingOrigin {
    MonkSpell,
    HarmonyGain,
    Other,
}

pub(crate) fn stance_damage(
    plan: &StancePlan,
    amount: i64,
    incoming: bool,
    healing: bool,
    serene: bool,
    healing_origin: HealingOrigin,
) -> Result<i64, ActorPlanError> {
    let mut result = i128::from(amount);
    for modifier in &plan.modifiers {
        let delta = match modifier {
            StanceModifier::DamageTaken(p) if incoming && !healing => Some(*p),
            StanceModifier::DamageDealt(p) if !incoming && !healing => Some(*p),
            StanceModifier::HealingDone {
                percent,
                serene_percent,
            } if !incoming && healing && healing_origin != HealingOrigin::Other => {
                Some(if serene { *serene_percent } else { *percent })
            }
            _ => None,
        };
        if let Some(delta) = delta {
            result = result * i128::from(100 + delta) / 100;
        }
    }
    i64::try_from(result).map_err(|_| ActorPlanError::InvalidBounds)
}

/// Reconciliation hook for the owner changing vocation; D145 retention does
/// not preserve a standard stance after its vocation becomes ineligible.
pub(crate) fn retain_stance(
    parameters: &Value,
    active: Option<StandardStance>,
    vocation: Vocation,
) -> Result<Option<StandardStance>, ActorPlanError> {
    let Some(current) = active else {
        return Ok(None);
    };
    if current != StandardStance::read(text(parameters, "stance")?)? {
        return Err(ActorPlanError::InvalidParameters("active_stance_profile"));
    }
    let eligible = required(parameters, "eligible_vocations")?
        .as_array()
        .ok_or(ActorPlanError::InvalidParameters("eligible_vocations"))?;
    if eligible
        .iter()
        .any(|v| v.as_str().and_then(Vocation::from_key) == Some(vocation))
    {
        return Ok(active);
    }
    if !flag(parameters, "prune_on_ineligible_vocation")? {
        return Err(ActorPlanError::InvalidParameters(
            "prune_on_ineligible_vocation",
        ));
    }
    Ok(None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackOrigin {
    Melee,
    Ranged,
    Fist,
    Spell,
    Rune,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShieldDebuffFacts {
    pub(crate) expires_at: SemanticTimeMicros,
    pub(crate) now: SemanticTimeMicros,
    pub(crate) origin: AttackOrigin,
    pub(crate) primary: i64,
    pub(crate) secondary: i64,
    /// Caster's grade captured when this condition was applied, never the victim's grade.
    pub(crate) wheel_grade: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShieldDebuffPlan {
    pub(crate) primary: i64,
    pub(crate) secondary: i64,
    pub(crate) consume: bool,
    pub(crate) expired: bool,
}

pub(crate) fn shield_debuff(
    parameters: &Value,
    facts: ShieldDebuffFacts,
) -> Result<ShieldDebuffPlan, ActorPlanError> {
    let spec = required(parameters, "next_auto_attack_reduction")?;
    let expired = facts.now >= facts.expires_at;
    let auto = matches!(
        facts.origin,
        AttackOrigin::Melee | AttackOrigin::Ranged | AttackOrigin::Fist
    );
    let consume = !expired && auto;
    let bonus = if facts.wheel_grade >= 2 {
        percent(spec, "wheel_grade_2_additional_percent")?
    } else {
        0
    };
    let reduction = (percent(spec, "percent")? + bonus).clamp(0, 100);
    let scale = |amount: i64| -> Result<i64, ActorPlanError> {
        if !consume {
            return Ok(amount);
        }
        i64::try_from(i128::from(amount) * i128::from(100 - reduction) / 100)
            .map_err(|_| ActorPlanError::InvalidBounds)
    };
    Ok(ShieldDebuffPlan {
        primary: scale(facts.primary)?,
        secondary: scale(facts.secondary)?,
        consume,
        expired,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamageElement {
    Physical,
    Energy,
    Earth,
    Fire,
    Ice,
    Holy,
    Death,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandKind {
    Weapon(ActorSkill),
    Shield,
    Ammunition,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandItem {
    pub(crate) kind: HandKind,
    pub(crate) attack: u32,
    pub(crate) defense: u32,
    pub(crate) bond: Option<DamageElement>,
}

#[derive(Debug, Clone)]
pub(crate) struct EquipmentFacts {
    pub(crate) level: u32,
    pub(crate) left: Option<HandItem>,
    pub(crate) right: Option<HandItem>,
    pub(crate) effective_skills: BTreeMap<ActorSkill, u32>,
    pub(crate) flurry_enlarged: bool,
    pub(crate) shield_slam_grade: u8,
    pub(crate) harmony: Option<HarmonyMultiplier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttackArea {
    Target,
    Matrix {
        rows: Vec<String>,
        directional: bool,
    },
}

fn matrix(
    value: &Value,
    key: &'static str,
    directional: bool,
) -> Result<AttackArea, ActorPlanError> {
    let rows = required(value, key)?
        .as_array()
        .ok_or(ActorPlanError::InvalidParameters(key))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or(ActorPlanError::InvalidParameters(key))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let width = rows.first().map_or(0, String::len);
    if width == 0
        || rows.len() > 32
        || width > 32
        || rows.iter().any(|row| {
            row.len() != width || row.chars().any(|c| !matches!(c, '.' | 'x' | 'c' | 'C'))
        })
        || rows
            .iter()
            .map(|row| row.chars().filter(|c| matches!(c, 'c' | 'C')).count())
            .sum::<usize>()
            != 1
    {
        return Err(ActorPlanError::InvalidParameters(key));
    }
    Ok(AttackArea::Matrix { rows, directional })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NextAutoAttackEffect {
    pub(crate) duration_ms: u64,
    pub(crate) reduction_percent: i32,
    pub(crate) wheel_bonus_percent: i32,
    pub(crate) affects_players: bool,
    pub(crate) refresh_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttackComponentPlan {
    pub(crate) area: AttackArea,
    pub(crate) bounds: MagnitudeBounds,
    /// Each component gets a separate draw from the owner's world RNG stream.
    pub(crate) draw_index: u8,
    pub(crate) next_auto_attack: Option<NextAutoAttackEffect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHarmonyRole {
    None,
    Builder,
    Spender,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentAttackPlan {
    pub(crate) components: Vec<AttackComponentPlan>,
    pub(crate) element: DamageElement,
    pub(crate) blocked_by_armor: bool,
    pub(crate) harmony_role: NativeHarmonyRole,
    /// Prevents an adapter from applying the whole-bound multiplier a second time.
    pub(crate) harmony_applied_to_bounds: bool,
}

pub(crate) fn equipment_attack(
    parameters: &Value,
    facts: &EquipmentFacts,
) -> Result<EquipmentAttackPlan, ActorPlanError> {
    let name = text(parameters, "spell")?;
    let power = integer(parameters, "base_power")?;
    if power <= 0 {
        return Err(ActorPlanError::InvalidParameters("base_power"));
    }
    let shield = matches!(name, "shield_bash" | "shield_slam");
    if !shield && !matches!(name, "flurry_of_blows" | "sweeping_takedown") {
        return Err(ActorPlanError::InvalidParameters("spell"));
    }
    let hands = [facts.left.as_ref(), facts.right.as_ref()];
    let item = hands.into_iter().flatten().find(|item| {
        if shield {
            item.kind == HandKind::Shield
        } else {
            matches!(item.kind, HandKind::Weapon(_))
        }
    });
    let mut vars = BTreeMap::from([
        ("level", f64::from(facts.level)),
        ("base_power", power as f64),
    ]);
    let element;
    if shield {
        let Some(item) = item else {
            let refusal = required(parameters, "refusal")?;
            return Err(ActorPlanError::MissingShield {
                message: text(refusal, "message")?.to_owned(),
                effect: text(refusal, "effect_asset_binding")?.to_owned(),
            });
        };
        let skill = facts
            .effective_skills
            .get(&ActorSkill::Shielding)
            .ok_or(ActorPlanError::MissingOwnedFact("shielding_skill"))?;
        vars.insert("shielding_skill", f64::from(*skill));
        vars.insert("shield_defense", f64::from(item.defense));
        element = DamageElement::Physical;
    } else {
        let (skill, attack) = if let Some(item) = item {
            let HandKind::Weapon(skill_key) = item.kind else {
                return Err(ActorPlanError::MissingOwnedFact("weapon"));
            };
            let skill = facts
                .effective_skills
                .get(&skill_key)
                .ok_or(ActorPlanError::MissingOwnedFact("weapon_skill"))?;
            (*skill, item.attack)
        } else {
            let unarmed = required(required(parameters, "input")?, "unarmed")?;
            let skill = u32::try_from(integer(unarmed, "attack_skill")?)
                .map_err(|_| ActorPlanError::InvalidBounds)?;
            let attack = u32::try_from(integer(unarmed, "attack_value")?)
                .map_err(|_| ActorPlanError::InvalidBounds)?;
            (skill, attack)
        };
        vars.insert("attack_skill", f64::from(skill));
        vars.insert("attack_value", f64::from(attack));
        element = item
            .and_then(|item| item.bond)
            .unwrap_or(DamageElement::Physical);
        if name == "sweeping_takedown" {
            let spec = required(parameters, "skill_bonus")?;
            let delta = f64::from(skill) - integer(spec, "delta_from")? as f64;
            let steps = required(spec, "steps")?
                .as_array()
                .ok_or(ActorPlanError::InvalidParameters("steps"))?;
            let mut coefficient = text(spec, "otherwise")?
                .parse::<f64>()
                .map_err(|_| ActorPlanError::InvalidParameters("otherwise"))?;
            for step in steps {
                if i64::from(skill) > integer(step, "above")? {
                    coefficient = text(step, "coefficient")?
                        .parse::<f64>()
                        .map_err(|_| ActorPlanError::InvalidParameters("coefficient"))?;
                    break;
                }
            }
            vars.insert("sweeping_skill_bonus", delta * delta * coefficient);
        }
    }
    let mut inner = expression_bounds(required(parameters, "formula")?, &vars)?;
    let area_spec = required(parameters, "area")?;
    let area = if name == "shield_bash" {
        AttackArea::Target
    } else if name == "flurry_of_blows" && facts.flurry_enlarged {
        matrix(required(parameters, "wheel_enlarged_area")?, "rows", true)?
    } else {
        matrix(area_spec, "rows", !shield)?
    };
    let next_auto_attack = if shield {
        let spec = required(parameters, "next_auto_attack_reduction")?;
        let duration = integer(spec, "duration_ms")?;
        if duration <= 0 {
            return Err(ActorPlanError::InvalidParameters("duration_ms"));
        }
        Some(NextAutoAttackEffect {
            duration_ms: duration as u64,
            reduction_percent: percent(spec, "percent")?,
            wheel_bonus_percent: if facts.shield_slam_grade >= 2 {
                percent(spec, "wheel_grade_2_additional_percent")?
            } else {
                0
            },
            affects_players: flag(spec, "affects_players")?,
            refresh_existing: text(spec, "replacement")? == "refresh_same_condition",
        })
    } else {
        None
    };
    let spender = name == "sweeping_takedown";
    if spender {
        let harmony = facts
            .harmony
            .ok_or(ActorPlanError::MissingOwnedFact("harmony_snapshot"))?;
        // Retain the existing integer-bound owner application, including its
        // quantization. This is not the unaccepted BP-only Harmony proposal.
        inner.minimum = harmony.apply(inner.minimum);
        inner.maximum = harmony.apply(inner.maximum);
    }
    let mut components = vec![AttackComponentPlan {
        area,
        bounds: inner,
        draw_index: 0,
        next_auto_attack,
    }];
    if spender {
        let outer = required(parameters, "outer_area")?;
        let factor = text(outer, "factor")?
            .parse::<f64>()
            .map_err(|_| ActorPlanError::InvalidParameters("factor"))?;
        if !factor.is_finite() || !(0.0..=1.0).contains(&factor) {
            return Err(ActorPlanError::InvalidBounds);
        }
        components.push(AttackComponentPlan {
            area: matrix(outer, "rows", true)?,
            bounds: bounds(inner.minimum as f64 * factor, inner.maximum as f64 * factor)?,
            draw_index: 1,
            next_auto_attack: None,
        });
    }
    Ok(EquipmentAttackPlan {
        components,
        element,
        blocked_by_armor: shield || name == "flurry_of_blows",
        harmony_role: if spender {
            NativeHarmonyRole::Spender
        } else if shield {
            NativeHarmonyRole::None
        } else {
            NativeHarmonyRole::Builder
        },
        harmony_applied_to_bounds: spender,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartyMemberFacts {
    pub(crate) id: u64,
    pub(crate) party_id: Option<u64>,
    pub(crate) vocation: Vocation,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
    pub(crate) alive: bool,
    pub(crate) removed: bool,
    pub(crate) health: u64,
}

fn visible(
    viewer: &PartyMemberFacts,
    target: &PartyMemberFacts,
    window: &Value,
) -> Result<bool, ActorPlanError> {
    let dx = i64::from(target.x) - i64::from(viewer.x);
    let dy = i64::from(target.y) - i64::from(viewer.y);
    Ok(viewer.id != target.id
        && target.alive
        && !target.removed
        && viewer.party_id.is_some()
        && viewer.party_id == target.party_id
        && viewer.floor == target.floor
        && (integer(window, "dx_min")?..=integer(window, "dx_max")?).contains(&dx)
        && (integer(window, "dy_min")?..=integer(window, "dy_max")?).contains(&dy))
}

pub(crate) fn focus_healing_target(
    parameters: &Value,
    caster: PartyMemberFacts,
    members: &[PartyMemberFacts],
) -> Result<u64, ActorPlanError> {
    let window = required(required(parameters, "harmony_gain_healing")?, "visibility")?;
    let mut target = caster;
    // Caller supplies party order as owned input. Equal HP retains the earlier
    // choice, as in the source; percentage deficits are not substituted.
    for member in members {
        if visible(&caster, member, window)? && member.health < target.health {
            target = *member;
        }
    }
    Ok(target.id)
}

fn base_vocation(vocation: Vocation) -> Vocation {
    match vocation {
        Vocation::EliteKnight => Vocation::Knight,
        Vocation::RoyalPaladin => Vocation::Paladin,
        Vocation::MasterSorcerer => Vocation::Sorcerer,
        Vocation::ElderDruid => Vocation::Druid,
        Vocation::ExaltedMonk => Vocation::Monk,
        value => value,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PartyBonusKind {
    DamageTaken,
    AutoAttackDamage,
    SpellAndRuneDamage,
    SpellAndRuneHealing,
}

impl PartyBonusKind {
    fn read(key: &str) -> Result<Self, ActorPlanError> {
        match key {
            "damage_taken_percent" => Ok(Self::DamageTaken),
            "auto_attack_damage_percent" => Ok(Self::AutoAttackDamage),
            "spell_and_rune_damage_percent" => Ok(Self::SpellAndRuneDamage),
            "spell_and_rune_healing_percent" => Ok(Self::SpellAndRuneHealing),
            _ => Err(ActorPlanError::InvalidParameters("party_bonus_kind")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyBonusPlan {
    pub(crate) target: u64,
    pub(crate) vocation: Vocation,
    pub(crate) kind: PartyBonusKind,
    pub(crate) percent: i32,
    /// A consumer combines duplicate monk sources by this key rather than summing.
    pub(crate) stacking_key: String,
}

pub(crate) fn plan_party_bonuses(
    parameters: &Value,
    caster: PartyMemberFacts,
    members: &[PartyMemberFacts],
    active_virtue: bool,
    serene: bool,
) -> Result<Vec<PartyBonusPlan>, ActorPlanError> {
    let spec = required(parameters, "party_bonus")?;
    if !active_virtue || caster.party_id.is_none() {
        return Ok(Vec::new());
    }
    let window = required(spec, "visibility")?;
    let mut present = BTreeSet::new();
    let mut visible_members = BTreeMap::new();
    for member in members {
        if visible(&caster, member, window)? {
            present.insert(base_vocation(member.vocation));
            visible_members.insert(member.id, *member);
        }
    }
    let mut result = Vec::new();
    for bonus in required(spec, "bonuses")?
        .as_array()
        .ok_or(ActorPlanError::InvalidParameters("bonuses"))?
    {
        let vocation = Vocation::from_key(text(bonus, "vocation")?)
            .ok_or(ActorPlanError::InvalidParameters("vocation"))?;
        let kind = text(bonus, "kind")?;
        let value = percent(bonus, "percent")?;
        let mut targets: Vec<u64> = visible_members
            .values()
            .filter(|m| base_vocation(m.vocation) == vocation)
            .map(|m| m.id)
            .collect();
        if serene && present.contains(&vocation) {
            targets.push(caster.id);
        }
        for target in targets {
            result.push(PartyBonusPlan {
                target,
                vocation,
                kind: PartyBonusKind::read(kind)?,
                percent: value,
                stacking_key: format!("monk_party:{target}:{kind}"),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    // The export freezes complete qualified profiles; numerical goldens below
    // are stated independently of the profile's expression representation.
    const PROFILE_PARAMETERS: &str = include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    );

    fn profile(name: &str) -> Value {
        let export: Value = serde_json::from_str(PROFILE_PARAMETERS).expect("closed profiles");
        export["profiles"]
            .as_array()
            .expect("profile array")
            .iter()
            .find(|p| {
                p["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    && p["carrier"].as_str() == Some("instant")
            })
            .expect("qualified instant profile")["execution"]["native_behavior"]["parameters"]
            .clone()
    }

    fn at(ms: u64) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(ms * 1000)
    }

    #[test]
    fn stance_toggle_retains_d145_and_never_mutates_input() {
        let p = json!({"slot":"standard","stance":"protector","eligible_vocations":["elite_knight"],
            "toggle_same_stance_off":true,"replace_existing":true,"keep_on_death":true,
            "persist_across_sessions":true,"toggle_off_costs_mana":true,"toggle_off_starts_cooldowns":true,
            "modifiers":[{"kind":"skill_percent_of_final","skills":["shielding"],"percent":30},
                {"kind":"damage_taken_percent","percent":-15,"exclude_healing":true},
                {"kind":"damage_dealt_percent","percent":-15,"exclude_healing":true}]});
        let before = p.clone();
        let plan =
            plan_stance(&p, Some(StandardStance::BloodRage), Vocation::EliteKnight).expect("plan");
        assert_eq!(plan.after, Some(StandardStance::Protector));
        assert_eq!(
            stance_skill(&plan, ActorSkill::Shielding, 100, 120, false),
            Ok(156)
        );
        assert_eq!(
            stance_damage(&plan, -101, true, false, false, HealingOrigin::Other),
            Ok(-85)
        );
        assert_eq!(
            stance_damage(&plan, 100, false, true, false, HealingOrigin::Other),
            Ok(100)
        );
        let off = plan_stance(&p, plan.after, Vocation::EliteKnight).expect("off");
        assert_eq!(off.after, None);
        assert!(off.modifiers.is_empty());
        assert!(
            off.keep_on_death && off.persist_across_sessions && off.debit_mana_and_start_cooldowns
        );
        assert_eq!(p, before);
        assert_eq!(
            plan_stance(&p, None, Vocation::Knight),
            Err(ActorPlanError::IneligibleVocation)
        );
    }

    #[test]
    fn justice_uses_base_fist_and_serene_live_value() {
        let plan = StancePlan {
            before: None,
            after: Some(StandardStance::Justice),
            modifiers: vec![StanceModifier::BaseFist {
                percent: 8,
                serene_percent: 16,
            }],
            keep_on_death: true,
            persist_across_sessions: true,
            debit_mana_and_start_cooldowns: true,
        };
        assert_eq!(
            stance_skill(&plan, ActorSkill::Fist, 101, 121, false),
            Ok(129)
        );
        assert_eq!(
            stance_skill(&plan, ActorSkill::Fist, 101, 121, true),
            Ok(137)
        );
        assert_eq!(
            stance_skill(&plan, ActorSkill::Sword, 101, 121, true),
            Ok(121)
        );
    }

    #[test]
    fn shield_debuff_is_one_autoattack_and_expires_at_boundary() {
        let p = json!({"next_auto_attack_reduction":{"percent":50,"wheel_grade_2_additional_percent":25}});
        let f = ShieldDebuffFacts {
            expires_at: at(10000),
            now: at(9999),
            origin: AttackOrigin::Fist,
            primary: -101,
            secondary: -31,
            wheel_grade: 2,
        };
        let plan = shield_debuff(&p, f).expect("debuff");
        assert_eq!(
            (plan.primary, plan.secondary, plan.consume),
            (-25, -7, true)
        );
        let spell = shield_debuff(
            &p,
            ShieldDebuffFacts {
                origin: AttackOrigin::Spell,
                ..f
            },
        )
        .expect("spell");
        assert_eq!(
            (spell.primary, spell.secondary, spell.consume),
            (-101, -31, false)
        );
        let expired = shield_debuff(
            &p,
            ShieldDebuffFacts {
                now: at(10000),
                ..f
            },
        )
        .expect("expiry");
        assert!(expired.expired);
        assert!(!expired.consume);
    }

    #[test]
    fn bounded_expression_inputs_and_negative_ranges_fail_closed() {
        let vars = BTreeMap::from([("level", 200.0), ("gained_charges", 2.0)]);
        assert_eq!(
            evaluate(
                &json!({"fn":"level_base_damage_healing","args":[{"var":"level"}]}),
                &vars,
                1
            ),
            Ok(40.0)
        );
        assert_eq!(
            evaluate(&json!({"var":"unknown"}), &vars, 1),
            Err(ActorPlanError::MissingOwnedFact("expression_input"))
        );
        assert_eq!(bounds(-1.0, 10.0), Err(ActorPlanError::InvalidBounds));
        assert_eq!(bounds(11.0, 10.0), Err(ActorPlanError::InvalidBounds));
        assert_eq!(
            evaluate(
                &json!({"op":"div","args":[{"const":"1"},{"const":"0"}]}),
                &vars,
                1
            ),
            Err(ActorPlanError::InvalidBounds)
        );
    }

    #[test]
    fn qualified_focus_profiles_fill_once_and_boost_only_after_the_healing_draw() {
        let facts = FocusFacts {
            harmony: 3,
            serene: false,
            forced_until: None,
            now: at(100),
            level: 200,
            sustain_active: true,
        };
        let harmony = plan_focus(&profile("focus harmony"), facts).expect("focus harmony");
        assert_eq!(
            (
                harmony.harmony_after,
                harmony.gained_charges,
                harmony.serene_after
            ),
            (5, 2, false)
        );
        let heal = harmony.healing.expect("positive gain heal");
        assert_eq!(
            heal.bounds,
            MagnitudeBounds {
                minimum: 88,
                maximum: 102
            }
        );
        assert_eq!(heal.finish_draw(89), Ok(120));
        assert_eq!(heal.finish_draw(87), Err(ActorPlanError::InvalidBounds));
        assert!(!harmony.individual_reset.clears(true));
        let serenity = plan_focus(&profile("focus serenity"), facts).expect("focus serenity");
        assert!(serenity.serene_after);
        assert_eq!(serenity.forced_until, Some(at(7100)));
        assert_eq!(
            serenity.healing.expect("serene gain heal").finish_draw(89),
            Ok(151)
        );
        assert!(serenity.individual_reset.clears(true));
        assert!(!serenity.individual_reset.clears(false));
        assert!(serenity.group_without_spell_reset.clears(false));
        assert!(!serenity.group_with_spell_reset.clears(false));
        let full = plan_focus(
            &profile("focus serenity"),
            FocusFacts {
                harmony: 5,
                ..facts
            },
        )
        .expect("already full");
        assert!(full.healing.is_none());
        assert_eq!(full.gained_charges, 0);
        assert_eq!(
            plan_focus(
                &profile("focus harmony"),
                FocusFacts {
                    harmony: 6,
                    ..facts
                }
            ),
            Err(ActorPlanError::InvalidHarmony)
        );
    }

    #[test]
    fn all_six_stance_profiles_emit_typed_modifiers_and_prune_on_vocation_change() {
        for (name, vocation) in [
            ("blood rage", Vocation::EliteKnight),
            ("protector", Vocation::EliteKnight),
            ("sharpshooter", Vocation::RoyalPaladin),
            ("virtue of harmony", Vocation::Monk),
            ("virtue of justice", Vocation::Monk),
            ("virtue of sustain", Vocation::ExaltedMonk),
        ] {
            let parameters = profile(name);
            let plan = plan_stance(&parameters, None, vocation).expect("qualified stance");
            assert!(!plan.modifiers.is_empty());
            assert!(plan.keep_on_death && plan.persist_across_sessions);
            assert_eq!(
                retain_stance(&parameters, plan.after, vocation),
                Ok(plan.after)
            );
            assert_eq!(
                retain_stance(&parameters, plan.after, Vocation::Druid),
                Ok(None)
            );
            assert_eq!(
                plan_stance(&parameters, plan.after, vocation)
                    .expect("toggle off")
                    .after,
                None
            );
        }
        let rage = plan_stance(&profile("blood rage"), None, Vocation::EliteKnight).expect("rage");
        assert_eq!(
            stance_skill(&rage, ActorSkill::Axe, 100, 120, false),
            Ok(150)
        );
        assert_eq!(
            stance_damage(&rage, -101, true, false, false, HealingOrigin::Other),
            Ok(-116)
        );
        let sharp =
            plan_stance(&profile("sharpshooter"), None, Vocation::RoyalPaladin).expect("sharp");
        assert_eq!(
            stance_skill(&sharp, ActorSkill::Distance, 100, 120, false),
            Ok(158)
        );
        let sustain =
            plan_stance(&profile("virtue of sustain"), None, Vocation::Monk).expect("sustain");
        assert_eq!(
            stance_damage(&sustain, 89, false, true, false, HealingOrigin::MonkSpell),
            Ok(120)
        );
        assert_eq!(
            stance_damage(&sustain, 89, false, true, true, HealingOrigin::HarmonyGain),
            Ok(151)
        );
        assert_eq!(
            stance_damage(&sustain, 89, false, true, true, HealingOrigin::Other),
            Ok(89)
        );
        let harmony =
            plan_stance(&profile("virtue of harmony"), None, Vocation::Monk).expect("harmony");
        assert_eq!(
            harmony.modifiers,
            vec![StanceModifier::HarmonyBaseScale {
                percent: 50,
                serene_percent: 100,
                refund_charges: 1
            }]
        );
    }

    fn equipment_facts() -> EquipmentFacts {
        EquipmentFacts {
            level: 1000,
            left: Some(HandItem {
                kind: HandKind::Weapon(ActorSkill::Fist),
                attack: 46,
                defense: 0,
                bond: Some(DamageElement::Energy),
            }),
            right: None,
            effective_skills: BTreeMap::from([
                (ActorSkill::Fist, 140),
                (ActorSkill::Shielding, 100),
            ]),
            flurry_enlarged: false,
            shield_slam_grade: 2,
            harmony: Some(HarmonyMultiplier::ONE),
        }
    }

    #[test]
    fn equipment_profiles_keep_source_goldens_hand_precedence_and_separate_sweeping_draws() {
        let facts = equipment_facts();
        let flurry = equipment_attack(&profile("flurry of blows"), &facts).expect("flurry");
        assert_eq!(
            flurry.components[0].bounds,
            MagnitudeBounds {
                minimum: 495,
                maximum: 606
            }
        );
        assert_eq!(flurry.element, DamageElement::Energy);
        assert_eq!(flurry.harmony_role, NativeHarmonyRole::Builder);
        assert!(!flurry.harmony_applied_to_bounds);
        let sweep = equipment_attack(&profile("sweeping takedown"), &facts).expect("sweep");
        assert_eq!(
            sweep.components[0].bounds,
            MagnitudeBounds {
                minimum: 670,
                maximum: 876
            }
        );
        assert_eq!(
            sweep.components[1].bounds,
            MagnitudeBounds {
                minimum: 502,
                maximum: 657
            }
        );
        assert_eq!(
            (
                sweep.components[0].draw_index,
                sweep.components[1].draw_index
            ),
            (0, 1)
        );
        assert!(sweep.harmony_applied_to_bounds);
        assert!(!sweep.blocked_by_armor);
        let charged = EquipmentFacts {
            harmony: HarmonyMultiplier::new(1000, 3, false, false),
            ..facts.clone()
        };
        let sweep =
            equipment_attack(&profile("sweeping takedown"), &charged).expect("whole-bound harmony");
        assert_eq!(
            sweep.components[0].bounds,
            MagnitudeBounds {
                minimum: 991,
                maximum: 1296
            }
        );
        assert_eq!(
            sweep.components[1].bounds,
            MagnitudeBounds {
                minimum: 743,
                maximum: 972
            }
        );
        let missing = EquipmentFacts {
            harmony: None,
            ..facts.clone()
        };
        assert_eq!(
            equipment_attack(&profile("sweeping takedown"), &missing),
            Err(ActorPlanError::MissingOwnedFact("harmony_snapshot"))
        );
        let shield = EquipmentFacts {
            level: 1,
            right: Some(HandItem {
                kind: HandKind::Shield,
                attack: 0,
                defense: 20,
                bond: None,
            }),
            ..facts
        };
        let bash = equipment_attack(&profile("shield bash"), &shield).expect("right shield");
        assert_eq!(
            bash.components[0].bounds,
            MagnitudeBounds {
                minimum: 99,
                maximum: 121
            }
        );
        assert_eq!(bash.components[0].area, AttackArea::Target);
        assert_eq!(
            bash.components[0]
                .next_auto_attack
                .expect("bash effect")
                .wheel_bonus_percent,
            0
        );
        let slam = equipment_attack(&profile("shield slam"), &shield).expect("slam");
        assert_eq!(
            slam.components[0].bounds,
            MagnitudeBounds {
                minimum: 93,
                maximum: 114
            }
        );
        assert_eq!(
            slam.components[0]
                .next_auto_attack
                .expect("slam effect")
                .wheel_bonus_percent,
            25
        );
        let no_shield = EquipmentFacts {
            right: None,
            ..shield
        };
        assert!(matches!(
            equipment_attack(&profile("shield bash"), &no_shield),
            Err(ActorPlanError::MissingShield { .. })
        ));
    }

    #[test]
    fn lowest_absolute_hp_and_party_union_obey_exact_visibility_and_nonstack_keys() {
        let caster = PartyMemberFacts {
            id: 1,
            party_id: Some(10),
            vocation: Vocation::Monk,
            x: 0,
            y: 0,
            floor: 7,
            alive: true,
            removed: false,
            health: 10000,
        };
        let knight = PartyMemberFacts {
            id: 2,
            vocation: Vocation::EliteKnight,
            x: 9,
            y: 7,
            health: 80,
            ..caster
        };
        let paladin = PartyMemberFacts {
            id: 3,
            vocation: Vocation::RoyalPaladin,
            x: -8,
            y: -6,
            health: 80,
            ..caster
        };
        let druid = PartyMemberFacts {
            id: 4,
            vocation: Vocation::Druid,
            health: 120,
            ..caster
        };
        let hidden = PartyMemberFacts {
            id: 5,
            vocation: Vocation::Sorcerer,
            x: 10,
            health: 1,
            ..caster
        };
        let dead = PartyMemberFacts {
            id: 6,
            alive: false,
            health: 0,
            ..caster
        };
        let members = [knight, paladin, druid, hidden, dead];
        assert_eq!(
            focus_healing_target(&profile("focus harmony"), caster, &members),
            Ok(2)
        );
        let p = profile("virtue of harmony");
        let normal = plan_party_bonuses(&p, caster, &members, true, false).expect("normal party");
        assert_eq!(normal.len(), 3);
        let serene =
            plan_party_bonuses(&p, caster, &members, true, true).expect("serene party union");
        assert_eq!(serene.len(), 6);
        assert_eq!(serene.iter().filter(|b| b.target == caster.id).count(), 3);
        assert!(
            serene
                .iter()
                .any(|b| b.target == 2 && b.kind == PartyBonusKind::DamageTaken && b.percent == -4)
        );
        assert!(serene.iter().any(|b| b.target == 3
            && b.kind == PartyBonusKind::AutoAttackDamage
            && b.percent == 8));
        assert!(serene.iter().any(|b| b.target == 4
            && b.kind == PartyBonusKind::SpellAndRuneHealing
            && b.percent == 16));
        assert_eq!(
            plan_party_bonuses(&p, caster, &members, false, true),
            Ok(Vec::new())
        );
        assert_eq!(normal[0].stacking_key, "monk_party:2:damage_taken_percent");
    }
}
