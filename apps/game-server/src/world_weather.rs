//! WEATHER-1: pure visual-weather selection (CP decisions D1023–D1028).
//!
//! These arithmetic primitives do not resolve a World binding or authorize publication. The
//! World-config owner must supply the retained private World root and the accepted semantic
//! occurrence for the World, clock identity, policy revision and absolute slot. In particular,
//! no Channel, process generation, wall-clock read, default epoch or fixture seed is introduced
//! here. Canonical occurrence derivation and the read-only binding belong to WEATHER-SEAM-1;
//! the server must not offer weather before that seam and the reviewed wire are integrated.
//! D1111 assigns the rain/fog/snow map and selector data below to WEATHER-1.

use oteryn_simulation_determinism::{
    DecisionError, DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

/// D1023: 360 Tibian minutes, each 2,500 real milliseconds.
pub(crate) const WEATHER_SLOT_MS: u64 = 900_000;
const SELECTOR_ENTRIES: usize = 16;
const WEATHER_PURPOSE: &str = "weather_visual_v1";

/// Arithmetic only: the caller must qualify both facts through the owning clock/config seam.
/// The absolute elapsed value is never reduced modulo a Tibian day. Before-epoch facts fail
/// closed; this function neither reads a clock nor substitutes an epoch.
pub(crate) fn absolute_weather_slot(epoch_utc_ms: u64, normalized_utc_ms: u64) -> Option<u64> {
    normalized_utc_ms
        .checked_sub(epoch_utc_ms)
        .map(|elapsed| elapsed / WEATHER_SLOT_MS)
}

/// An explicitly admitted policy-local preset identity; it is not a registered wire value.
/// WEATHER-1 maps shipped identities to rain, fog and snow below; client assets remain a
/// separate WEATHER-CLIENT-1 integration requirement.
/// There is deliberately no `Default` or "clear" variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WeatherPreset(u8);

impl WeatherPreset {
    pub(crate) fn index(self) -> u8 {
        self.0
    }
}

/// Visual identities only: none is a gameplay modifier or a client asset path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeatherVisual {
    Rain,
    Fog,
    Snow,
}

/// D1111: authored in this owned module, without a new content file or a clear preset.
pub(crate) const WEATHER_VISUAL_PRESETS: [(WeatherPreset, WeatherVisual); 3] = [
    (WeatherPreset(1), WeatherVisual::Rain),
    (WeatherPreset(2), WeatherVisual::Fog),
    (WeatherPreset(3), WeatherVisual::Snow),
];

/// Explicit authored weights: rain 6/16, fog 5/16, snow 5/16. A World must bind this
/// policy and its retained revision explicitly; this data never substitutes for a missing
/// World binding. No implicit default, clock, activation rule or asset admission is supplied.
pub(crate) const WEATHER_VISUAL_POLICY: WeatherPolicy = WeatherPolicy {
    selector: [
        WeatherPreset(1),
        WeatherPreset(1),
        WeatherPreset(1),
        WeatherPreset(1),
        WeatherPreset(1),
        WeatherPreset(1),
        WeatherPreset(2),
        WeatherPreset(2),
        WeatherPreset(2),
        WeatherPreset(2),
        WeatherPreset(2),
        WeatherPreset(3),
        WeatherPreset(3),
        WeatherPreset(3),
        WeatherPreset(3),
        WeatherPreset(3),
    ],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeatherPolicyError {
    PresetCount,
    InvalidPreset,
    DuplicatePreset,
    SelectorLength,
    UnadmittedPreset,
}

/// D1024: exactly sixteen selector entries. Repetitions express weights, so sampling takes
/// exactly one deterministic draw without a rejection loop or a history-dependent cursor.
/// Construction validates an explicit policy; it supplies no default presets or weights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WeatherPolicy {
    selector: [WeatherPreset; SELECTOR_ENTRIES],
}

impl WeatherPolicy {
    pub(crate) fn new(admitted: &[u8], selector: &[u8]) -> Result<Self, WeatherPolicyError> {
        if admitted.is_empty() || admitted.len() > SELECTOR_ENTRIES {
            return Err(WeatherPolicyError::PresetCount);
        }
        let mut admitted_mask = 0_u16;
        for &index in admitted {
            if !(1..=16).contains(&index) {
                return Err(WeatherPolicyError::InvalidPreset);
            }
            let bit = 1_u16 << (index - 1);
            if admitted_mask & bit != 0 {
                return Err(WeatherPolicyError::DuplicatePreset);
            }
            admitted_mask |= bit;
        }
        let entries: [u8; SELECTOR_ENTRIES] = selector
            .try_into()
            .map_err(|_| WeatherPolicyError::SelectorLength)?;
        for index in entries {
            if !(1..=16).contains(&index) {
                return Err(WeatherPolicyError::InvalidPreset);
            }
            if admitted_mask & (1_u16 << (index - 1)) == 0 {
                return Err(WeatherPolicyError::UnadmittedPreset);
            }
        }
        Ok(Self {
            selector: entries.map(WeatherPreset),
        })
    }

    /// The high four bits partition the entire u64 draw space into sixteen equal buckets.
    fn preset_for_draw(&self, draw: u64) -> WeatherPreset {
        self.selector[usize::from(draw.to_be_bytes()[0] >> 4)]
    }

    /// Pure selection, not binding admission. `occurrence` must already bind the exact World,
    /// clock identity, policy revision and absolute slot under the owner's canonical contract.
    /// A retry/restart/Channel move supplies the same root and occurrence, never a new seed.
    /// No simulation, World light, transport, timer, storage or mutable RNG state is consumed.
    pub(crate) fn select(
        &self,
        world_root: &GameplayDecisionRoot,
        occurrence: DecisionOccurrenceId,
    ) -> Result<WeatherPreset, DecisionError> {
        deterministic_decision_u64(world_root, occurrence, WEATHER_PURPOSE, 0)
            .map(|draw| self.preset_for_draw(draw))
    }
}

#[cfg(test)]
#[path = "world_weather_tests.rs"]
mod tests;
