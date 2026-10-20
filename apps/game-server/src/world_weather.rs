//! WEATHER-1: Oteryn-designed visual weather (CP D1023-D1028, D1446, D1448).
//!
//! CP D1448: the owner chose Oteryn's own weather values, live in production. Every value here
//! is `OTERYN_DESIGN` (not Tibia reference data) and is visual only: no gameplay state reads or
//! writes it. Weather is a stateless, deterministic, public function of (area, absolute 15
//! minute slot): no secret seed, timer, history or mutable RNG exists, so restart, reconnect and
//! Channel transfer observe the same value. Each region and each city is its own independent
//! area with its own draw.
//!
//! Periods: the slot axis is cut into 12-slot blocks. Inside a block, periods follow each other,
//! each lasting `duration_slots` (1 to 3 slots, 15 to 45 minutes) with the last one truncated at
//! the block end. A period draws its kind from the area's climate weights and its intensity
//! uniformly (1..=3).
//!
//! Extension point: [`WeatherSource`] is the seam for a later real-world binding. Area data
//! already carries `real_world: null | {latitude, longitude}`; a live source can implement the
//! trait and read those coordinates. That binding is not implemented here.
//!
//! Location approximation (OTERYN_DESIGN): no area geometry exists server-side yet, so a player
//! maps to the nearest area anchor (a region position or a city hometown temple) by x and y.
//! Areas without an anchor never win a lookup.

use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_weather::{
    WEATHER_INTENSITY_MAX, WEATHER_KIND_MAX, WorldWeather,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// D1023: 360 Tibian minutes, each 2,500 real milliseconds.
pub(crate) const WEATHER_SLOT_MS: u64 = 900_000;
const BLOCK_SLOTS: u64 = 12;

const INDEX_SCHEMA: &str = "OTERYN_WEATHER_INDEX/v1";
const CLIMATES_SCHEMA: &str = "OTERYN_WEATHER_CLIMATES/v1";
const AREAS_SCHEMA: &str = "OTERYN_WEATHER_AREAS/v1";

/// The kind names the data may use, with their wire codes.
const KIND_CODES: [(&str, u8); 9] = [
    ("clear", 1),
    ("cloudy", 2),
    ("rain", 3),
    ("storm", 4),
    ("fog", 5),
    ("snow", 6),
    ("blizzard", 7),
    ("heat_haze", 8),
    ("sandstorm", 9),
];

/// One weather period: it starts at `start_slot` and holds until the next period starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WeatherPeriod {
    pub(crate) start_slot: u64,
    pub(crate) kind: u8,
    pub(crate) intensity: u8,
}

/// The seam between "where" and "what": given an area index and an absolute slot, the period in
/// effect. [`OterynDesignSource`] is today's implementation; a later real-world source can read
/// the area's `real_world` coordinates instead (not implemented, CP D1448).
pub(crate) trait WeatherSource {
    fn period(&self, area: &WeatherArea, slot: u64) -> Option<WeatherPeriod>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WeatherDataError {
    Parse(&'static str),
    Schema(&'static str),
    Invalid(&'static str),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile {
    schema: String,
    family: String,
    kind: String,
    owner: String,
    population_state: String,
    design_status: String,
    revision: String,
    slot_ms: u64,
    files: BTreeMap<String, String>,
    notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    min: u8,
    max: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClimatesFile {
    schema: String,
    design_status: String,
    notes: String,
    duration_slots: Range,
    intensity: Range,
    climates: BTreeMap<String, BTreeMap<String, u32>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchorData {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RealWorldData {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AreaData {
    key: String,
    kind: String,
    climate: String,
    anchor: Option<AnchorData>,
    real_world: Option<RealWorldData>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AreasFile {
    schema: String,
    design_status: String,
    notes: String,
    areas: Vec<AreaData>,
}

/// One region or city: its climate, its lookup anchor and the optional real-world coordinates
/// reserved for a later live binding.
#[derive(Debug, Clone)]
pub(crate) struct WeatherArea {
    pub(crate) key: String,
    climate: usize,
    anchor: Option<(i32, i32)>,
    pub(crate) real_world: Option<(f64, f64)>,
}

#[derive(Debug, Clone)]
struct Climate {
    /// Cumulative weights with the kind wire code.
    cumulative: Vec<(u64, u8)>,
    total: u64,
}

/// The shipped Oteryn design: public, deterministic, sha256-based draws.
#[derive(Debug, Clone)]
pub(crate) struct OterynDesignSource {
    revision: String,
    climates: Vec<Climate>,
    duration: (u8, u8),
    intensity: (u8, u8),
}

fn draw(revision: &str, area: &str, block: u64, index: u32, purpose: u8) -> u64 {
    let digest = Sha256::new()
        .chain_update(b"oteryn.weather.design.v1")
        .chain_update((revision.len() as u32).to_be_bytes())
        .chain_update(revision.as_bytes())
        .chain_update((area.len() as u32).to_be_bytes())
        .chain_update(area.as_bytes())
        .chain_update(block.to_be_bytes())
        .chain_update(index.to_be_bytes())
        .chain_update([purpose])
        .finalize();
    let mut word = [0_u8; 8];
    word.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(word)
}

/// Maps a uniform u64 onto `0..bound` without modulo bias worth caring about for a visual draw.
fn scale(draw: u64, bound: u64) -> u64 {
    ((u128::from(draw) * u128::from(bound)) >> 64) as u64
}

impl WeatherSource for OterynDesignSource {
    fn period(&self, area: &WeatherArea, slot: u64) -> Option<WeatherPeriod> {
        let climate = self.climates.get(area.climate)?;
        let block = slot / BLOCK_SLOTS;
        let block_start = block * BLOCK_SLOTS;
        let block_end = block_start + BLOCK_SLOTS;
        let span = u64::from(self.duration.1 - self.duration.0) + 1;
        let intensity_span = u64::from(self.intensity.1 - self.intensity.0) + 1;
        let mut start = block_start;
        let mut index = 0_u32;
        while start < block_end {
            let length = u64::from(self.duration.0)
                + scale(draw(&self.revision, &area.key, block, index, b'd'), span);
            let end = (start + length).min(block_end);
            if slot < end {
                let pick = scale(
                    draw(&self.revision, &area.key, block, index, b'k'),
                    climate.total,
                );
                let kind = climate
                    .cumulative
                    .iter()
                    .find(|(upper, _)| pick < *upper)
                    .map(|(_, kind)| *kind)?;
                let intensity = u64::from(self.intensity.0)
                    + scale(
                        draw(&self.revision, &area.key, block, index, b'i'),
                        intensity_span,
                    );
                return Some(WeatherPeriod {
                    start_slot: start,
                    kind,
                    intensity: u8::try_from(intensity).ok()?,
                });
            }
            start = end;
            index += 1;
        }
        None
    }
}

/// The loaded, validated weather data and the source that turns it into observations.
#[derive(Debug)]
pub(crate) struct WorldWeatherSource {
    source: OterynDesignSource,
    areas: Vec<WeatherArea>,
    token: [u8; 32],
}

impl WorldWeatherSource {
    /// Loads and validates the three JSON files. Any problem fails closed: the caller then holds
    /// no source and never offers capability 26.
    pub(crate) fn from_json(
        index: &str,
        climates: &str,
        areas: &str,
    ) -> Result<Self, WeatherDataError> {
        let index: IndexFile =
            serde_json::from_str(index).map_err(|_| WeatherDataError::Parse("index"))?;
        let climates: ClimatesFile =
            serde_json::from_str(climates).map_err(|_| WeatherDataError::Parse("climates"))?;
        let areas: AreasFile =
            serde_json::from_str(areas).map_err(|_| WeatherDataError::Parse("areas"))?;
        if index.schema != INDEX_SCHEMA
            || climates.schema != CLIMATES_SCHEMA
            || areas.schema != AREAS_SCHEMA
        {
            return Err(WeatherDataError::Schema("schema"));
        }
        for status in [
            &index.design_status,
            &climates.design_status,
            &areas.design_status,
        ] {
            if status != "OTERYN_DESIGN" {
                return Err(WeatherDataError::Schema("design_status"));
            }
        }
        if index.slot_ms != WEATHER_SLOT_MS || index.revision.is_empty() {
            return Err(WeatherDataError::Invalid("index"));
        }
        let duration = (climates.duration_slots.min, climates.duration_slots.max);
        let intensity = (climates.intensity.min, climates.intensity.max);
        if duration.0 == 0
            || duration.0 > duration.1
            || u64::from(duration.1) > BLOCK_SLOTS
            || intensity.0 == 0
            || intensity.0 > intensity.1
            || intensity.1 > WEATHER_INTENSITY_MAX
        {
            return Err(WeatherDataError::Invalid("ranges"));
        }
        let mut climate_names = Vec::new();
        let mut table = Vec::new();
        for (name, weights) in &climates.climates {
            let mut cumulative = Vec::new();
            let mut total = 0_u64;
            for (kind, weight) in weights {
                let code = KIND_CODES
                    .iter()
                    .find(|(known, _)| known == kind)
                    .map(|(_, code)| *code)
                    .filter(|code| *code <= WEATHER_KIND_MAX)
                    .ok_or(WeatherDataError::Invalid("kind"))?;
                if *weight == 0 {
                    return Err(WeatherDataError::Invalid("weight"));
                }
                total += u64::from(*weight);
                cumulative.push((total, code));
            }
            if cumulative.is_empty() {
                return Err(WeatherDataError::Invalid("climate"));
            }
            climate_names.push(name.clone());
            table.push(Climate { cumulative, total });
        }
        if table.is_empty() {
            return Err(WeatherDataError::Invalid("climates"));
        }
        let mut seen = BTreeSet::new();
        let mut loaded = Vec::new();
        for area in areas.areas {
            if area.key.is_empty() || !seen.insert(area.key.clone()) {
                return Err(WeatherDataError::Invalid("area key"));
            }
            let climate = climate_names
                .iter()
                .position(|name| *name == area.climate)
                .ok_or(WeatherDataError::Invalid("area climate"))?;
            let real_world = match area.real_world {
                None => None,
                Some(point) => {
                    if !(-90.0..=90.0).contains(&point.latitude)
                        || !(-180.0..=180.0).contains(&point.longitude)
                    {
                        return Err(WeatherDataError::Invalid("real_world"));
                    }
                    Some((point.latitude, point.longitude))
                }
            };
            loaded.push(WeatherArea {
                key: area.key,
                climate,
                anchor: area.anchor.map(|anchor| (anchor.x, anchor.y)),
                real_world,
            });
        }
        if !loaded.iter().any(|area| area.anchor.is_some()) {
            return Err(WeatherDataError::Invalid("anchors"));
        }
        let token: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn.weather.binding.v1")
            .chain_update(index.revision.as_bytes())
            .finalize()
            .into();
        Ok(Self {
            source: OterynDesignSource {
                revision: index.revision,
                climates: table,
                duration,
                intensity,
            },
            areas: loaded,
            token,
        })
    }

    /// The nearest anchored area to a position (x and y only; the first of equal distances in
    /// key order). OTERYN_DESIGN approximation until area geometry exists.
    pub(crate) fn area_at(&self, position: ActorPosition) -> Option<&WeatherArea> {
        self.areas
            .iter()
            .filter_map(|area| {
                let (x, y) = area.anchor?;
                let dx = i128::from(position.x) - i128::from(x);
                let dy = i128::from(position.y) - i128::from(y);
                Some((dx * dx + dy * dy, area))
            })
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, area)| area)
    }

    /// The full-replacement observation for a position at a UTC time fact. `Unavailable` (never
    /// a fictitious value) when no area or period resolves.
    pub(crate) fn observe(&self, position: ActorPosition, now_utc_ms: u64) -> WorldWeather {
        self.try_observe(position, now_utc_ms)
            .unwrap_or(WorldWeather::Unavailable)
    }

    fn try_observe(&self, position: ActorPosition, now_utc_ms: u64) -> Option<WorldWeather> {
        let area = self.area_at(position)?;
        let period = self.source.period(area, now_utc_ms / WEATHER_SLOT_MS)?;
        Some(WorldWeather::Active {
            binding_token: self.token,
            absolute_slot: period.start_slot,
            kind: period.kind,
            intensity: period.intensity,
        })
    }

    #[cfg(test)]
    pub(crate) fn areas(&self) -> &[WeatherArea] {
        &self.areas
    }

    #[cfg(test)]
    pub(crate) fn source(&self) -> &OterynDesignSource {
        &self.source
    }
}

/// The shipped data, loaded once. `None` when it is missing or invalid: capability 26 is then
/// never offered (fail closed).
pub(crate) fn shipped() -> Option<&'static WorldWeatherSource> {
    static SHIPPED: OnceLock<Option<WorldWeatherSource>> = OnceLock::new();
    SHIPPED
        .get_or_init(|| {
            WorldWeatherSource::from_json(
                include_str!("../../../content/weather/index.json"),
                include_str!("../../../content/weather/climates.json"),
                include_str!("../../../content/weather/areas.json"),
            )
            .ok()
        })
        .as_ref()
}

#[cfg(test)]
#[path = "world_weather_tests.rs"]
mod tests;
