//! Generated raid catalogue admission and deterministic firing realization.
//! Source citations and unresolved facts remain in content; only fully bound
//! records can produce a definition for the production runner.
use super::{
    Announcement, AnnouncementKind, CHECK_INTERVAL_MS, LUA_RAID_ACTIVE_IDLE_MS,
    LUA_RAID_CHECKS_PER_DAY, MAX_CANDIDATES, MAX_CREATURES, MAX_WAVES, RaidArea,
    RaidCalendarWindow, RaidDefinition, RaidEndPolicy, RaidError, RaidPolicy, RaidPosition,
    RaidRandom, Wave, WaveEntry,
};
use oteryn_gameplay_content::content::ProductionKey;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const CATALOGUE: &str = include_str!("../../../../../content/encounters/raids/catalogue.json");
const SCHEMA: &str = "OTERYN_RAID_CATALOGUE/v1";
const COORDINATE_FRAME: &str = "global-target-2026-09-27";

#[derive(Debug, Clone, Deserialize)]
pub struct RaidCatalogue {
    schema: String,
    records: Vec<CatalogueRaid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueKind {
    City,
    Area,
    Boss,
    MultiVariant,
    Calendar,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueRaid {
    pub key: String,
    pub kind: CatalogueKind,
    pub active: bool,
    pub inactive_reasons: Vec<String>,
    pub schedule: CatalogueSchedule,
    pub variant_group: Option<String>,
    pub variants: Vec<CatalogueVariant>,
    pub end: CatalogueEnd,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogueSchedule {
    LegacyXml {
        interval_s: u32,
        margin_ms: u64,
        repeat: bool,
        source_order: u16,
    },
    Lua {
        allowed_weekdays: Vec<u8>,
        min_active_players: u32,
        check_interval_ms: u32,
        active_idle_ms: u32,
        checks_per_day: u32,
        min_gap_ms: u64,
        max_checks_per_day: Option<u32>,
        initial_chance_milli_percent: u32,
        target_chance_per_day_milli_percent: u32,
        max_chance_per_check_milli_percent: u32,
    },
    Calendar {
        window: Option<CatalogueWindow>,
        interval_ms: Option<u32>,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CatalogueWindow {
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
}

impl CatalogueWindow {
    pub fn window(self) -> RaidCalendarWindow {
        RaidCalendarWindow {
            start_month: self.start_month,
            start_day: self.start_day,
            end_month: self.end_month,
            end_day: self.end_day,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueVariant {
    pub key: String,
    pub announcements: Vec<CatalogueAnnouncement>,
    pub waves: Vec<CatalogueWave>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueAnnouncement {
    pub delay_ms: u32,
    pub text: String,
    pub kind: CatalogueAnnouncementKind,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueAnnouncementKind {
    Warning,
    Advance,
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueWave {
    pub delay_ms: u32,
    pub entries: Vec<CatalogueEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueEntry {
    pub creature: String,
    pub amount: CatalogueAmount,
    pub area_anchor: String,
    pub area: CatalogueArea,
    pub boss: bool,
    pub spawn_mode: CatalogueSpawnMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueSpawnMode {
    Single,
    Area,
    Zone,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CatalogueAmount {
    pub min: u16,
    pub max: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueArea {
    pub from: CataloguePosition,
    pub to: CataloguePosition,
    pub coordinate_frame: Option<String>,
    pub regions: Vec<CatalogueRegion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogueRegion {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CataloguePosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

impl CataloguePosition {
    pub fn position(self) -> RaidPosition {
        RaidPosition {
            x: self.x,
            y: self.y,
            floor: self.floor,
        }
    }
}

impl CatalogueArea {
    pub fn bound(&self) -> Result<RaidArea, RaidError> {
        let area = RaidArea {
            from: self.from.position(),
            to: self.to.position(),
        };
        area.validate()?;
        if self.coordinate_frame.as_deref() != Some(COORDINATE_FRAME)
            || self.regions.is_empty()
            || [area.from, area.to].iter().any(|at| {
                !(0..=65_535).contains(&at.x)
                    || !(0..=65_535).contains(&at.y)
                    || !(0..=15).contains(&at.floor)
            })
            || self.regions.iter().any(|region| {
                !region.path.starts_with("content/world/placements/region-")
                    || region.sha256.len() != 64
                    || !region.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(area)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogueEnd {
    UntilReset,
    DespawnAfter { despawn_after_ms: u32 },
    BossDeathOrTimeout { despawn_after_ms: u32 },
    Unknown,
}

impl CatalogueEnd {
    pub fn policy(&self) -> Result<RaidEndPolicy, RaidError> {
        match *self {
            Self::UntilReset => Ok(RaidEndPolicy::UntilReset),
            Self::DespawnAfter { despawn_after_ms } if despawn_after_ms > 0 => {
                Ok(RaidEndPolicy::DespawnAfter {
                    after_last_wave_ms: despawn_after_ms,
                })
            }
            Self::BossDeathOrTimeout { despawn_after_ms } if despawn_after_ms > 0 => {
                Ok(RaidEndPolicy::BossDeathOrTimeout {
                    after_last_wave_ms: despawn_after_ms,
                })
            }
            _ => Err(RaidError::InvalidDefinition),
        }
    }

    fn despawn_after_ms(&self) -> Option<u32> {
        match *self {
            Self::DespawnAfter { despawn_after_ms }
            | Self::BossDeathOrTimeout { despawn_after_ms } => Some(despawn_after_ms),
            Self::UntilReset | Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedRaid {
    pub definition: RaidDefinition,
    pub variant_index: usize,
    pub areas: BTreeMap<ProductionKey, RaidArea>,
    pub boss_entries: BTreeSet<(u16, u16)>,
    pub spawn_modes: BTreeMap<(u16, u16), CatalogueSpawnMode>,
    pub end_policy: RaidEndPolicy,
}

impl RaidCatalogue {
    pub fn embedded() -> Result<Self, RaidError> {
        Self::from_json(CATALOGUE)
    }

    pub fn from_json(source: &str) -> Result<Self, RaidError> {
        let catalogue: Self =
            serde_json::from_str(source).map_err(|_| RaidError::InvalidDefinition)?;
        if catalogue.schema != SCHEMA
            || catalogue.records.is_empty()
            || catalogue.records.len() > MAX_CANDIDATES
        {
            return Err(RaidError::InvalidDefinition);
        }
        let mut keys = BTreeSet::new();
        for record in &catalogue.records {
            if !keys.insert(record.production_key()?)
                || record.active != record.inactive_reasons.is_empty()
                || record
                    .inactive_reasons
                    .iter()
                    .any(|reason| reason.trim().is_empty())
            {
                return Err(RaidError::InvalidDefinition);
            }
            if record.active {
                record.validate()?;
            }
        }
        Ok(catalogue)
    }

    pub fn records(&self) -> &[CatalogueRaid] {
        &self.records
    }

    pub fn active(&self) -> impl Iterator<Item = &CatalogueRaid> {
        self.records.iter().filter(|raid| raid.active)
    }

    pub fn get(&self, key: &ProductionKey) -> Option<&CatalogueRaid> {
        self.records.iter().find(|raid| raid.key == key.as_str())
    }
}

impl CatalogueRaid {
    pub fn production_key(&self) -> Result<ProductionKey, RaidError> {
        ProductionKey::new(&self.key).map_err(|_| RaidError::InvalidDefinition)
    }

    pub fn validate(&self) -> Result<(), RaidError> {
        if !self.active || !self.inactive_reasons.is_empty() || self.variants.is_empty() {
            return Err(RaidError::InvalidDefinition);
        }
        match &self.schedule {
            CatalogueSchedule::LegacyXml { interval_s: 0, .. } | CatalogueSchedule::Unknown => {
                return Err(RaidError::InvalidDefinition);
            }
            CatalogueSchedule::Lua {
                allowed_weekdays,
                check_interval_ms,
                active_idle_ms,
                checks_per_day,
                max_checks_per_day,
                initial_chance_milli_percent,
                target_chance_per_day_milli_percent,
                max_chance_per_check_milli_percent,
                ..
            } if allowed_weekdays.is_empty()
                || allowed_weekdays.iter().any(|day| !(1..=7).contains(day))
                // Crystal data/libs/systems/raids.lua:14-15,90-102. These
                // scheduler constants have no source-supported per-raid override.
                || i64::from(*check_interval_ms) != CHECK_INTERVAL_MS
                || u64::from(*active_idle_ms) != LUA_RAID_ACTIVE_IDLE_MS
                || *checks_per_day != LUA_RAID_CHECKS_PER_DAY
                || *max_checks_per_day == Some(0)
                || *initial_chance_milli_percent > 100_000
                || *target_chance_per_day_milli_percent > 100_000
                || *max_chance_per_check_milli_percent > 100_000 =>
            {
                return Err(RaidError::InvalidDefinition);
            }
            CatalogueSchedule::Calendar {
                window,
                interval_ms,
            } => {
                window
                    .ok_or(RaidError::InvalidCalendar)?
                    .window()
                    .validate()?;
                if interval_ms.is_none_or(|interval| interval == 0) {
                    return Err(RaidError::InvalidCalendar);
                }
            }
            _ => {}
        }
        let mut variants = BTreeSet::new();
        for (index, variant) in self.variants.iter().enumerate() {
            if !variants.insert(&variant.key) || variant.waves.len() > MAX_WAVES {
                return Err(RaidError::InvalidDefinition);
            }
            // Validate each variant at its maximum requested population, not a
            // lucky small sample that could hide an invalid range or capacity.
            self.realize(index, None)?.definition.deadline(0)?;
        }
        Ok(())
    }

    pub fn select(&self, seed: [u8; 32], firing: [u8; 16]) -> Result<SelectedRaid, RaidError> {
        if !self.active || !self.inactive_reasons.is_empty() {
            return Err(RaidError::InvalidDefinition);
        }
        let random = RaidRandom::new(seed, firing);
        let index = random.variant(self.variants.len())?;
        self.realize(index, Some(&random))
    }

    fn realize(
        &self,
        index: usize,
        random: Option<&RaidRandom>,
    ) -> Result<SelectedRaid, RaidError> {
        let variant = self
            .variants
            .get(index)
            .ok_or(RaidError::InvalidDefinition)?;
        let mut areas = BTreeMap::new();
        let mut boss_entries = BTreeSet::new();
        let mut spawn_modes = BTreeMap::new();
        let mut waves = Vec::with_capacity(variant.waves.len());
        for (wave_index, wave) in variant.waves.iter().enumerate() {
            let wave_index = u16::try_from(wave_index).map_err(|_| RaidError::Exhausted)?;
            let mut entries = Vec::with_capacity(wave.entries.len());
            for (entry_index, entry) in wave.entries.iter().enumerate() {
                let entry_index = u16::try_from(entry_index).map_err(|_| RaidError::Exhausted)?;
                if entry.amount.min == 0
                    || entry.amount.min > entry.amount.max
                    || entry.amount.max > MAX_CREATURES
                {
                    return Err(RaidError::InvalidDefinition);
                }
                let creature = ProductionKey::new(&entry.creature)
                    .map_err(|_| RaidError::InvalidDefinition)?;
                let anchor = ProductionKey::new(&entry.area_anchor)
                    .map_err(|_| RaidError::InvalidDefinition)?;
                let area = entry.area.bound()?;
                if areas
                    .insert(anchor.clone(), area)
                    .is_some_and(|existing| existing != area)
                {
                    return Err(RaidError::InvalidDefinition);
                }
                let count = if let Some(random) = random {
                    random.amount(wave_index, entry_index, entry.amount.min, entry.amount.max)?
                } else {
                    entry.amount.max
                };
                spawn_modes.insert((wave_index, entry_index), entry.spawn_mode);
                if entry.boss {
                    boss_entries.insert((wave_index, entry_index));
                }
                entries.push(WaveEntry {
                    creature,
                    count,
                    area_anchor: anchor,
                });
            }
            waves.push(Wave {
                delay_ms: wave.delay_ms,
                entries,
            });
        }
        let announcements = variant
            .announcements
            .iter()
            .map(|announcement| {
                Ok(Announcement {
                    delay_ms: announcement.delay_ms,
                    text: announcement.text.clone(),
                    kind: match announcement.kind {
                        CatalogueAnnouncementKind::Warning => AnnouncementKind::Warning,
                        CatalogueAnnouncementKind::Advance => AnnouncementKind::Advance,
                        CatalogueAnnouncementKind::Unknown => {
                            return Err(RaidError::InvalidDefinition);
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>, RaidError>>()?;
        let policy = match self.schedule {
            CatalogueSchedule::LegacyXml { interval_s, .. } => RaidPolicy::Random {
                mean_interval_s: interval_s,
                min_gap_s: 0,
            },
            CatalogueSchedule::Lua { .. } | CatalogueSchedule::Calendar { .. } => {
                RaidPolicy::SourceScheduled
            }
            CatalogueSchedule::Unknown => return Err(RaidError::InvalidDefinition),
        };
        let definition = RaidDefinition {
            policy,
            despawn_after_ms: self.end.despawn_after_ms(),
            announcements,
            waves,
        };
        definition.validate()?;
        Ok(SelectedRaid {
            definition,
            variant_index: index,
            areas,
            boss_entries,
            spawn_modes,
            end_policy: self.end.policy()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raid::{RaidRunAction, RaidRunCursor};

    #[test]
    fn every_active_catalogue_raid_parses_and_inactive_has_reasons()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = RaidCatalogue::embedded()?;
        // RAIDS-GLOBAL 2026-11-01 replaces the former single-Badger catalogue:
        // 160 Crystal XML definitions + Sugar Mommy + 78 unresolved wiki identities.
        assert_eq!(catalogue.records().len(), 239);
        assert_eq!(catalogue.active().count(), 135);
        for record in catalogue.records() {
            if record.active {
                record.select([7; 32], [9; 16])?.definition.validate()?;
            } else {
                assert!(!record.inactive_reasons.is_empty());
                assert!(record.select([7; 32], [9; 16]).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn catalogue_capacity_accepts_512_and_rejects_513() -> Result<(), Box<dyn std::error::Error>> {
        let mut source: serde_json::Value = serde_json::from_str(CATALOGUE)?;
        let template = source["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|record| record["key"] == "raid:badger")
            .ok_or("badger")?
            .clone();
        let mut records = Vec::with_capacity(MAX_CANDIDATES);
        for index in 0..MAX_CANDIDATES {
            let mut record = template.clone();
            record["key"] = serde_json::json!(format!("raid:capacity-{index}"));
            records.push(record);
        }
        source["records"] = serde_json::Value::Array(records);
        let maximum = RaidCatalogue::from_json(&serde_json::to_string(&source)?)?;
        assert_eq!(maximum.records().len(), MAX_CANDIDATES);
        let mut excess = template;
        excess["key"] = serde_json::json!("raid:capacity-excess");
        source["records"]
            .as_array_mut()
            .ok_or("records")?
            .push(excess);
        assert!(RaidCatalogue::from_json(&serde_json::to_string(&source)?).is_err());
        Ok(())
    }

    #[test]
    fn inactive_reason_and_native_area_cannot_be_bypassed() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut source: serde_json::Value = serde_json::from_str(CATALOGUE)?;
        let records = source["records"].as_array_mut().ok_or("records")?;
        let inactive = records
            .iter_mut()
            .find(|r| r["active"] == false)
            .ok_or("inactive")?;
        inactive["inactive_reasons"] = serde_json::json!([]);
        assert!(RaidCatalogue::from_json(&serde_json::to_string(&source)?).is_err());
        let mut source: serde_json::Value = serde_json::from_str(CATALOGUE)?;
        let records = source["records"].as_array_mut().ok_or("records")?;
        let active = records
            .iter_mut()
            .find(|r| r["active"] == true)
            .ok_or("active")?;
        active["variants"][0]["waves"][0]["entries"][0]["area"]["regions"] = serde_json::json!([]);
        assert!(RaidCatalogue::from_json(&serde_json::to_string(&source)?).is_err());
        Ok(())
    }

    #[test]
    fn a_firing_selects_one_variant_and_replays_same_amounts()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = RaidCatalogue::embedded()?;
        let mut raid = catalogue
            .get(&ProductionKey::new("raid:badger")?)
            .ok_or("badger")?
            .clone();
        let mut variant = raid.variants[0].clone();
        variant.key = "alternate-source-variant".into();
        variant.waves[0].entries[0].amount = CatalogueAmount { min: 2, max: 5 };
        raid.variants.push(variant);
        raid.validate()?;
        let selected = raid.select([3; 32], [5; 16])?;
        assert_eq!(selected, raid.select([3; 32], [5; 16])?);
        assert_eq!(selected.definition.waves.len(), 1);
        assert!(selected.variant_index < 2);
        Ok(())
    }

    #[test]
    fn registered_variants_keep_source_order_and_only_selected_record_spawns()
    -> Result<(), Box<dyn std::error::Error>> {
        // Crystal raids.xml:68-70 registers these separately. The source
        // scheduler checks them in order and starts only one (raids.cpp:128-141).
        let catalogue = RaidCatalogue::embedded()?;
        let mut selected = Vec::new();
        for (offset, suffix) in [1, 2, 3].into_iter().enumerate() {
            let key = ProductionKey::new(&format!("raid:farmine.fleabringer{suffix}"))?;
            let raid = catalogue.get(&key).ok_or("Fleabringer variant")?;
            assert_eq!(raid.kind, CatalogueKind::MultiVariant);
            assert_eq!(
                raid.variant_group.as_deref(),
                Some("raid-group:farmine.fleabringer")
            );
            assert_eq!(raid.variants.len(), 1);
            let CatalogueSchedule::LegacyXml { source_order, .. } = raid.schedule else {
                return Err("source schedule identity".into());
            };
            assert_eq!(usize::from(source_order), 54 + offset);
            let realization = raid.select([3; 32], [5; 16])?;
            assert_eq!(realization.definition.waves.len(), 3);
            assert_eq!(realization.areas.len(), 3);
            selected.push(realization);
        }
        for sibling in &selected[1..] {
            assert!(
                sibling
                    .areas
                    .keys()
                    .all(|anchor| !selected[0].areas.contains_key(anchor))
            );
        }
        Ok(())
    }

    #[test]
    fn source_maxima_and_post_spawn_announcement_remain_executable()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = RaidCatalogue::embedded()?;
        let walkers = catalogue
            .get(&ProductionKey::new("raid:oramond.walkers")?)
            .ok_or("walkers")?;
        assert_eq!(walkers.select([1; 32], [1; 16])?.definition.waves.len(), 50);
        let vampires = catalogue
            .get(&ProductionKey::new("raid:yalahar.vampires")?)
            .ok_or("vampires")?;
        let vampires = vampires.select([1; 32], [1; 16])?;
        assert_eq!(
            vampires
                .definition
                .waves
                .iter()
                .flat_map(|wave| &wave.entries)
                .map(|entry| entry.count)
                .sum::<u16>(),
            500
        );
        let trolls = catalogue
            .get(&ProductionKey::new("raid:carlin.trolls")?)
            .ok_or("trolls")?;
        // Global count evidence conflicts with this source, so production
        // admission is blocked. Its donor event ordering still proves the pure
        // planner must retain broadcasts that follow the final spawn.
        assert!(!trolls.active);
        assert!(trolls.select([1; 32], [1; 16]).is_err());
        let definition = trolls.realize(0, None)?.definition;
        let mut cursor = RaidRunCursor::default();
        let mut last_announcement = None;
        let mut complete = None;
        while let Some(step) = cursor.next_due(&definition, 0, 1_000_000)? {
            match step.action {
                RaidRunAction::Announce { .. } => last_announcement = Some(step.due_at_ms),
                RaidRunAction::Complete => complete = Some(step.due_at_ms),
                RaidRunAction::SpawnWave { .. } => {}
            }
            cursor = cursor.commit(&step)?;
        }
        assert_eq!(last_announcement, Some(183_000));
        assert_eq!(complete, last_announcement);
        Ok(())
    }
}
