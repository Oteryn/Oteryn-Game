//! Crystal 201189f court_warlock_fight.lua:22-40,134-147,204-278,363-403.
//! A timed source adapter on NativePhaseOwner, sharing its scope and terminal death proof.
use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

pub const RAUBRITTERS: [&str; 4] = [
    "oteryn:creature.poisoned_raubritter",
    "oteryn:creature.frozen_raubritter",
    "oteryn:creature.raging_raubritter",
    "oteryn:creature.energised_raubritter",
];
pub const FALCONS: [&str; 2] = [
    "oteryn:creature.falcon_knight",
    "oteryn:creature.falcon_paladin",
];
pub const WARLOCK: &str = "oteryn:creature.court_warlock";
const ROLES: [Role; 1] = [Role {
    creature: WARLOCK,
    after: 0,
}];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wave {
    pub second: u64,
    pub count: usize,
    pub creatures: &'static [&'static str],
}

/// The donor clock starts three seconds after admission. Its 45-second branch
/// takes precedence over the 22-second branch (floor(45/2)), not a 22.5s interval.
pub fn waves() -> Vec<Wave> {
    let mut waves = vec![Wave {
        second: 3,
        count: 7,
        creatures: &RAUBRITTERS,
    }];
    for second in 1..=315 {
        if second == 315 {
            waves.push(Wave {
                second: second + 3,
                count: 22,
                creatures: &FALCONS,
            });
        } else if second % 45 == 0 || second % 22 == 0 {
            waves.push(Wave {
                second: second + 3,
                count: 7,
                creatures: &RAUBRITTERS,
            });
        }
    }
    waves
}

#[derive(Debug)]
pub struct CourtWarlock {
    phase: NativePhaseOwner,
    start: u64,
    next: usize,
    pending: usize,
    minions: Vec<(ExactActorRef, bool)>,
    spawned: i64,
    killed: i64,
    terminal: Option<ExactActorRef>,
    victory_at: Option<u64>,
    center: MovementLocalPosition,
    low: MovementLocalPosition,
    high: MovementLocalPosition,
}
impl CourtWarlock {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        now: u64,
        low: MovementLocalPosition,
        high: MovementLocalPosition,
        center: MovementLocalPosition,
    ) -> Result<Self, CarrierError> {
        Ok(Self {
            phase: NativePhaseOwner::bind(
                runtime,
                Plan {
                    roles: &ROLES,
                    terminal: 1,
                },
                players,
            )?,
            start: now,
            next: 0,
            pending: 0,
            minions: Vec::new(),
            spawned: 0,
            killed: 0,
            terminal: None,
            victory_at: None,
            center,
            low,
            high,
        })
    }
    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }
    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.terminal == Some(actor) || self.minions.iter().any(|(a, _)| *a == actor)
    }
    pub fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.terminal == Some(actor)
    }
    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }
    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.phase.current(runtime)?;
        if self.terminal == Some(actor) {
            return self.phase.projected_death(runtime, actor);
        }
        runtime.borrow_combat_death().projected_death(actor)?;
        if let Some(index) = self.minions.iter().position(|(a, _)| *a == actor) {
            let (_, raubritter) = self.minions.remove(index);
            if raubritter {
                self.killed += 1;
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub fn in_room(&self, at: MovementLocalPosition) -> bool {
        at.floor == self.low.floor
            && (self.low.x..=self.high.x).contains(&at.x)
            && (self.low.y..=self.high.y).contains(&at.y)
    }
    pub fn exit_due(&self, now: u64) -> bool {
        now.saturating_sub(self.start) >= 1_800_000_000
            || self
                .victory_at
                .is_some_and(|at| now.saturating_sub(at) >= 60_000_000)
    }
    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        for &(actor, _) in &self.minions {
            if runtime.contains_live_creature(actor) {
                runtime.remove_encounter_actor(actor)?;
            }
        }
        if let Some(actor) = self.terminal
            && runtime.contains_live_creature(actor)
        {
            runtime.remove_encounter_actor(actor)?;
        }
        Ok(())
    }
    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        if self.phase.progress.completed() {
            self.victory_at.get_or_insert(now);
            return Ok(());
        }
        let schedule = waves();
        if let Some(wave) = schedule.get(self.next) {
            if now.saturating_sub(self.start) / 1_000_000 < wave.second {
                return Ok(());
            }
            // The actual spawn owner validates every cell and policy. Retain the
            // partial index on a capacity refusal so retries never duplicate spawns.
            while self.pending < wave.count {
                let seed = self
                    .start
                    .wrapping_add((self.next * 31 + self.pending * 17) as u64);
                let creature = wave.creatures[(seed as usize) % wave.creatures.len()];
                let at = self
                    .free_tile(runtime, seed, &admits)
                    .ok_or(CarrierError::PlanConflict)?;
                let health = runtime.companion_policy(creature)?.maximum_health;
                let actors = runtime.commit_quest_creature_spawn(&[(creature, health, at)])?;
                self.minions
                    .push((actors[0], self.next + 1 < schedule.len()));
                if self.next + 1 < schedule.len() {
                    self.spawned += 1;
                }
                self.pending += 1;
            }
            self.pending = 0;
            self.next += 1;
        } else if self.terminal.is_none() {
            let census = runtime.positioned_melee_census()?;
            if census.iter().any(|(a, at, s)| {
                s.is_none() && self.in_room(at.position()) && runtime.contains_live_creature(*a)
            }) {
                return Ok(());
            }
            let at = if runtime.native_summon_cell_free(self.center) && admits(runtime, self.center)
            {
                self.center
            } else {
                self.free_tile(runtime, now, &admits)
                    .ok_or(CarrierError::PlanConflict)?
            };
            let maximum = runtime.companion_policy(WARLOCK)?.maximum_health;
            let health = if self.spawned == 0 {
                maximum
            } else {
                maximum * (self.spawned * 100 - self.killed * 60) / (self.spawned * 100)
            };
            let actors = runtime.commit_quest_creature_spawn(&[(WARLOCK, health.max(1), at)])?;
            self.phase.attach(runtime, 0, actors[0])?;
            self.terminal = Some(actors[0]);
        }
        Ok(())
    }
    fn free_tile(
        &self,
        runtime: &ChannelRuntimeV1,
        seed: u64,
        admits: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Option<MovementLocalPosition> {
        let width = u64::try_from(self.high.x - self.low.x + 1).ok()?;
        let height = u64::try_from(self.high.y - self.low.y + 1).ok()?;
        let size = width * height;
        for offset in 0..size {
            let n = seed.wrapping_add(offset) % size;
            let at = MovementLocalPosition {
                x: self.low.x + i32::try_from(n % width).ok()?,
                y: self.low.y + i32::try_from(n / width).ok()?,
                floor: self.low.floor,
            };
            if runtime.native_summon_cell_free(at) && admits(runtime, at) {
                return Some(at);
            }
        }
        None
    }
}
