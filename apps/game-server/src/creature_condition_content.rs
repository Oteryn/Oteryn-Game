//! Pure, fail-closed lowering of a bounded Content condition subset into COND-1 definitions.
//! This prepares definitions only. The current Channel owner must independently resolve the
//! target, apply through its accepted commit boundary, and consume due ticks. No HP, movement,
//! presentation, timer, actor ownership or production activation is performed here.
use crate::content::{
    ProjectV2AttributeModifierMode, ProjectV2ConditionLifetime, ProjectV2DamageOverTime,
    ProjectV2DefinitionRef, ProjectV2ExactRatio, ProjectV2Family, ProjectV2FirstTick,
    ProjectV2FormulaAuthoring, ProjectV2InlineEffect, ProjectV2InlineEffectOperation,
};
use crate::foundation::{
    AttributeModifier, AttributeModifiers, ConditionDefinition, ConditionValues, DamageSchedule,
    DamageSegment, DotElement, ExactSpeedRatio, RationalSpeedRange, SpeedRange,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionContentError {
    NotCondition,
    UnsupportedCondition,
    UnsupportedParameters,
    UnsupportedDamageSchedule,
    MissingOrMismatchedFormula,
    UnsupportedFormula,
    InexactSpeedCoefficient,
    NumericOverflow,
    InvalidDefinition,
}

/// The optional formula is an exactly resolved content reference, never a fallback formula.
/// `revision` is supplied by the versioned content loader; it confers no runtime authority.
pub(crate) fn lower_condition_definition(
    effect: &ProjectV2InlineEffect,
    revision: u32,
    resolved_formula: Option<(&ProjectV2DefinitionRef, &ProjectV2FormulaAuthoring)>,
) -> Result<ConditionDefinition, ConditionContentError> {
    use ConditionContentError as E;
    let ProjectV2InlineEffectOperation::Condition {
        duration_ms,
        condition,
    } = &effect.operation
    else {
        return Err(E::NotCondition);
    };
    if revision == 0 {
        return Err(E::InvalidDefinition);
    }
    if (condition.condition_type != "attributes" && !condition.attribute_modifiers.is_empty())
        || condition.light.is_some()
        || (condition.regeneration.is_some() && condition.condition_type != "regeneration")
        || condition.buff_spell.is_some()
    {
        return Err(E::UnsupportedParameters);
    }
    let values = match condition.condition_type.as_str() {
        "regeneration" => {
            if condition.lifetime != ProjectV2ConditionLifetime::FixedDuration
                || condition.damage_over_time.is_some()
                || condition.speed_formula.is_some()
                || !condition.attribute_modifiers.is_empty()
            {
                return Err(E::UnsupportedParameters);
            }
            let payload = condition.regeneration.ok_or(E::UnsupportedParameters)?;
            let mut interval = 0u64;
            for (gain, period) in [
                (payload.health_gain, payload.health_interval_ms),
                (payload.mana_gain, payload.mana_interval_ms),
            ] {
                match (gain, period) {
                    (None, None) => {}
                    (Some(gain), Some(period)) if gain > 0 && period > 0 => {
                        if interval == 0 {
                            interval = period
                        } else {
                            let mut rhs = period;
                            while rhs != 0 {
                                let remainder = interval % rhs;
                                interval = rhs;
                                rhs = remainder;
                            }
                        }
                    }
                    _ => return Err(E::UnsupportedParameters),
                }
            }
            let interval_ms = u32::try_from(interval).map_err(|_| E::NumericOverflow)?;
            let duration_ms = u32::try_from(duration_ms.ok_or(E::UnsupportedParameters)?)
                .map_err(|_| E::NumericOverflow)?;
            if interval_ms == 0 || duration_ms == 0 {
                return Err(E::UnsupportedParameters);
            }
            // One gcd cursor produces every distinct HP/MP due boundary; the source registry
            // emits each retained gain only at its own exact elapsed-time interval.
            ConditionValues::Recovery {
                duration_ms,
                interval_ms,
            }
        }

        "invisible" | "drunk" | "rooted" | "feared" => {
            if condition.lifetime != ProjectV2ConditionLifetime::FixedDuration
                || condition.damage_over_time.is_some()
                || condition.speed_formula.is_some()
                || !condition.attribute_modifiers.is_empty()
            {
                return Err(E::UnsupportedParameters);
            }
            let duration_ms = u32::try_from(duration_ms.ok_or(E::InvalidDefinition)?)
                .map_err(|_| E::NumericOverflow)?;
            let kind = match condition.condition_type.as_str() {
                "invisible" => crate::foundation::StatusKind::Invisible,
                "drunk" => crate::foundation::StatusKind::Drunk,
                "rooted" => crate::foundation::StatusKind::Rooted,
                _ => crate::foundation::StatusKind::Feared,
            };
            ConditionValues::TimedStatus { kind, duration_ms }
        }
        "attributes" => {
            if condition.lifetime != ProjectV2ConditionLifetime::FixedDuration
                || condition.damage_over_time.is_some()
                || condition.speed_formula.is_some()
                || resolved_formula.is_some()
            {
                return Err(E::UnsupportedParameters);
            }
            let mut modifiers = AttributeModifiers::default();
            for source in &condition.attribute_modifiers {
                let value = i32::try_from(source.value).map_err(|_| E::NumericOverflow)?;
                let modifier = match source.mode {
                    ProjectV2AttributeModifierMode::PercentOfBase => {
                        AttributeModifier::PercentOfBase(value)
                    }
                    ProjectV2AttributeModifierMode::Add => AttributeModifier::Add(value),
                };
                let target = match source.attribute.as_str() {
                    "skill_melee" => &mut modifiers.melee,
                    "skill_distance" => &mut modifiers.distance,
                    "stat_magicpoints" => &mut modifiers.magic_points,
                    _ => return Err(E::UnsupportedParameters),
                };
                if target.replace(modifier).is_some() {
                    return Err(E::UnsupportedParameters);
                }
            }
            ConditionValues::SourceAttributes {
                modifiers,
                duration_ms: u32::try_from(duration_ms.ok_or(E::InvalidDefinition)?)
                    .map_err(|_| E::NumericOverflow)?,
            }
        }
        "haste" | "paralyze" => {
            if condition.lifetime != ProjectV2ConditionLifetime::FixedDuration
                || condition.damage_over_time.is_some()
            {
                return Err(E::UnsupportedParameters);
            }
            let (reference, formula) = resolved_formula.ok_or(E::MissingOrMismatchedFormula)?;
            if reference.family != ProjectV2Family::Formula
                || condition.speed_formula.as_ref() != Some(reference)
            {
                return Err(E::MissingOrMismatchedFormula);
            }
            let ProjectV2FormulaAuthoring::SpeedModifier {
                minimum_multiplier,
                minimum_offset,
                maximum_multiplier,
                maximum_offset,
            } = formula
            else {
                return Err(E::UnsupportedFormula);
            };
            let duration_ms = u32::try_from(duration_ms.ok_or(E::InvalidDefinition)?)
                .map_err(|_| E::NumericOverflow)?;
            let b_min = i32::try_from(*minimum_offset).map_err(|_| E::NumericOverflow)?;
            let b_max = i32::try_from(*maximum_offset).map_err(|_| E::NumericOverflow)?;
            match (
                exact_thousandths(*minimum_multiplier),
                exact_thousandths(*maximum_multiplier),
            ) {
                (Ok(a_min), Ok(a_max)) => ConditionValues::Speed {
                    paralysis: condition.condition_type == "paralyze",
                    range: SpeedRange {
                        a_min,
                        b_min,
                        a_max,
                        b_max,
                    },
                    duration_ms,
                },
                (Err(E::InexactSpeedCoefficient), _) | (_, Err(E::InexactSpeedCoefficient)) => {
                    if minimum_multiplier.denominator == 0 || maximum_multiplier.denominator == 0 {
                        return Err(E::InvalidDefinition);
                    }
                    ConditionValues::RationalSpeed {
                        paralysis: condition.condition_type == "paralyze",
                        range: RationalSpeedRange {
                            a_min: ExactSpeedRatio {
                                numerator: minimum_multiplier.numerator,
                                denominator: minimum_multiplier.denominator,
                            },
                            b_min,
                            a_max: ExactSpeedRatio {
                                numerator: maximum_multiplier.numerator,
                                denominator: maximum_multiplier.denominator,
                            },
                            b_max,
                        },
                        duration_ms,
                    }
                }
                (Err(error), _) | (_, Err(error)) => return Err(error),
            }
        }
        name => {
            let element = match name {
                "poison" => DotElement::Poison,
                "fire" => DotElement::Fire,
                "energy" => DotElement::Energy,
                "bleeding" => DotElement::Bleeding,
                "drown" => DotElement::Drown,
                "freezing" => DotElement::Freezing,
                "dazzled" => DotElement::Dazzled,
                "cursed" => DotElement::Cursed,
                _ => return Err(E::UnsupportedCondition),
            };
            if condition.lifetime != ProjectV2ConditionLifetime::DamageSchedule
                || duration_ms.is_some()
                || condition.speed_formula.is_some()
                || resolved_formula.is_some()
            {
                return Err(E::UnsupportedParameters);
            }
            let delayed =
                |first_tick: &ProjectV2FirstTick| *first_tick == ProjectV2FirstTick::AfterInterval;
            let number = |n: u64| u32::try_from(n).map_err(|_| E::NumericOverflow);
            let schedule = match &condition.damage_over_time {
                Some(ProjectV2DamageOverTime::Fixed { first_tick, ticks }) => {
                    DamageSchedule::Fixed {
                        delayed: delayed(first_tick),
                        segments: ticks
                            .iter()
                            .map(|t| {
                                Ok(DamageSegment {
                                    count: t.count,
                                    amount: number(t.amount)?,
                                    interval_ms: number(t.interval_ms)?,
                                })
                            })
                            .collect::<Result<Vec<_>, E>>()?,
                    }
                }
                Some(ProjectV2DamageOverTime::Decreasing {
                    first_tick,
                    total_minimum,
                    total_maximum,
                    tick_interval_ms,
                    initial_tick_amount,
                }) => DamageSchedule::Decreasing {
                    delayed: delayed(first_tick),
                    minimum: number(*total_minimum)?,
                    maximum: number(*total_maximum)?,
                    initial: initial_tick_amount.map(number).transpose()?,
                    interval_ms: number(*tick_interval_ms)?,
                },
                Some(ProjectV2DamageOverTime::Geometric {
                    first_tick,
                    base_minimum,
                    base_maximum,
                    factor,
                    tick_counts,
                    tick_interval_ms,
                }) => DamageSchedule::Geometric {
                    delayed: delayed(first_tick),
                    minimum: number(*base_minimum)?,
                    maximum: number(*base_maximum)?,
                    numerator: u32::try_from(factor.numerator).map_err(|_| E::NumericOverflow)?,
                    denominator: number(factor.denominator)?,
                    counts: tick_counts.clone(),
                    interval_ms: number(*tick_interval_ms)?,
                },
                None => return Err(E::UnsupportedDamageSchedule),
            };
            return ConditionDefinition::new_damage_schedule(
                &effect.key,
                revision,
                element,
                schedule,
            )
            .ok_or(E::InvalidDefinition);
        }
    };
    ConditionDefinition::new(&effect.key, revision, values).ok_or(E::InvalidDefinition)
}

fn exact_thousandths(ratio: ProjectV2ExactRatio) -> Result<i32, ConditionContentError> {
    use ConditionContentError as E;
    if ratio.denominator == 0 {
        return Err(E::InvalidDefinition);
    }
    let scaled = i128::from(ratio.numerator) * 1_000;
    let denominator = i128::from(ratio.denominator);
    if scaled % denominator != 0 {
        return Err(E::InexactSpeedCoefficient);
    }
    i32::try_from(scaled / denominator).map_err(|_| E::NumericOverflow)
}

#[cfg(test)]
#[path = "condition_content_tests.rs"]
mod tests;
