//! WEATHER-1: pure visual-weather selection (CP decisions D1023–D1028).
//!
//! These arithmetic primitives do not resolve a World binding or authorize publication. The
//! World-config owner must supply the retained private World root and the accepted semantic
//! occurrence for the World, clock identity, policy revision and absolute slot. In particular,
//! no Channel, process generation, wall-clock read, default epoch or fixture seed is introduced
//! here. Canonical occurrence derivation and the read-only binding belong to WEATHER-SEAM-1;
//! the server must not offer weather before that seam and the reviewed wire are integrated.
//! CP D1446: no sourced preset data exists, so this module authors none. A World offers weather
//! only when its [`WorldWeatherSource`] (retained binding plus an admitted policy) is present.

use crate::foundation::WorldId;
use crate::world_clock::RetainedWorldConfig;
use oteryn_protocol_oteryn::world_weather::WorldWeather;
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
    crate::world_clock::WorldClockSource::new(epoch_utc_ms)
        .absolute_elapsed_ms(normalized_utc_ms)
        .map(|elapsed| elapsed / WEATHER_SLOT_MS)
}

/// An explicitly admitted policy-local preset identity; it is not a registered wire value.
/// There is deliberately no `Default` or "clear" variant, and (CP D1446) no preset table is
/// authored here: the repository has no sourced rain/fog/snow preset or weight data, so the
/// World-config owner must supply an admitted [`WeatherPolicy`] through the
/// [`WorldWeatherSource`] seam. Client assets remain WEATHER-CLIENT-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WeatherPreset(u8);

impl WeatherPreset {
    pub(crate) fn index(self) -> u8 {
        self.0
    }
}

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

/// The retained, World-bound weather binding and its admitted policy: the seam through which a
/// connection observes weather (D1025/D1446). Immutable input data; it is neither ownership nor
/// session authority, and it holds no timer, history or mutable RNG. Restart, reconnect and
/// Channel move construct no new seed: the observation is a pure function of this binding and
/// the absolute slot.
pub(crate) struct WorldWeatherSource {
    world: WorldId,
    config: RetainedWorldConfig,
    policy: WeatherPolicy,
}

impl WorldWeatherSource {
    pub(crate) fn new(world: WorldId, config: RetainedWorldConfig, policy: WeatherPolicy) -> Self {
        Self {
            world,
            config,
            policy,
        }
    }

    /// The full-replacement observation at a UTC time fact. `Unavailable` (never a fictitious
    /// preset) when the binding is not readable for this World or the fact precedes the epoch.
    pub(crate) fn observe(&self, now_utc_ms: u64) -> WorldWeather {
        self.try_observe(now_utc_ms)
            .unwrap_or(WorldWeather::Unavailable)
    }

    fn try_observe(&self, now_utc_ms: u64) -> Option<WorldWeather> {
        let read = self.config.read(self.world)?;
        let epoch = read.clock.epoch_utc_ms();
        let slot = absolute_weather_slot(epoch, now_utc_ms)?;
        let identity =
            binding_identity(self.world, read.clock_identity, read.policy_revision, epoch);
        let token = digest(b"oteryn.weather.binding.v1", &identity, &[]);
        let occurrence = digest(
            b"oteryn.weather.occurrence.v1",
            &identity,
            &slot.to_be_bytes(),
        );
        let mut id = [0_u8; 16];
        id.copy_from_slice(&occurrence[..16]);
        let preset = self
            .policy
            .select(read.root, DecisionOccurrenceId::from_bytes(id))
            .ok()?;
        Some(WorldWeather::Active {
            binding_token: token,
            absolute_slot: slot,
            preset_index: preset.index(),
        })
    }
}

/// Canonical public binding identity: World, clock identity, policy revision and epoch. Never
/// contains the seed.
fn binding_identity(world: WorldId, clock: &str, policy: &str, epoch: u64) -> Vec<u8> {
    let mut identity = world.as_bytes().to_vec();
    for text in [clock, policy] {
        identity.extend_from_slice(&(text.len() as u32).to_be_bytes());
        identity.extend_from_slice(text.as_bytes());
    }
    identity.extend_from_slice(&epoch.to_be_bytes());
    identity
}

fn digest(domain: &[u8], identity: &[u8], suffix: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::new()
        .chain_update(domain)
        .chain_update(identity)
        .chain_update(suffix)
        .finalize()
        .into()
}

#[cfg(test)]
#[path = "world_weather_tests.rs"]
mod tests;
