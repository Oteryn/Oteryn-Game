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
pub enum EventKind {
    Chyllfroest,
    FlowerMonth,
    MummysCurse,
}

impl EventKind {
    pub const ALL: [Self; 3] = [Self::Chyllfroest, Self::FlowerMonth, Self::MummysCurse];

    pub fn key(self) -> &'static str {
        match self {
            Self::Chyllfroest => "chyllfroest",
            Self::FlowerMonth => "flower_month",
            Self::MummysCurse => "mummys_curse",
        }
    }

    /// Annual windows use UTC; daily events use the committed reset timestamp.
    pub fn window(self) -> Option<((u8, u8), (u8, u8))> {
        match self {
            // movements_prohibit_entry.lua:12 includes May 1, not merely April.
            Self::Chyllfroest => Some(((4, 1), (5, 2))),
            Self::FlowerMonth => Some(((6, 1), (7, 1))),
            Self::MummysCurse => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Active,
    Victory,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    Tick,
    Jar(u16),
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
        }
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
            if self.phase == Phase::Active {
                self.phase = Phase::Failure;
            }
            self.observed_ms = now;
            return Ok(false);
        }
        self.observed_ms = now;
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
            (EventKind::MummysCurse | EventKind::Chyllfroest, Input::Tick) => Ok(false),
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
