use std::collections::BTreeMap;

use super::{
    MAX_PARTICIPANTS, MONSTERS_PER_WAVE, SoulpitError, SoulpitStack, TIME_LIMIT_MICROS,
    WaveProgress, WaveTransition, plan_wave, wave_stacks,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EncounterPhase {
    Preparing,
    Fighting,
    Completed,
    TimedOut,
    Abandoned,
    Interrupted,
}

impl EncounterPhase {
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Preparing | Self::Fighting)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoulpitMonster {
    pub stack: SoulpitStack,
    pub is_summon: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterSnapshot {
    pub phase: EncounterPhase,
    pub wave: u8,
    pub started_at_micros: u64,
    pub wave_started_at_micros: u64,
    pub last_observed_at_micros: u64,
    pub surviving_slots: Vec<u8>,
    pub monsters_alive: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EncounterEffect {
    NoChange,
    /// Stable plan until `spawned` acknowledges the writer's seven persistent identities.
    SpawnWave {
        wave: u8,
        stacks: [SoulpitStack; MONSTERS_PER_WAVE],
    },
    WaveStarted {
        wave: u8,
    },
    Completed {
        surviving_slots: Vec<u8>,
    },
    TimedOut,
    Abandoned,
    Interrupted,
}

/// Durable value format; deserialization alone is not admission. Always call `restore` before
/// using one, with the existing carrier's actor bound rather than a new gameplay population cap.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterCheckpoint {
    version: u8,
    phase: EncounterPhase,
    wave: u8,
    started_at_micros: u64,
    wave_started_at_micros: u64,
    last_observed_at_micros: u64,
    original_slots: u8,
    surviving_slots: u8,
    spawned_ids: Option<[Option<u64>; MONSTERS_PER_WAVE]>,
    spawned_waves: BTreeMap<u8, [Option<u64>; MONSTERS_PER_WAVE]>,
    seen_monsters: BTreeMap<u64, (u8, SoulpitMonster)>,
    live_monsters: BTreeMap<u64, SoulpitMonster>,
}

/// Value-only state machine. The owning transaction persists effects/receipts and proves each
/// death/leave from the live actor owner; client payloads never supply these lifecycle events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoulpitEncounter {
    phase: EncounterPhase,
    wave: u8,
    started_at_micros: u64,
    wave_started_at_micros: u64,
    last_observed_at_micros: u64,
    original_slots: u8,
    surviving_slots: u8,
    spawned_ids: Option<[Option<u64>; MONSTERS_PER_WAVE]>,
    spawned_waves: BTreeMap<u8, [Option<u64>; MONSTERS_PER_WAVE]>,
    seen_monsters: BTreeMap<u64, (u8, SoulpitMonster)>,
    live_monsters: BTreeMap<u64, SoulpitMonster>,
}

impl SoulpitEncounter {
    pub fn checkpoint(&self) -> EncounterCheckpoint {
        EncounterCheckpoint {
            version: 1,
            phase: self.phase,
            wave: self.wave,
            started_at_micros: self.started_at_micros,
            wave_started_at_micros: self.wave_started_at_micros,
            last_observed_at_micros: self.last_observed_at_micros,
            original_slots: self.original_slots,
            surviving_slots: self.surviving_slots,
            spawned_ids: self.spawned_ids,
            spawned_waves: self.spawned_waves.clone(),
            seen_monsters: self.seen_monsters.clone(),
            live_monsters: self.live_monsters.clone(),
        }
    }

    pub fn restore(
        checkpoint: EncounterCheckpoint,
        max_monsters: usize,
    ) -> Result<Self, SoulpitError> {
        let valid_slots = (1_u8 << MAX_PARTICIPANTS) - 1;
        if checkpoint.version != 1
            || !(1..=super::WAVE_COUNT).contains(&checkpoint.wave)
            || checkpoint.original_slots == 0
            || checkpoint.original_slots & !valid_slots != 0
            || checkpoint.surviving_slots & !checkpoint.original_slots != 0
            || checkpoint.wave_started_at_micros < checkpoint.started_at_micros
            || checkpoint.last_observed_at_micros < checkpoint.wave_started_at_micros
            || checkpoint.seen_monsters.len() > max_monsters
            || checkpoint.live_monsters.len() > checkpoint.seen_monsters.len()
            || (checkpoint.phase.is_terminal() && !checkpoint.live_monsters.is_empty())
            || (checkpoint.phase == EncounterPhase::Preparing
                && (checkpoint.spawned_ids.is_some() || !checkpoint.live_monsters.is_empty()))
            || (checkpoint.phase == EncounterPhase::Fighting && checkpoint.spawned_ids.is_none())
            || (checkpoint.phase == EncounterPhase::Abandoned && checkpoint.surviving_slots != 0)
            || (checkpoint.phase == EncounterPhase::TimedOut
                && checkpoint.last_observed_at_micros - checkpoint.started_at_micros
                    < TIME_LIMIT_MICROS)
            || (checkpoint.phase == EncounterPhase::Completed
                && (checkpoint.wave != super::WAVE_COUNT
                    || checkpoint.spawned_ids.is_none()
                    || checkpoint.surviving_slots == 0
                    || checkpoint.last_observed_at_micros - checkpoint.wave_started_at_micros
                        < super::WAVE_CHECK_DELAY_MICROS
                    || checkpoint.last_observed_at_micros - checkpoint.started_at_micros
                        >= TIME_LIMIT_MICROS))
        {
            return Err(SoulpitError::InvalidState);
        }
        let mut principals = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (id, (wave, monster)) in &checkpoint.seen_monsters {
            if *id == 0 || *wave == 0 || *wave > checkpoint.wave {
                return Err(SoulpitError::InvalidState);
            }
            if checkpoint.spawned_ids.is_none() && *wave == checkpoint.wave {
                return Err(SoulpitError::InvalidState);
            }
            if !monster.is_summon {
                principals[usize::from(*wave - 1)].push(monster.stack.get());
            }
        }
        let expected_waves =
            usize::from(checkpoint.wave - 1) + usize::from(checkpoint.spawned_ids.is_some());
        if checkpoint.spawned_waves.len() != expected_waves
            || checkpoint.spawned_waves.get(&checkpoint.wave).copied() != checkpoint.spawned_ids
        {
            return Err(SoulpitError::InvalidState);
        }
        let mut acknowledged = std::collections::BTreeSet::new();
        for wave in 1..=checkpoint.wave {
            let actual = &mut principals[usize::from(wave - 1)];
            actual.sort_unstable();
            let Some(ids) = checkpoint.spawned_waves.get(&wave) else {
                if wave != checkpoint.wave || checkpoint.spawned_ids.is_some() || !actual.is_empty()
                {
                    return Err(SoulpitError::InvalidState);
                }
                continue;
            };
            let mut expected = Vec::new();
            for (id, stack) in ids.iter().copied().zip(wave_stacks(wave)?) {
                let Some(id) = id else {
                    continue;
                };
                if id == 0
                    || !acknowledged.insert(id)
                    || checkpoint.seen_monsters.get(&id)
                        != Some(&(
                            wave,
                            SoulpitMonster {
                                stack,
                                is_summon: false,
                            },
                        ))
                {
                    return Err(SoulpitError::InvalidState);
                }
                expected.push(stack.get());
            }
            expected.sort_unstable();
            if *actual != expected {
                return Err(SoulpitError::InvalidState);
            }
        }
        for (id, monster) in &checkpoint.live_monsters {
            if checkpoint.seen_monsters.get(id) != Some(&(checkpoint.wave, *monster)) {
                return Err(SoulpitError::InvalidState);
            }
        }
        Ok(Self {
            phase: checkpoint.phase,
            wave: checkpoint.wave,
            started_at_micros: checkpoint.started_at_micros,
            wave_started_at_micros: checkpoint.wave_started_at_micros,
            last_observed_at_micros: checkpoint.last_observed_at_micros,
            original_slots: checkpoint.original_slots,
            surviving_slots: checkpoint.surviving_slots,
            spawned_ids: checkpoint.spawned_ids,
            spawned_waves: checkpoint.spawned_waves,
            seen_monsters: checkpoint.seen_monsters,
            live_monsters: checkpoint.live_monsters,
        })
    }

    pub fn new(started_at_micros: u64, participant_slots: &[u8]) -> Result<Self, SoulpitError> {
        if participant_slots.is_empty() || participant_slots.len() > MAX_PARTICIPANTS {
            return Err(SoulpitError::InvalidParticipants);
        }
        let mut slots = 0_u8;
        for slot in participant_slots {
            if usize::from(*slot) >= MAX_PARTICIPANTS || slots & (1 << slot) != 0 {
                return Err(SoulpitError::InvalidParticipants);
            }
            slots |= 1 << slot;
        }
        Ok(Self {
            phase: EncounterPhase::Preparing,
            wave: 1,
            started_at_micros,
            wave_started_at_micros: started_at_micros,
            last_observed_at_micros: started_at_micros,
            original_slots: slots,
            surviving_slots: slots,
            spawned_ids: None,
            spawned_waves: BTreeMap::new(),
            seen_monsters: BTreeMap::new(),
            live_monsters: BTreeMap::new(),
        })
    }

    pub fn snapshot(&self) -> EncounterSnapshot {
        EncounterSnapshot {
            phase: self.phase,
            wave: self.wave,
            started_at_micros: self.started_at_micros,
            wave_started_at_micros: self.wave_started_at_micros,
            last_observed_at_micros: self.last_observed_at_micros,
            surviving_slots: self.survivors(),
            monsters_alive: self.live_monsters.len(),
        }
    }

    pub fn monster(&self, monster_id: u64) -> Option<SoulpitMonster> {
        self.live_monsters.get(&monster_id).copied()
    }

    /// Physical events observe the independent kick/last-player terminal rules without
    /// running the global wave-clearance tick (encounters.lua:37-60; soulpit_fight.lua:4-19).
    pub fn observe_terminal(&mut self, now_micros: u64) -> Result<EncounterEffect, SoulpitError> {
        if self.phase.is_terminal() {
            return Ok(EncounterEffect::NoChange);
        }
        self.validate_clock(now_micros)?;
        self.last_observed_at_micros = now_micros;
        if now_micros - self.started_at_micros >= TIME_LIMIT_MICROS {
            self.finish(EncounterPhase::TimedOut);
            return Ok(EncounterEffect::TimedOut);
        }
        if self.surviving_slots == 0 {
            self.finish(EncounterPhase::Abandoned);
            return Ok(EncounterEffect::Abandoned);
        }
        Ok(EncounterEffect::NoChange)
    }

    pub fn advance(&mut self, now_micros: u64) -> Result<EncounterEffect, SoulpitError> {
        let terminal = self.observe_terminal(now_micros)?;
        if self.phase.is_terminal() {
            return Ok(terminal);
        }
        let transition = plan_wave(WaveProgress {
            wave: self.wave,
            encounter_started_at_micros: self.started_at_micros,
            wave_started_at_micros: self.wave_started_at_micros,
            now_micros,
            spawn_applied: self.spawned_ids.is_some(),
            monsters_alive: self.live_monsters.len(),
            participants: self.surviving_slots.count_ones() as u8,
        })?;
        self.last_observed_at_micros = now_micros;
        Ok(match transition {
            WaveTransition::Waiting => EncounterEffect::NoChange,
            WaveTransition::Spawn(stacks) => EncounterEffect::SpawnWave {
                wave: self.wave,
                stacks,
            },
            WaveTransition::NextWave(wave) => {
                self.wave = wave;
                self.wave_started_at_micros = now_micros;
                self.spawned_ids = None;
                self.phase = EncounterPhase::Preparing;
                EncounterEffect::WaveStarted { wave }
            }
            WaveTransition::Completed => {
                self.phase = EncounterPhase::Completed;
                EncounterEffect::Completed {
                    surviving_slots: self.survivors(),
                }
            }
            WaveTransition::TimedOut => {
                self.finish(EncounterPhase::TimedOut);
                EncounterEffect::TimedOut
            }
            WaveTransition::Abandoned => {
                self.finish(EncounterPhase::Abandoned);
                EncounterEffect::Abandoned
            }
        })
    }

    /// Same acknowledged attempts are a no-op; a conflicting retry never replaces creatures.
    /// None records a failed Game.createSoulPitMonster attempt (soulpit_fight.lua:130-134).
    /// Seven failed attempts still start the wave, which clears at the ordinary 4.5s check.
    pub fn spawned(
        &mut self,
        wave: u8,
        monster_ids: [Option<u64>; MONSTERS_PER_WAVE],
        now_micros: u64,
    ) -> Result<bool, SoulpitError> {
        if wave < self.wave {
            return Ok(false);
        }
        if wave != self.wave || self.phase.is_terminal() {
            return Err(SoulpitError::InvalidState);
        }
        if let Some(existing) = self.spawned_ids {
            return if existing == monster_ids {
                Ok(false)
            } else {
                Err(SoulpitError::ConflictingReplay)
            };
        }
        self.validate_clock(now_micros)?;
        if self.surviving_slots == 0
            || now_micros - self.started_at_micros >= TIME_LIMIT_MICROS
            || now_micros - self.wave_started_at_micros < super::SPAWN_DELAY_MICROS
        {
            return Err(SoulpitError::InvalidState);
        }
        for (index, id) in monster_ids.iter().enumerate() {
            let Some(id) = id else {
                continue;
            };
            if *id == 0
                || monster_ids[..index].contains(&Some(*id))
                || self.seen_monsters.contains_key(id)
            {
                return Err(SoulpitError::InvalidMonster);
            }
        }
        let stacks = wave_stacks(wave)?;
        for (id, stack) in monster_ids.into_iter().zip(stacks) {
            let Some(id) = id else {
                continue;
            };
            let monster = SoulpitMonster {
                stack,
                is_summon: false,
            };
            self.live_monsters.insert(id, monster);
            self.seen_monsters.insert(id, (wave, monster));
        }
        self.spawned_ids = Some(monster_ids);
        self.spawned_waves.insert(wave, monster_ids);
        self.phase = EncounterPhase::Fighting;
        self.last_observed_at_micros = now_micros;
        Ok(true)
    }

    /// monster.cpp:1405-1411 copies stack into summons; they block clearance and grant no XP.
    pub fn summoned(
        &mut self,
        wave: u8,
        monster_id: u64,
        stack: SoulpitStack,
    ) -> Result<bool, SoulpitError> {
        if wave < self.wave {
            return Ok(false);
        }
        if wave != self.wave || self.phase != EncounterPhase::Fighting {
            return Err(SoulpitError::InvalidState);
        }
        if monster_id == 0 {
            return Err(SoulpitError::InvalidMonster);
        }
        let monster = SoulpitMonster {
            stack,
            is_summon: true,
        };
        if let Some(existing) = self.seen_monsters.get(&monster_id) {
            return if *existing == (wave, monster) {
                Ok(false)
            } else {
                Err(SoulpitError::ConflictingReplay)
            };
        }
        self.seen_monsters.insert(monster_id, (wave, monster));
        self.live_monsters.insert(monster_id, monster);
        Ok(true)
    }

    pub fn defeated(
        &mut self,
        wave: u8,
        monster_id: u64,
    ) -> Result<Option<SoulpitMonster>, SoulpitError> {
        if wave < self.wave || self.phase.is_terminal() {
            return Ok(None);
        }
        if wave != self.wave || self.phase != EncounterPhase::Fighting {
            return Err(SoulpitError::InvalidState);
        }
        if !self.seen_monsters.contains_key(&monster_id) {
            return Err(SoulpitError::InvalidMonster);
        }
        Ok(self.live_monsters.remove(&monster_id))
    }

    /// Exit/death/logout all remove this survivor. The caller observes terminals in the same transition
    /// to collect the final-player Abandoned effect (soulpit_fight.lua:4-19).
    pub fn leave(&mut self, slot: u8, now_micros: u64) -> Result<bool, SoulpitError> {
        self.validate_slot(slot)?;
        if self.phase.is_terminal() {
            return Ok(false);
        }
        let present = self.surviving_slots & (1 << slot) != 0;
        if !present {
            return Ok(false);
        }
        self.validate_clock(now_micros)?;
        self.surviving_slots &= !(1 << slot);
        self.last_observed_at_micros = now_micros;
        Ok(present)
    }

    /// Donor normal logout saves the arena position; relog can re-enter while another player
    /// keeps the encounter alive (player.cpp:11626; soulpit_fight.lua:10-19).
    pub fn rejoin(&mut self, slot: u8, now_micros: u64) -> Result<bool, SoulpitError> {
        self.validate_slot(slot)?;
        self.validate_clock(now_micros)?;
        if self.phase.is_terminal()
            || self.surviving_slots == 0
            || now_micros - self.started_at_micros >= TIME_LIMIT_MICROS
        {
            return Err(SoulpitError::InvalidState);
        }
        let absent = self.surviving_slots & (1 << slot) == 0;
        if !absent {
            return Ok(false);
        }
        self.surviving_slots |= 1 << slot;
        self.last_observed_at_micros = now_micros;
        Ok(absent)
    }

    /// The donor encounter/timers are process-local (soulpit.lua:28-29); restart does not
    /// recreate a fight or refund its consumed core. Login placement remains the actor owner's job.
    pub fn interrupt(&mut self, now_micros: u64) -> Result<EncounterEffect, SoulpitError> {
        if self.phase.is_terminal() {
            return Ok(EncounterEffect::NoChange);
        }
        self.validate_clock(now_micros)?;
        self.last_observed_at_micros = now_micros;
        self.finish(EncounterPhase::Interrupted);
        Ok(EncounterEffect::Interrupted)
    }

    fn validate_slot(&self, slot: u8) -> Result<(), SoulpitError> {
        if usize::from(slot) >= MAX_PARTICIPANTS || self.original_slots & (1 << slot) == 0 {
            Err(SoulpitError::InvalidParticipants)
        } else {
            Ok(())
        }
    }

    fn validate_clock(&self, now_micros: u64) -> Result<(), SoulpitError> {
        if now_micros < self.last_observed_at_micros {
            return Err(SoulpitError::InvalidClock);
        }
        Ok(())
    }

    fn survivors(&self) -> Vec<u8> {
        (0..5_u8)
            .filter(|slot| self.surviving_slots & (1 << slot) != 0)
            .collect()
    }

    fn finish(&mut self, phase: EncounterPhase) {
        self.phase = phase;
        self.live_monsters.clear();
    }
}
