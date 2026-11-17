//! Credited World progress. Persistence, current authority and native source eligibility
//! stay with the existing World-event owner and its consumers.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterKind {
    Deeplings,
    DemonWars,
    FireEarth,
    HiveBorn,
    Overhunting,
    FireFeathered,
    MagesTower,
    TheirMastersVoice,
}
impl CounterKind {
    pub const ALL: [Self; 8] = [
        Self::Deeplings,
        Self::DemonWars,
        Self::FireEarth,
        Self::HiveBorn,
        Self::Overhunting,
        Self::FireFeathered,
        Self::MagesTower,
        Self::TheirMastersVoice,
    ];
    pub const fn key(self) -> &'static str {
        match self {
            Self::Deeplings => "deeplings",
            Self::DemonWars => "demon_wars",
            Self::FireEarth => "fire_from_the_earth",
            Self::HiveBorn => "hive_born",
            Self::Overhunting => "overhunting",
            Self::FireFeathered => "the_fire_feathered_serpent",
            Self::MagesTower => "the_mage_s_tower",
            Self::TheirMastersVoice => "their_masters_voice",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
    pub const fn index(self) -> usize {
        self as usize
    }
}
/// Unspecified reference numbers require explicit authored bindings; zero is invalid
/// for the corresponding mechanic. There is no implicit production balance preset.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub deepling_points: u64,
    pub deer_kills: u64,
    pub wolf_recovery: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Credit {
    Kill(u8),
    Task(u8),
    TrappedWolves(u32),
    HeartDestroyed,
    SoulNetDestroyed,
    CoralPathCompleted,
    StairPathCompleted,
    MissionComplete,
    /// The native phase producer proves this exact current wave/home-slot cohort.
    VoiceServant {
        wave: u8,
        ordinal: u8,
    },
    VoiceMage,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterError {
    InvalidState,
    InvalidCredit,
    MissingParameter,
    Overflow,
    StaleEpoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterState {
    pub epoch: u64,
    pub entered_epoch: u64,
    pub stage: u8,
    pub phase_version: u64,
    pub credit_generation: u64,
    /// Personal quest lifetime; Hive/Deepling cycles can span several saves.
    #[serde(default = "initial_occurrence_generation")]
    pub occurrence_generation: u64,
    pub revision: u64,
    pub counts: [u64; 3],
    pub condition: bool,
    pub deadline_ms: Option<u64>,
}
impl CounterState {
    pub const fn new(kind: CounterKind, epoch: u64, earth_active: bool) -> Self {
        Self {
            epoch,
            entered_epoch: epoch,
            stage: match kind {
                CounterKind::DemonWars => 3,
                CounterKind::FireEarth => earth_active as u8,
                _ => 1,
            },
            phase_version: 1,
            credit_generation: 1,
            occurrence_generation: 1,
            revision: 1,
            counts: [0; 3],
            condition: false,
            deadline_ms: None,
        }
    }
    pub fn validate(self, kind: CounterKind) -> Result<(), CounterError> {
        let valid = match kind {
            CounterKind::DemonWars => (1..=5).contains(&self.stage),
            CounterKind::MagesTower => (1..=4).contains(&self.stage),
            CounterKind::TheirMastersVoice => {
                (1..=4).contains(&self.stage)
                    && self.counts[2] < (1 << 25)
                    && match self.stage {
                        1 => self.counts[0] < 100 && self.counts[1] == 0 && self.counts[2] == 0,
                        2 => {
                            self.counts[0] >= 100
                                && (1..=25).contains(&self.counts[1])
                                && self.counts[2] != (1 << 25) - 1
                        }
                        _ => {
                            self.counts[0] >= 100
                                && self.counts[1] == 25
                                && self.counts[2] == (1 << 25) - 1
                        }
                    }
            }
            CounterKind::Overhunting => (1..=2).contains(&self.stage),
            CounterKind::FireEarth => self.stage <= 1,
            _ => (1..=3).contains(&self.stage),
        };
        if !valid
            || self.entered_epoch > self.epoch
            || self.phase_version == 0
            || self.credit_generation == 0
            || self.occurrence_generation == 0
            || self.revision == 0
            || (self.deadline_ms.is_some()
                != ((kind == CounterKind::MagesTower && self.stage == 3)
                    || (kind == CounterKind::TheirMastersVoice && self.counts[0] > 0)))
        {
            return Err(CounterError::InvalidState);
        }
        Ok(())
    }
    pub const fn is_active(self, kind: CounterKind) -> bool {
        match kind {
            CounterKind::FireEarth => self.stage == 1,
            CounterKind::FireFeathered => self.stage == 3,
            CounterKind::MagesTower => self.stage < 4,
            _ => self.stage > 0,
        }
    }
    fn finish(self, mut next: Self, kind: CounterKind) -> Result<Self, CounterError> {
        if next.stage != self.stage
            || (kind == CounterKind::TheirMastersVoice && next.counts[1] != self.counts[1])
        {
            next.phase_version = self
                .phase_version
                .checked_add(1)
                .ok_or(CounterError::Overflow)?;
            next.entered_epoch = next.epoch;
        }
        if (kind == CounterKind::FireFeathered && self.stage == 3 && next.stage == 1)
            || (kind == CounterKind::TheirMastersVoice
                && (next.stage != self.stage || next.counts[1] != self.counts[1]))
        {
            next.credit_generation = self
                .credit_generation
                .checked_add(1)
                .ok_or(CounterError::Overflow)?;
        }
        // These source lifecycles reset on a downgrade, not on daily epoch changes
        // or forward access stages. Other counter kinds remain unbound for quests.
        let new_occurrence = match kind {
            CounterKind::HiveBorn => next.stage < self.stage,
            CounterKind::Deeplings => self.stage == 3 && next.stage == 1,
            _ => false,
        };
        if new_occurrence {
            next.occurrence_generation = self
                .occurrence_generation
                .checked_add(1)
                .ok_or(CounterError::Overflow)?;
        }
        if next != self {
            next.revision = self.revision.checked_add(1).ok_or(CounterError::Overflow)?;
        }
        next.validate(kind)?;
        Ok(next)
    }
    pub fn credit(
        self,
        kind: CounterKind,
        credit: Credit,
        _parameters: &Parameters,
        now_ms: u64,
    ) -> Result<Self, CounterError> {
        self.validate(kind)?;
        if kind == CounterKind::TheirMastersVoice
            && self.deadline_ms.is_some_and(|deadline| now_ms >= deadline)
        {
            return Err(CounterError::InvalidCredit);
        }
        let mut next = self;
        match (kind, credit) {
            // Crystal their_masters_voice.lua:1-6,59-83,111-163. Twenty-five uses
            // qualify a Character; they do not cap extraction. The durable observer
            // tracks that separately. Each current home-slot dies once across Channels.
            (CounterKind::TheirMastersVoice, Credit::Task(0)) => {
                if next.deadline_ms.is_none() {
                    next.deadline_ms = Some(
                        now_ms
                            .checked_add(3_600_000)
                            .ok_or(CounterError::Overflow)?,
                    );
                }
                add(&mut next.counts[0], 1)?;
                if next.stage == 1 && next.counts[0] >= 100 {
                    next.stage = 2;
                    next.counts[1] = 1;
                }
            }
            (CounterKind::TheirMastersVoice, Credit::VoiceServant { wave, ordinal })
                if next.stage == 2 && u64::from(wave) == next.counts[1] && ordinal < 25 =>
            {
                next.counts[2] |= 1_u64 << ordinal;
                if next.counts[2] == (1_u64 << 25) - 1 {
                    if next.counts[1] == 25 {
                        next.stage = 3;
                    } else {
                        next.counts[1] += 1;
                        next.counts[2] = 0;
                    }
                }
            }
            (CounterKind::TheirMastersVoice, Credit::VoiceMage) if next.stage == 3 => {
                next.stage = 4;
            }
            (CounterKind::DemonWars, Credit::Kill(side @ 0..=1)) => {
                add(&mut next.counts[usize::from(side)], 1)?
            }
            (CounterKind::FireFeathered, Credit::Kill(0)) if next.stage < 3 => {
                add(&mut next.counts[0], 1)?;
                next.stage = if next.counts[0] >= 2000 {
                    3
                } else if next.counts[0] >= 1000 {
                    2
                } else {
                    1
                };
            }
            (CounterKind::FireFeathered, Credit::Kill(1)) if next.stage == 3 => {
                add(&mut next.counts[1], 1)?
            }
            (CounterKind::FireFeathered, Credit::MissionComplete) if next.stage == 3 => {
                next.condition = true
            }
            (CounterKind::FireFeathered, Credit::Kill(0..=1) | Credit::MissionComplete) => {}
            (CounterKind::MagesTower, Credit::Kill(0)) => add(&mut next.counts[0], 1)?,
            (CounterKind::MagesTower, Credit::Kill(1))
                if next.stage == 1 && next.counts[0] >= 2000 =>
            {
                next.stage = 2;
                next.counts[0] = 0;
            }
            (CounterKind::MagesTower, Credit::Kill(2)) if next.stage == 2 => {
                next.stage = 3;
                next.deadline_ms = Some(now_ms.checked_add(300_000).ok_or(CounterError::Overflow)?);
            }
            (CounterKind::MagesTower, Credit::Kill(1..=2)) => {}
            (CounterKind::HiveBorn, Credit::Task(task @ 0..=5)) => {
                let eligible = match next.stage {
                    1 => task < 4,
                    2 => task >= 4,
                    _ => true,
                };
                if eligible {
                    add(&mut next.counts[0], if task < 4 { 1 } else { 2 })?;
                    add(&mut next.counts[1], 1)?;
                }
            }
            (CounterKind::Overhunting, Credit::Kill(0)) if next.stage == 1 => {
                add(&mut next.counts[0], 1)?
            }
            (CounterKind::Overhunting, Credit::Kill(1)) if next.stage == 2 => {
                add(&mut next.counts[1], 1)?
            }
            (CounterKind::Overhunting, Credit::TrappedWolves(units))
                if units > 0 && next.stage == 2 =>
            {
                add(&mut next.counts[1], u64::from(units) * 10)?
            }
            (CounterKind::Overhunting, Credit::Kill(0..=1)) => {}
            (CounterKind::Overhunting, Credit::TrappedWolves(units)) if units > 0 => {}
            (CounterKind::Deeplings, Credit::HeartDestroyed) if next.stage == 1 => {
                add(&mut next.counts[0], 2)?
            }
            (CounterKind::Deeplings, Credit::SoulNetDestroyed) if next.stage == 1 => {
                add(&mut next.counts[0], 5)?
            }
            (CounterKind::Deeplings, Credit::CoralPathCompleted | Credit::StairPathCompleted)
                if next.stage == 2 =>
            {
                next.condition = true
            }
            (
                CounterKind::Deeplings,
                Credit::HeartDestroyed
                | Credit::SoulNetDestroyed
                | Credit::CoralPathCompleted
                | Credit::StairPathCompleted,
            ) => {}
            (CounterKind::FireEarth, Credit::Kill(0)) if next.stage == 1 => {
                add(&mut next.counts[0], 1)?
            }
            (CounterKind::FireEarth, Credit::Kill(0)) => {}
            _ => return Err(CounterError::InvalidCredit),
        }
        if kind == CounterKind::FireFeathered
            && next.stage == 3
            && next.condition
            && next.counts[1] >= 2000
        {
            next.stage = 1;
            next.counts = [0; 3];
            next.condition = false;
        }
        self.finish(next, kind)
    }
    pub fn tick(self, kind: CounterKind, now_ms: u64) -> Result<Self, CounterError> {
        self.validate(kind)?;
        let mut next = self;
        if kind == CounterKind::TheirMastersVoice
            && next.deadline_ms.is_some_and(|deadline| now_ms >= deadline)
        {
            // The donor clears only some local variables. Reset the owned encounter
            // coherently: no retained participants/mageSpawned/current_mage leakage.
            next.stage = 1;
            next.counts = [0; 3];
            next.condition = false;
            next.deadline_ms = None;
            if self.stage == 1 {
                next.phase_version = next
                    .phase_version
                    .checked_add(1)
                    .ok_or(CounterError::Overflow)?;
                next.credit_generation = next
                    .credit_generation
                    .checked_add(1)
                    .ok_or(CounterError::Overflow)?;
            }
        }
        if kind == CounterKind::MagesTower
            && next.deadline_ms.is_some_and(|deadline| now_ms >= deadline)
        {
            next.stage = 4;
            next.deadline_ms = None;
        }
        self.finish(next, kind)
    }
    pub fn at_save(
        self,
        kind: CounterKind,
        epoch: u64,
        earth_active: bool,
        p: &Parameters,
        now_ms: u64,
    ) -> Result<Self, CounterError> {
        self.validate(kind)?;
        if epoch < self.epoch {
            return Err(CounterError::StaleEpoch);
        }
        if epoch == self.epoch {
            return Ok(self);
        }
        let mut next = self.tick(kind, now_ms)?;
        next.epoch = epoch;
        match kind {
            CounterKind::DemonWars => {
                let difference = i128::from(next.counts[0]) - i128::from(next.counts[1]);
                let step = if difference >= 100 {
                    1_i8
                } else if difference <= -100 {
                    -1
                } else {
                    0
                };
                if next.stage == 3 && difference.unsigned_abs() > 400 {
                    next.stage = if difference > 0 { 5 } else { 1 };
                } else {
                    next.stage = u8::try_from((next.stage as i8 + step).clamp(1, 5))
                        .map_err(|_| CounterError::InvalidState)?;
                }
                next.counts = [0; 3];
            }
            CounterKind::HiveBorn => {
                let age = epoch
                    .checked_sub(next.entered_epoch)
                    .ok_or(CounterError::InvalidState)?;
                next.stage = match next.stage {
                    1 if next.counts[0] >= 200 => 2,
                    2 if next.counts[1] >= 200 => 3,
                    2 if age >= 3 => 1,
                    3 if age >= 5 => {
                        if next.counts[1] >= 200 {
                            2
                        } else {
                            1
                        }
                    }
                    stage => stage,
                };
                next.counts = [0; 3];
            }
            CounterKind::Deeplings => {
                if p.deepling_points == 0 {
                    return Err(CounterError::MissingParameter);
                }
                if next.stage == 1 && next.counts[0] >= p.deepling_points {
                    next.stage = 2;
                    next.counts = [0; 3];
                } else if next.stage == 2 && next.condition {
                    next.stage = 3;
                    next.condition = false;
                } else if next.stage == 3 && epoch - next.entered_epoch >= 5 {
                    next.stage = 1;
                    next.counts = [0; 3];
                }
            }
            CounterKind::Overhunting => {
                if p.deer_kills == 0 || p.wolf_recovery == 0 {
                    return Err(CounterError::MissingParameter);
                }
                if next.stage == 1 && next.counts[0] >= p.deer_kills {
                    next.stage = 2;
                    next.counts = [0; 3];
                } else if next.stage == 2 && next.counts[1] >= p.wolf_recovery {
                    next.stage = 1;
                    next.counts = [0; 3];
                } else {
                    next.counts = [0; 3];
                }
            }
            CounterKind::MagesTower if next.stage == 4 => {
                next.stage = 1;
            }
            CounterKind::FireEarth => {
                next.stage = u8::from(earth_active);
                next.counts = [0; 3];
            }
            _ => {}
        }
        self.finish(next, kind)
    }
}
fn initial_occurrence_generation() -> u64 {
    1
}

fn add(counter: &mut u64, amount: u64) -> Result<(), CounterError> {
    *counter = counter.checked_add(amount).ok_or(CounterError::Overflow)?;
    Ok(())
}

pub const BINDINGS_JSON: &str =
    include_str!("../../../../content/encounters/world-changes/credited-kills.json");
#[derive(Debug, Deserialize)]
pub struct Bindings {
    pub schema: String,
    pub parameters: Parameters,
    pub definitions: Vec<Definition>,
    pub spawn_rules: Vec<SpawnRule>,
    pub spawn_sources: Vec<SpawnSource>,
}
#[derive(Debug, Deserialize)]
pub struct Definition {
    pub key: String,
    pub kind: CounterKind,
    pub sources: Vec<KillSource>,
}
#[derive(Debug, Deserialize)]
pub struct KillSource {
    pub creature: String,
    pub lane: u8,
    pub area: [i32; 6],
}
impl Bindings {
    pub fn validate(&self) -> Result<(), CounterError> {
        if self.schema != "OTERYN_WORLD_CREDIT_COUNTERS/v1"
            || self.definitions.len() != 8
            || self.parameters.deepling_points == 0
            || self.parameters.deer_kills == 0
            || self.parameters.wolf_recovery == 0
            || self.parameters.deepling_points > 1_000_000_000
            || self.parameters.deer_kills > 1_000_000_000
            || self.parameters.wolf_recovery > 1_000_000_000
        {
            return Err(CounterError::MissingParameter);
        }
        if self.spawn_sources.len() > 64
            || self.spawn_sources.iter().any(|source| {
                source.source.is_empty()
                    || source.source.len() > 128
                    || source.evidence.is_empty()
                    || source.point.cell.floor > 15
                    || source.point.respawn_ms == 0
                    || source.point.respawn_ms > 86_400_000
                    || !source.point.creature.starts_with("oteryn:creature.")
                    || source.point.creature.len() > 128
                    || source.point.direction != "north"
            })
        {
            return Err(CounterError::InvalidState);
        }
        if self.spawn_rules.len() > 256 {
            return Err(CounterError::InvalidState);
        }
        for rule in &self.spawn_rules {
            let [x0, x1, y0, y1, z0, z1] = rule.area;
            if !rule.creature.starts_with("oteryn:creature.")
                || rule.creature.len() > 128
                || !(0..=65535).contains(&x0)
                || !(x0..=65535).contains(&x1)
                || !(0..=65535).contains(&y0)
                || !(y0..=65535).contains(&y1)
                || !(0..=15).contains(&z0)
                || !(z0..=15).contains(&z1)
                || rule
                    .stages
                    .iter()
                    .flatten()
                    .any(|key| !key.starts_with("oteryn:creature.") || key.len() > 128)
            {
                return Err(CounterError::InvalidState);
            }
        }
        for (index, definition) in self.definitions.iter().enumerate() {
            if definition.kind != CounterKind::ALL[index]
                || definition.key != definition.kind.key()
                || definition.sources.len() > 64
            {
                return Err(CounterError::InvalidState);
            }
            for source in &definition.sources {
                let [x0, x1, y0, y1, z0, z1] = source.area;
                if source.lane > 2
                    || source.creature.len() > 128
                    || !source.creature.starts_with("oteryn:creature.")
                    || !(0..=65535).contains(&x0)
                    || !(x0..=65535).contains(&x1)
                    || !(0..=65535).contains(&y0)
                    || !(y0..=65535).contains(&y1)
                    || !(0..=15).contains(&z0)
                    || !(z0..=15).contains(&z1)
                {
                    return Err(CounterError::InvalidState);
                }
            }
        }
        Ok(())
    }
    pub fn candidate(&self, creature: &str) -> bool {
        self.definitions
            .iter()
            .any(|d| d.sources.iter().any(|s| s.creature == creature))
    }
    pub fn credits(&self, creature: &str, position: (i32, i32, i16)) -> Vec<(CounterKind, Credit)> {
        self.definitions
            .iter()
            .filter_map(|d| {
                d.sources
                    .iter()
                    .find(|source| {
                        let [x0, x1, y0, y1, z0, z1] = source.area;
                        source.creature == creature
                            && (x0..=x1).contains(&position.0)
                            && (y0..=y1).contains(&position.1)
                            && (z0..=z1).contains(&i32::from(position.2))
                    })
                    .map(|source| (d.kind, Credit::Kill(source.lane)))
            })
            .collect()
    }
}

/// Exact native point membership is evaluated against its immutable base creature.
/// A missing target suppresses that point; it never creates a raid or a second spawn owner.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnRule {
    pub kind: CounterKind,
    pub creature: String,
    pub area: [i32; 6],
    pub stages: [Option<String>; 6],
}
impl Bindings {
    pub fn spawn_projection<'a>(
        &'a self,
        creature: &str,
        position: (i32, i32, i16),
        stages: [u8; 8],
    ) -> Option<Option<&'a str>> {
        self.spawn_rules
            .iter()
            .find(|rule| {
                let [x0, x1, y0, y1, z0, z1] = rule.area;
                rule.creature == creature
                    && (x0..=x1).contains(&position.0)
                    && (y0..=y1).contains(&position.1)
                    && (z0..=z1).contains(&i32::from(position.2))
            })
            .map(|rule| {
                rule.stages
                    .get(usize::from(stages[rule.kind.index()]))
                    .and_then(Option::as_deref)
            })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnSource {
    pub kind: CounterKind,
    pub source: String,
    pub point: SpawnSourcePoint,
    pub evidence: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnSourcePoint {
    pub cell: SpawnSourceCell,
    pub creature: String,
    pub direction: String,
    pub respawn_ms: u64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnSourceCell {
    pub x: u16,
    pub y: u16,
    pub floor: u8,
}
