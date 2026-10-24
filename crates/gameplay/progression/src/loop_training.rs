//! LOOP-TRAIN-1 pure OFFLINE0/exercise planning. Source candidates stay gated.
use oteryn_gameplay_domain::domain::character_build::{
    BuildFormula, DeathBuildState, MAX_BUILD_PROGRESS, cumulative_progress, relevel,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POOL_SECONDS: u32 = 43_200;
pub const MAX_OFFLINE_SECONDS: u64 = 21 * 24 * 60 * 60;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingRates {
    pub schema_version: u8,
    pub provisional: bool,
    pub enabled: bool,
    pub source: String,
    pub offline_divisors: [u32; 8],
    pub percent_scale: u16,
    pub exercise_gains: [u64; 8],
    pub vocations: Vec<VocationRate>,
    pub statues: BTreeMap<String, usize>,
    pub dummies: Vec<String>,
    pub weapons: BTreeMap<String, usize>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VocationRate {
    pub vocation: String,
    pub top_attack_ms: u32,
    pub mana_amount: u32,
    pub top_mana_ticks_ms: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainingUnavailable;

impl TrainingRates {
    /// Configuration is bounded and fail closed. Merely loading source candidates enables nothing.
    pub fn from_json(bytes: &[u8]) -> Result<Self, TrainingUnavailable> {
        if bytes.len() > 32_768 {
            return Err(TrainingUnavailable);
        }
        let rates: Self = serde_json::from_slice(bytes).map_err(|_| TrainingUnavailable)?;
        if rates.schema_version != 1
            || !rates.provisional
            || rates.source.is_empty()
            || rates.percent_scale != 100
            || rates.vocations.is_empty()
            || rates.vocations.len() > 32
            || rates.statues.len() > 32
            || rates.dummies.len() > 32
            || rates.weapons.len() > 64
            || rates.exercise_gains[..7]
                .iter()
                .any(|gain| *gain == 0 || *gain > 1_000_000)
            || rates.exercise_gains[7] != 0
            || rates.offline_divisors[..7]
                .iter()
                .any(|v| *v == 0 || *v > 1000)
            || rates.statues.values().any(|family| *family > 5)
            || rates.weapons.values().any(|family| *family > 6)
            || rates.vocations.iter().enumerate().any(|(i, v)| {
                v.vocation.is_empty()
                    || v.vocation.len() > 128
                    || v.top_attack_ms == 0
                    || v.mana_amount == 0
                    || v.mana_amount > 1_000_000
                    || rates.vocations[..i]
                        .iter()
                        .any(|prior| prior.vocation == v.vocation)
            })
        {
            return Err(TrainingUnavailable);
        }
        Ok(rates)
    }

    pub fn offline_gain(
        &self,
        vocation: &str,
        family: usize,
        seconds: u32,
    ) -> Result<u64, TrainingUnavailable> {
        if !self.enabled || family > 5 || seconds > POOL_SECONDS {
            return Err(TrainingUnavailable);
        }
        let rate = self
            .vocations
            .iter()
            .find(|rate| rate.vocation == vocation)
            .ok_or(TrainingUnavailable)?;
        let (numerator, denominator) = if family == 0 {
            (
                u64::from(rate.mana_amount),
                u64::from(if rate.top_mana_ticks_ms == 0 {
                    1000
                } else {
                    rate.top_mana_ticks_ms
                }),
            )
        } else {
            (
                1,
                u64::from(rate.top_attack_ms) * u64::from(self.offline_divisors[family]),
            )
        };
        Ok(u64::from(seconds) * 1000 * numerator / denominator)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfflinePlan {
    pub pool_after: u32,
    pub training_seconds: u32,
}

/// The durable owner, never the caller, supplies the qualified elapsed interval.
pub fn plan_offline(
    pool: u32,
    selected: Option<usize>,
    elapsed: u64,
) -> Result<OfflinePlan, TrainingUnavailable> {
    if pool > POOL_SECONDS || selected.is_some_and(|family| family > 5) {
        return Err(TrainingUnavailable);
    }
    let elapsed = elapsed.min(MAX_OFFLINE_SECONDS);
    if selected.is_some() && elapsed < 600 {
        return Ok(OfflinePlan {
            pool_after: pool,
            training_seconds: 0,
        });
    }
    let trained = if selected.is_some() {
        elapsed.min(u64::from(pool))
    } else {
        0
    };
    Ok(OfflinePlan {
        pool_after: u32::try_from(
            (u64::from(pool) - trained + elapsed - trained).min(u64::from(POOL_SECONDS)),
        )
        .map_err(|_| TrainingUnavailable)?,
        training_seconds: u32::try_from(trained).map_err(|_| TrainingUnavailable)?,
    })
}

/// Advance through the existing native curves; no copied skill advancement loop.
pub fn add_gain<B: DeathBuildState>(
    before: &B,
    formula: &dyn BuildFormula,
    family: usize,
    gain: u64,
) -> Result<B, TrainingUnavailable> {
    if family > 6 || gain == 0 {
        return Err(TrainingUnavailable);
    }
    let (mut magic, mut skills) = (before.magic(), before.skills());
    let slot = if family == 0 {
        &mut magic
    } else {
        &mut skills[family - 1]
    };
    let floor = if family == 0 { 0 } else { 10 };
    let required = |level| formula.required(before.vocation(), family, level);
    if required(slot.0.saturating_add(1)).is_none() {
        return Err(TrainingUnavailable);
    }
    let cumulative = cumulative_progress(slot.0, slot.1, floor, required)
        .checked_add(gain)
        .filter(|value| *value <= MAX_BUILD_PROGRESS)
        .ok_or(TrainingUnavailable)?;
    *slot = relevel(cumulative, floor, required);
    B::from_death_families(before.vocation(), magic, skills).ok_or(TrainingUnavailable)
}

#[cfg(test)]
#[path = "loop_training_tests.rs"]
mod tests;
