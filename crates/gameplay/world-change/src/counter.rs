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
}
impl CounterKind {
    pub const ALL: [Self; 7] = [
        Self::Deeplings,
        Self::DemonWars,
        Self::FireEarth,
        Self::HiveBorn,
        Self::Overhunting,
        Self::FireFeathered,
        Self::MagesTower,
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
            CounterKind::Overhunting => (1..=2).contains(&self.stage),
            CounterKind::FireEarth => self.stage <= 1,
            _ => (1..=3).contains(&self.stage),
        };
        if !valid
            || self.entered_epoch > self.epoch
            || self.phase_version == 0
            || self.credit_generation == 0
            || self.revision == 0
            || (self.deadline_ms.is_some() != (kind == CounterKind::MagesTower && self.stage == 3))
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
        if next.stage != self.stage {
            next.phase_version = self
                .phase_version
                .checked_add(1)
                .ok_or(CounterError::Overflow)?;
            next.entered_epoch = next.epoch;
        }
        if kind == CounterKind::FireFeathered && self.stage == 3 && next.stage == 1 {
            next.credit_generation = self
                .credit_generation
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
        let mut next = self;
        match (kind, credit) {
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
            || self.definitions.len() != 7
            || self.parameters.deepling_points == 0
            || self.parameters.deer_kills == 0
            || self.parameters.wolf_recovery == 0
            || self.parameters.deepling_points > 1_000_000_000
            || self.parameters.deer_kills > 1_000_000_000
            || self.parameters.wolf_recovery > 1_000_000_000
        {
            return Err(CounterError::MissingParameter);
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
