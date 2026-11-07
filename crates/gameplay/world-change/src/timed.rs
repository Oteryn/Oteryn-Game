//! WEVENTS: deterministic transitions for the existing World event owner.
//! Times are authoritative owner times, never client supplied timestamps.
use serde::{Deserialize, Serialize};

pub const MINUTE: i64 = 60_000;
pub const HOUR: i64 = 60 * MINUTE;

/// The failed draw consumes its slot too; Channels and restarts cannot reroll it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntervalClock {
    last_slot: Option<i64>,
}

impl IntervalClock {
    pub fn check(
        &mut self,
        start: i64,
        end: i64,
        now: i64,
        interval: i64,
        chance_permille: u16,
        draw: u64,
    ) -> Result<bool, &'static str> {
        if start < 0 || end <= start || interval <= 0 || chance_permille > 1000 {
            return Err("invalid interval policy");
        }
        if now < start || now >= end {
            return Ok(false);
        }
        let slot = (now - start) / interval;
        if self.last_slot.is_some_and(|last| slot <= last) {
            return Ok(false);
        }
        self.last_slot = Some(slot);
        Ok(((u128::from(draw) * 1000) >> 64) < u128::from(chance_permille))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Chyllfroest,
    FlowerMonth,
    MummysCurse,
    SpringIntoLife,
    RiseOfDevovorga,
    #[serde(rename = "devovorga_s_essence")]
    DevovorgasEssence,
    Kingsday,
    BankRobbery,
    InsectoidInvasion,
}

impl EventKind {
    pub const ALL: [Self; 9] = [
        Self::Chyllfroest,
        Self::FlowerMonth,
        Self::MummysCurse,
        Self::SpringIntoLife,
        Self::RiseOfDevovorga,
        Self::DevovorgasEssence,
        Self::Kingsday,
        Self::BankRobbery,
        Self::InsectoidInvasion,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Chyllfroest => "chyllfroest",
            Self::FlowerMonth => "flower_month",
            Self::MummysCurse => "mummys_curse",
            Self::SpringIntoLife => "spring_into_life",
            Self::RiseOfDevovorga => "rise_of_devovorga",
            Self::DevovorgasEssence => "devovorga_s_essence",
            Self::Kingsday => "kingsday",
            Self::BankRobbery => "bank_robbery",
            Self::InsectoidInvasion => "insectoid_invasion",
        }
    }

    pub fn catalogue_key(self) -> &'static str {
        if self == Self::MummysCurse {
            "the_mummy_s_curse"
        } else {
            self.key()
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.key() == key || kind.catalogue_key() == key)
    }
    pub fn server_save_window(self) -> bool {
        matches!(self, Self::SpringIntoLife | Self::RiseOfDevovorga)
    }

    /// Annual windows use UTC; daily events use the committed reset timestamp.
    pub fn window(self) -> Option<((u8, u8), (u8, u8))> {
        match self {
            // movements_prohibit_entry.lua:12 includes May 1, not merely April.
            Self::Chyllfroest => Some(((4, 1), (5, 2))),
            Self::FlowerMonth => Some(((6, 1), (7, 1))),
            Self::SpringIntoLife => Some(((4, 16), (4, 23))),
            Self::RiseOfDevovorga => Some(((9, 1), (9, 8))),
            Self::MummysCurse
            | Self::DevovorgasEssence
            | Self::Kingsday
            | Self::BankRobbery
            | Self::InsectoidInvasion => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Active,
    Dormant,
    Victory,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    Tick,
    Jar(u16),
    BeginObject(u16),
    Extinguish(u16),
    Fill(u16),
    Defeat(u8),
    Enter(u8),
    Reward,
    Attend,
    Recover,
    Observe,
    ClaimObject(u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventState {
    pub start_ms: i64,
    pub end_ms: i64,
    pub observed_ms: i64,
    pub phase: Phase,
    pub wave: u32,
    pub objects: u8,
    pub boss_spawns: u8,
    pub wall_opened_ms: Option<i64>,
    #[serde(default)]
    pub clock: IntervalClock,
    #[serde(default)]
    pub lifecycle: Box<crate::lifecycle::LifecycleState>,
}

impl EventState {
    pub fn new(start_ms: i64, end_ms: i64) -> Self {
        Self {
            start_ms,
            end_ms,
            observed_ms: start_ms,
            phase: Phase::Active,
            wave: 0,
            objects: 0,
            boss_spawns: 0,
            wall_opened_ms: None,
            clock: IntervalClock::default(),
            lifecycle: Box::default(),
        }
    }

    pub fn for_kind(kind: EventKind, start: i64, end: i64) -> Result<Self, &'static str> {
        if start < 0 || end <= start {
            return Err("invalid event window");
        }
        let mut state = Self::new(start, end);
        if matches!(
            kind,
            EventKind::SpringIntoLife
                | EventKind::RiseOfDevovorga
                | EventKind::DevovorgasEssence
                | EventKind::Kingsday
                | EventKind::BankRobbery
                | EventKind::InsectoidInvasion
        ) {
            state.lifecycle.generation = 1;
        }
        if kind == EventKind::SpringIntoLife {
            for member in 1..=2 {
                state.lifecycle.objects.insert(member, Default::default());
            }
        }
        if kind == EventKind::InsectoidInvasion {
            for member in 1..=3 {
                let mut object = crate::objects::SharedObjectState::default();
                object.ignite()?;
                state.lifecycle.objects.insert(member, object);
            }
            state.lifecycle.objects.insert(0, Default::default());
        }
        if kind == EventKind::BankRobbery {
            state.lifecycle.objects.insert(0, Default::default());
            state.phase = Phase::Dormant;
        }
        Ok(state)
    }
    pub fn object_ready(&self, member: u16, now: i64) -> bool {
        self.lifecycle.ready(member, now)
    }
    pub fn note_spawn(&mut self, member: u16, placed: bool) -> Result<bool, &'static str> {
        self.lifecycle.note_spawn(member, placed)
    }

    /// A false result is a valid no-op, not participation. Wrong event inputs fail.
    pub fn advance(
        &mut self,
        kind: EventKind,
        now: i64,
        input: Input,
        draw: u64,
    ) -> Result<bool, &'static str> {
        if now < self.observed_ms || self.end_ms <= self.start_ms {
            return Err("invalid event clock");
        }
        if now >= self.end_ms {
            if self.phase != Phase::Victory && self.phase != Phase::Failure {
                self.phase = Phase::Failure;
            }
            self.observed_ms = now;
            return Ok(false);
        }
        self.observed_ms = now;
        if input == Input::Observe {
            return Ok(matches!(self.phase, Phase::Active | Phase::Victory));
        }
        match (kind, input) {
            (EventKind::FlowerMonth, Input::Tick) => {
                let next = u32::try_from((now - self.start_ms) / (15 * MINUTE) + 1)
                    .map_err(|_| "wave overflow")?;
                let changed =
                    self.clock
                        .check(self.start_ms, self.end_ms, now, 15 * MINUTE, 1000, draw)?;
                self.wave = next;
                Ok(changed)
            }
            (EventKind::SpringIntoLife, Input::BeginObject(member))
                if self.phase == Phase::Active || self.phase == Phase::Victory =>
            {
                self.lifecycle.begin(member, now, HOUR)
            }
            (EventKind::SpringIntoLife, Input::Tick) => {
                if !self.lifecycle.successful_spawns.is_empty() {
                    self.phase = Phase::Victory;
                }
                Ok(false)
            }
            (EventKind::RiseOfDevovorga, Input::Tick) => {
                if self.phase != Phase::Active {
                    return Ok(false);
                }
                if self.lifecycle.stage == 0
                    && now
                        >= self
                            .start_ms
                            .checked_add(24 * HOUR)
                            .ok_or("clock exhausted")?
                {
                    self.lifecycle.next_generation()?;
                    self.lifecycle.stage = 1;
                    return Ok(true);
                }
                if self.lifecycle.stage == 1
                    && self
                        .lifecycle
                        .first_defeat_ms
                        .is_some_and(|at| now - at >= 5 * MINUTE)
                {
                    self.lifecycle.next_generation()?;
                    self.lifecycle.failed_attempts = self
                        .lifecycle
                        .failed_attempts
                        .checked_add(1)
                        .ok_or("attempt count exhausted")?;
                    return Ok(true);
                }
                Ok(false)
            }
            (EventKind::RiseOfDevovorga, Input::Defeat(member)) if self.phase == Phase::Active => {
                let (changed, victory) = self.lifecycle.defeat(member, now)?;
                if victory {
                    self.phase = Phase::Victory;
                }
                Ok(changed)
            }
            (EventKind::Kingsday, Input::Tick)
                if matches!(self.phase, Phase::Active | Phase::Victory) =>
            {
                let first = self.start_ms.checked_add(HOUR).ok_or("clock exhausted")?;
                if first >= self.end_ms {
                    return Ok(false);
                }
                let changed = self
                    .clock
                    .check(first, self.end_ms, now, HOUR, 1000, draw)?;
                if changed {
                    self.wave = self.wave.checked_add(1).ok_or("wave exhausted")?;
                }
                Ok(changed)
            }
            (EventKind::BankRobbery, Input::Tick) if self.phase == Phase::Dormant => {
                let first = self.start_ms.checked_add(HOUR).ok_or("clock exhausted")?;
                if first >= self.end_ms {
                    return Ok(false);
                }
                let changed = self.clock.check(first, self.end_ms, now, HOUR, 10, draw)?;
                if changed {
                    self.phase = Phase::Active;
                    self.wave = 1;
                    self.lifecycle.begin(0, now, 0)?;
                }
                Ok(changed)
            }
            (EventKind::BankRobbery, Input::ClaimObject(0)) if self.phase == Phase::Active => self
                .lifecycle
                .objects
                .get_mut(&0)
                .ok_or("missing recovery bag")?
                .claim(now),
            (EventKind::Kingsday, Input::Defeat(_) | Input::Attend)
                if matches!(self.phase, Phase::Active | Phase::Victory) =>
            {
                self.phase = Phase::Victory;
                Ok(true)
            }
            (EventKind::BankRobbery, Input::Recover) if self.phase == Phase::Active => {
                self.phase = Phase::Victory;
                Ok(true)
            }
            (EventKind::InsectoidInvasion, Input::Extinguish(member))
                if self.phase == Phase::Active =>
            {
                let Some(object) = self.lifecycle.objects.get(&member).filter(|_| member != 0)
                else {
                    return Err("unknown fire");
                };
                if !object.lit()
                    || self
                        .lifecycle
                        .objects
                        .get(&0)
                        .is_none_or(|bucket| bucket.units() == 0)
                {
                    return Ok(false);
                }
                // Stage both changes so exhaustion cannot consume water without
                // extinguishing the corresponding shared fire.
                let mut objects = self.lifecycle.objects.clone();
                objects.get_mut(&0).ok_or("missing bucket")?.take(1)?;
                let changed = objects
                    .get_mut(&member)
                    .ok_or("unknown fire")?
                    .extinguish()?;
                self.lifecycle.objects = objects;
                if changed
                    && self
                        .lifecycle
                        .objects
                        .iter()
                        .filter(|(id, _)| **id != 0)
                        .all(|(_, o)| !o.lit())
                {
                    self.phase = Phase::Victory;
                }
                Ok(changed)
            }
            (EventKind::InsectoidInvasion, Input::Fill(0)) if self.phase == Phase::Active => self
                .lifecycle
                .objects
                .get_mut(&0)
                .ok_or("missing bucket")?
                .add(1, 3),
            (EventKind::DevovorgasEssence, Input::Enter(0..=4)) if self.phase == Phase::Active => {
                Ok(true)
            }
            (
                EventKind::MummysCurse
                | EventKind::Chyllfroest
                | EventKind::DevovorgasEssence
                | EventKind::Kingsday
                | EventKind::BankRobbery
                | EventKind::InsectoidInvasion,
                Input::Tick,
            ) => Ok(false),
            (EventKind::MummysCurse, Input::Jar(aid)) => {
                if self.phase != Phase::Active || aid != 50006 + u16::from(self.objects) {
                    return Ok(false);
                }
                // Donor: first four jars have a 1/10 chance; last jar is certain.
                if self.objects < 4 && draw >= u64::MAX / 10 {
                    return Ok(false);
                }
                self.objects += 1;
                if self.objects == 5 {
                    self.phase = Phase::Victory;
                    self.boss_spawns = 1;
                    self.wall_opened_ms = Some(now);
                }
                Ok(true)
            }
            _ => Err("input does not belong to event"),
        }
    }

    pub fn flower_area(&self) -> Option<usize> {
        (self.wave > 0 && self.phase == Phase::Active)
            .then(|| usize::try_from((self.wave - 1) % 4).unwrap_or(0))
    }

    pub fn reward_admitted(&self, credited: u64, claimed: bool) -> bool {
        self.phase == Phase::Victory && credited > 0 && !claimed
    }

    /// Donor createWall delay: 61000 + 6*20*1800 = 277000 ms;
    /// the original 3514 barrier is separately restored at twenty minutes.
    pub fn mummy_walls(&self, now: i64) -> (bool, bool) {
        self.wall_opened_ms.map_or((true, false), |opened| {
            (now - opened >= 20 * MINUTE, now - opened >= 277_000)
        })
    }
}
