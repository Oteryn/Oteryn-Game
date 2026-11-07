//! Crystal201189f moveevent-claustrophobic-inferno-raid.lua13-81 and
//! lib/quests/soul_war.lua126-261,903-907. Fixed source DATA only; current
//! World/Raid/Map owners must independently authorize every physical effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
const fn p(x: i32, y: i32, floor: i16) -> Position {
    Position { x, y, floor }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Raid {
    First,
    Second,
    Third,
}
impl Raid {
    pub const ALL: [Self; 3] = [Self::First, Self::Second, Self::Third];
    pub const fn key(self) -> &'static str {
        match self {
            Self::First => "raid:soul-war.claustrophobic-inferno-1",
            Self::Second => "raid:soul-war.claustrophobic-inferno-2",
            Self::Third => "raid:soul-war.claustrophobic-inferno-3",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|raid| raid.key() == key)
    }
    pub fn from_sand(at: Position) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|raid| spec(*raid).sand.contains(&at))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Species {
    Brachiodemon,
    InfernalDemon,
    InfernalPhantom,
}
impl Species {
    pub const fn native_key(self) -> &'static str {
        match self {
            Self::Brachiodemon => "oteryn:creature.brachiodemon",
            Self::InfernalDemon => "oteryn:creature.infernal_demon",
            Self::InfernalPhantom => "oteryn:creature.infernal_phantom",
        }
    }
}
#[derive(Debug)]
pub struct Spec {
    pub raid: Raid,
    pub zone: [Position; 2],
    pub sand: &'static [Position],
    pub species: [&'static [Position]; 3],
    pub exit: Position,
    /// Donor bare Pos: no coordinate is fabricated for this one source slot.
    pub unresolved_phantom: Option<u8>,
}
impl Spec {
    pub fn contains(&self, at: Position) -> bool {
        let [a, b] = self.zone;
        (a.x..=b.x).contains(&at.x)
            && (a.y..=b.y).contains(&at.y)
            && (a.floor..=b.floor).contains(&at.floor)
    }
    pub fn ingress(&self, from: Position, to: Position) -> bool {
        let rejected_from_y = if self.raid == Raid::Second {
            to.y.checked_sub(1)
        } else {
            to.y.checked_add(1)
        };
        self.sand.contains(&to) && rejected_from_y != Some(from.y)
    }
}

static FIRST: Spec = Spec {
    raid: Raid::First,
    zone: [p(33985, 31053, 9), p(34045, 31077, 9)],
    sand: &[
        p(34012, 31049, 9),
        p(34013, 31049, 9),
        p(34014, 31049, 9),
        p(34015, 31049, 9),
    ],
    species: [
        &[
            p(33994, 31068, 9),
            p(33998, 31062, 9),
            p(34012, 31061, 9),
            p(34006, 31075, 9),
            p(34037, 31072, 9),
            p(34026, 31063, 9),
            p(34034, 31057, 9),
            p(34019, 31067, 9),
            p(34040, 31064, 9),
        ],
        &[p(33994, 31073, 9), p(34015, 31075, 9), p(34034, 31070, 9)],
        &[
            p(34018, 31058, 9),
            p(33997, 31058, 9),
            p(34008, 31066, 9),
            p(34013, 31070, 9),
            p(34037, 31060, 9),
            p(34026, 31073, 9),
            p(34006, 31058, 9),
        ],
    ],
    exit: p(34009, 31083, 9),
    unresolved_phantom: None,
};

static SECOND: Spec = Spec {
    raid: Raid::Second,
    zone: [p(33988, 31042, 10), p(34043, 31068, 10)],
    sand: &[
        p(34012, 31075, 10),
        p(34011, 31075, 10),
        p(34010, 31075, 10),
    ],
    species: [
        &[
            p(33999, 31048, 10),
            p(33996, 31053, 10),
            p(33996, 31061, 10),
            p(34006, 31056, 10),
            p(34010, 31066, 10),
            p(34019, 31068, 10),
            p(34023, 31063, 10),
            p(34013, 31046, 10),
            p(34022, 31045, 10),
            p(34036, 31063, 10),
            p(34035, 31047, 10),
        ],
        &[
            p(34017, 31040, 10),
            p(34001, 31058, 10),
            p(34016, 31056, 10),
            p(34028, 31046, 10),
            p(34030, 31058, 10),
        ],
        &[
            p(33991, 31059, 10),
            p(34005, 31045, 10),
            p(34006, 31051, 10),
            p(34020, 31051, 10),
            p(34036, 31053, 10),
            p(34020, 31072, 10),
            p(34007, 31070, 10),
        ],
    ],
    exit: p(34011, 31028, 10),
    unresolved_phantom: Some(8),
};

static THIRD: Spec = Spec {
    raid: Raid::Third,
    zone: [p(33987, 31043, 11), p(34044, 31076, 11)],
    sand: &[
        p(34009, 31036, 11),
        p(34010, 31036, 11),
        p(34011, 31036, 11),
        p(34012, 31036, 11),
        p(34013, 31036, 11),
        p(34014, 31036, 11),
    ],
    species: [
        &[
            p(34016, 31047, 11),
            p(34007, 31047, 11),
            p(33998, 31051, 11),
            p(33993, 31057, 11),
            p(33995, 31065, 11),
            p(34006, 31069, 11),
            p(34007, 31060, 11),
            p(34020, 31060, 11),
            p(34032, 31058, 11),
            p(34032, 31067, 11),
            p(34039, 31067, 11),
        ],
        &[
            p(33998, 31061, 11),
            p(34012, 31060, 11),
            p(34030, 31063, 11),
        ],
        &[
            p(34001, 31047, 11),
            p(34013, 31064, 11),
            p(34007, 31053, 11),
            p(34023, 31054, 11),
            p(34038, 31053, 11),
        ],
    ],
    exit: p(34014, 31085, 11),
    unresolved_phantom: None,
};

pub fn spec(raid: Raid) -> &'static Spec {
    match raid {
        Raid::First => &FIRST,
        Raid::Second => &SECOND,
        Raid::Third => &THIRD,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Effect {
    Warning {
        phase: u8,
        entry: u16,
        index: u16,
        at: Position,
    },
    Spawn {
        phase: u8,
        entry: u16,
        index: u16,
        species: Species,
        at: Position,
    },
    Advance,
    Reset,
    KickCurrentPlayers,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedEvent {
    raid: Raid,
    id: u64,
    due: u64,
    pub effect: Effect,
}
impl PreparedEvent {
    pub fn due_ms(self) -> u64 {
        self.due
    }
    pub fn source_id(self) -> u64 {
        self.id
    }
    pub fn raid(self) -> Raid {
        self.raid
    }
    pub fn source_valid(self) -> bool {
        if self.id == 0 {
            return false;
        }
        let record = spec(self.raid);
        match self.effect {
            Effect::Warning {
                phase,
                entry,
                index,
                at,
            } => {
                (1..=6).contains(&phase)
                    && record
                        .species
                        .get(usize::from(entry))
                        .and_then(|positions| positions.get(usize::from(index)))
                        == Some(&at)
            }
            Effect::Spawn {
                phase,
                entry,
                index,
                species,
                at,
            } => {
                (1..=6).contains(&phase)
                    && [
                        Species::Brachiodemon,
                        Species::InfernalDemon,
                        Species::InfernalPhantom,
                    ]
                    .get(usize::from(entry))
                        == Some(&species)
                    && record
                        .species
                        .get(usize::from(entry))
                        .and_then(|positions| positions.get(usize::from(index)))
                        == Some(&at)
            }
            Effect::Advance | Effect::Reset | Effect::KickCurrentPlayers => true,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Overflow,
    StaleEvent,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    raid: Raid,
    phase: Option<u8>,
    last_end_seconds: Option<u64>,
    next: u64,
    events: Vec<PreparedEvent>,
}
impl Controller {
    pub fn snapshot(&self) -> Result<Vec<u8>, Refusal> {
        serde_json::to_vec(self).map_err(|_| Refusal::Overflow)
    }
    /// The existing Raid owner stores only this bounded fixed-source DATA.
    /// A decoded state is not a firing, node, map or creature-admission grant.
    pub fn from_snapshot(raid: Raid, bytes: &[u8]) -> Result<Self, Refusal> {
        if bytes.len() > 32768 {
            return Err(Refusal::StaleEvent);
        }
        let state: Self = serde_json::from_slice(bytes).map_err(|_| Refusal::StaleEvent)?;
        let bound = spec(raid).species.iter().map(|p| p.len()).sum::<usize>() * 4 + 2;
        if state.raid != raid
            || state.events.len() > bound
            || state.phase.is_some_and(|phase| !(1..=6).contains(&phase))
        {
            return Err(Refusal::StaleEvent);
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut controls = 0;
        let mut kicks = 0;
        for event in &state.events {
            if event.raid != raid
                || !event.source_valid()
                || event.id > state.next
                || !ids.insert(event.id)
            {
                return Err(Refusal::StaleEvent);
            }
            let valid = match event.effect {
                Effect::Warning { phase, .. } | Effect::Spawn { phase, .. } => {
                    Some(phase) == state.phase
                }
                Effect::Advance => {
                    controls += 1;
                    state.phase.is_some_and(|phase| phase < 6)
                }
                Effect::Reset => {
                    controls += 1;
                    state.phase == Some(6)
                }
                Effect::KickCurrentPlayers => {
                    kicks += 1;
                    true
                }
            };
            if !valid {
                return Err(Refusal::StaleEvent);
            }
        }
        if controls != usize::from(state.phase.is_some()) || kicks > 1 {
            return Err(Refusal::StaleEvent);
        }
        Ok(state)
    }
    pub fn active(&self) -> bool {
        self.phase.is_some()
    }
    pub fn last_end_seconds(&self) -> Option<u64> {
        self.last_end_seconds
    }
    pub fn new(raid: Raid) -> Self {
        Self {
            raid,
            phase: None,
            last_end_seconds: None,
            next: 0,
            events: Vec::new(),
        }
    }
    pub fn start(
        &mut self,
        from: Position,
        to: Position,
        now_ms: u64,
        now_seconds: u64,
    ) -> Result<bool, Refusal> {
        if self.phase.is_some()
            || !spec(self.raid).ingress(from, to)
            || self
                .last_end_seconds
                .is_some_and(|end| now_seconds.saturating_sub(end) < 10)
        {
            return Ok(false);
        }
        let (events, next) = self.stage_events(1, now_ms)?;
        self.events = events;
        self.next = next;
        self.phase = Some(1);
        Ok(true)
    }
    fn stage_events(&self, phase: u8, now_ms: u64) -> Result<(Vec<PreparedEvent>, u64), Refusal> {
        let source = spec(self.raid);
        let count = source
            .species
            .iter()
            .map(|points| points.len())
            .sum::<usize>();
        let capacity = count
            .checked_mul(4)
            .and_then(|value| value.checked_add(2))
            .ok_or(Refusal::Overflow)?;
        let mut events = Vec::new();
        events
            .try_reserve(capacity)
            .map_err(|_| Refusal::Overflow)?;
        // This5s callback is plain addEvent in onReset, outside the Encounter's
        // cancelEvents set, and therefore survives a later source start.
        events.extend(
            self.events
                .iter()
                .filter(|event| event.effect == Effect::KickCurrentPlayers)
                .copied(),
        );
        let mut next = self.next;
        let mut push = |delay, effect| -> Result<(), Refusal> {
            next = next.checked_add(1).ok_or(Refusal::Overflow)?;
            events.push(PreparedEvent {
                raid: self.raid,
                id: next,
                due: now_ms.checked_add(delay).ok_or(Refusal::Overflow)?,
                effect,
            });
            Ok(())
        };
        for (entry, (species, points)) in [
            Species::Brachiodemon,
            Species::InfernalDemon,
            Species::InfernalPhantom,
        ]
        .into_iter()
        .zip(source.species)
        .enumerate()
        {
            let entry = u16::try_from(entry).map_err(|_| Refusal::Overflow)?;
            for (index, &at) in points.iter().enumerate() {
                let index = u16::try_from(index).map_err(|_| Refusal::Overflow)?;
                for delay in [1000, 2000, 3000] {
                    push(
                        delay,
                        Effect::Warning {
                            phase,
                            entry,
                            index,
                            at,
                        },
                    )?;
                }
                push(
                    3000,
                    Effect::Spawn {
                        phase,
                        entry,
                        index,
                        species,
                        at,
                    },
                )?;
            }
        }
        push(
            20_000,
            if phase == 6 {
                Effect::Reset
            } else {
                Effect::Advance
            },
        )?;
        Ok((events, next))
    }
    pub fn next_due(&self) -> Option<u64> {
        self.events.iter().map(|event| event.due).min()
    }
    pub fn prepare(&self, now_ms: u64) -> Option<PreparedEvent> {
        self.events
            .iter()
            .min_by_key(|event| (event.due, event.id))
            .copied()
            .filter(|event| event.due <= now_ms)
    }
    pub fn commit(
        &mut self,
        event: PreparedEvent,
        now_ms: u64,
        now_seconds: u64,
    ) -> Result<(), Refusal> {
        if self.prepare(now_ms) != Some(event) {
            return Err(Refusal::StaleEvent);
        }
        match event.effect {
            Effect::Advance => {
                let phase = self
                    .phase
                    .filter(|phase| *phase < 6)
                    .ok_or(Refusal::StaleEvent)?
                    + 1;
                let (events, next) = self.stage_events(phase, now_ms)?;
                self.events = events;
                self.next = next;
                self.phase = Some(phase);
            }
            Effect::Reset => {
                if self.phase != Some(6) {
                    return Err(Refusal::StaleEvent);
                }
                let id = self.next.checked_add(1).ok_or(Refusal::Overflow)?;
                let due = now_ms.checked_add(5000).ok_or(Refusal::Overflow)?;
                let mut events = Vec::new();
                events.try_reserve(2).map_err(|_| Refusal::Overflow)?;
                events.extend(
                    self.events
                        .iter()
                        .filter(|held| held.effect == Effect::KickCurrentPlayers)
                        .copied(),
                );
                events.push(PreparedEvent {
                    raid: self.raid,
                    id,
                    due,
                    effect: Effect::KickCurrentPlayers,
                });
                self.events = events;
                self.next = id;
                self.phase = None;
                self.last_end_seconds = Some(now_seconds);
            }
            _ => {
                let index = self
                    .events
                    .iter()
                    .position(|held| *held == event)
                    .ok_or(Refusal::StaleEvent)?;
                self.events.remove(index);
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_claustrophobic_literal_three_source_specs_preserve_unknown_and_outside_positions()
    -> Result<(), Refusal> {
        assert_eq!(
            spec(Raid::First).species.map(|points| points.len()),
            [9, 3, 7]
        );
        assert_eq!(
            spec(Raid::Second).species.map(|points| points.len()),
            [11, 5, 7]
        );
        assert_eq!(
            spec(Raid::Third).species.map(|points| points.len()),
            [11, 3, 5]
        );
        assert_eq!(spec(Raid::Second).unresolved_phantom, Some(8));
        assert!(
            !spec(Raid::Second).contains(p(34020, 31072, 10)),
            "literal source position is not clipped to the zone"
        );
        assert_eq!(spec(Raid::First).exit, p(34009, 31083, 9));
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_source_direction_cooldown_and_active_start_are_exact()
    -> Result<(), Refusal> {
        for raid in [Raid::First, Raid::Second, Raid::Third] {
            let to = spec(raid).sand[0];
            let direction = if raid == Raid::Second { 1 } else { -1 };
            let mut owner = Controller::new(raid);
            let wrong = Position {
                y: to.y - direction,
                ..to
            };
            let right = Position {
                y: to.y + direction,
                ..to
            };
            assert!(!owner.start(wrong, to, 0, 100)?);
            assert!(
                owner.start(right, to, 0, 100)?,
                "actual source ingress starts the encounter"
            );
            assert!(
                !owner.start(right, to, 1, 100)?,
                "source Encounter:start refuses while active"
            );
            while let Some(event) = owner.prepare(120_000) {
                let reset = event.effect == Effect::Reset;
                owner.commit(event, 120_000, 220)?;
                if reset {
                    break;
                }
            }
        }
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_source_phase_delay_uses_actual_callback_execution_and_pending_receipts()
    -> Result<(), Refusal> {
        let mut owner = Controller::new(Raid::First);
        let to = spec(Raid::First).sand[0];
        assert!(owner.start(Position { y: to.y - 1, ..to }, to, 0, 100)?);
        let pending = owner.prepare(100_000).ok_or(Refusal::StaleEvent)?;
        assert_eq!(
            owner.prepare(100_000),
            Some(pending),
            "unacknowledged callback keeps its source identity"
        );
        owner.commit(pending, 100_000, 200)?;
        assert_eq!(
            owner.commit(pending, 100_000, 200),
            Err(Refusal::StaleEvent)
        );
        let mut spawns = 0;
        while let Some(event) = owner.prepare(100_000) {
            if matches!(event.effect, Effect::Spawn { .. }) {
                spawns += 1;
            }
            owner.commit(event, 100_000, 200)?;
        }
        assert_eq!(
            spawns, 19,
            "only stage1 spawns after the delayed pass, no fictitious catchup stages"
        );
        assert_eq!(
            owner.next_due(),
            Some(101_000),
            "next stage begins at actual advance100s"
        );
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_reset_kick_and_ten_second_restart_use_separate_source_events()
    -> Result<(), Refusal> {
        let mut owner = Controller::new(Raid::First);
        let to = spec(Raid::First).sand[0];
        let from = Position { y: to.y - 1, ..to };
        assert!(owner.start(from, to, 0, 100)?);
        let mut reset = 0;
        let mut spawn = 0;
        for now in 0..=120 {
            while let Some(event) = owner.prepare(now * 1000) {
                if event.effect == Effect::Reset {
                    reset += 1;
                }
                if matches!(event.effect, Effect::Spawn { .. }) {
                    spawn += 1;
                }
                owner.commit(event, now * 1000, 100 + now)?;
            }
        }
        assert_eq!((spawn, reset), (114, 1));
        assert_eq!(owner.next_due(), Some(125_000));
        assert!(!owner.start(from, to, 129_000, 229)?);
        assert!(
            owner.start(from, to, 130_000, 230)?,
            "equality10 is allowed"
        );
        let kick = owner.prepare(130_000).ok_or(Refusal::StaleEvent)?;
        assert_eq!(
            kick.effect,
            Effect::KickCurrentPlayers,
            "global5s kick is not canceled by a later source start"
        );
        owner.commit(kick, 130_000, 230)?;
        assert_eq!(owner.next_due(), Some(131_000));
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_nominal120_does_not_replace_actual_sixth_stage_reset()
    -> Result<(), Refusal> {
        let mut owner = Controller::new(Raid::First);
        let to = spec(Raid::First).sand[0];
        assert!(owner.start(Position { y: to.y - 1, ..to }, to, 0, 100)?);
        for now in [100_000, 120_000] {
            while let Some(event) = owner.prepare(now) {
                assert_ne!(
                    event.effect,
                    Effect::Reset,
                    "no independent120s reset exists in the pinned source"
                );
                owner.commit(event, now, 100 + now / 1000)?;
            }
        }
        assert_eq!(owner.phase, Some(3));
        assert_eq!(owner.next_due(), Some(121_000));
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_executed_late_reset_cancels_all_stage_handles() -> Result<(), Refusal> {
        let mut owner = Controller::new(Raid::First);
        let to = spec(Raid::First).sand[0];
        assert!(owner.start(Position { y: to.y - 1, ..to }, to, 0, 100)?);
        for now in 0..=100 {
            while let Some(event) = owner.prepare(now * 1000) {
                owner.commit(event, now * 1000, 100 + now)?;
            }
        }
        assert_eq!(owner.phase, Some(6));
        let old = owner
            .events
            .iter()
            .find(|event| matches!(event.effect, Effect::Spawn { phase: 6, .. }))
            .copied()
            .ok_or(Refusal::StaleEvent)?;
        let mut reset = 0;
        while let Some(event) = owner.prepare(1_000_000) {
            let is_reset = event.effect == Effect::Reset;
            owner.commit(event, 1_000_000, 1100)?;
            if is_reset {
                reset += 1;
                break;
            }
        }
        assert_eq!(reset, 1);
        assert_eq!(owner.phase, None);
        assert_eq!(owner.commit(old, 1_000_000, 1100), Err(Refusal::StaleEvent));
        assert_eq!(owner.prepare(1_000_000), None);
        assert_eq!(owner.next_due(), Some(1_005_000));
        let kick = owner.prepare(1_005_000).ok_or(Refusal::StaleEvent)?;
        assert_eq!(kick.effect, Effect::KickCurrentPlayers);
        owner.commit(kick, 1_005_000, 1105)?;
        assert_eq!(owner.next_due(), None);
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_receipt_cannot_advance_another_raid_with_equal_wave_count()
    -> Result<(), Refusal> {
        let mut first = Controller::new(Raid::First);
        let mut third = Controller::new(Raid::Third);
        for owner in [&mut first, &mut third] {
            let to = spec(owner.raid).sand[0];
            assert!(owner.start(Position { y: to.y - 1, ..to }, to, 0, 100)?);
            while let Some(event) = owner.prepare(3000) {
                owner.commit(event, 3000, 103)?;
            }
        }
        let foreign = first.prepare(20_000).ok_or(Refusal::StaleEvent)?;
        assert_eq!(foreign.effect, Effect::Advance);
        assert_eq!(
            third.commit(foreign, 20_000, 120),
            Err(Refusal::StaleEvent),
            "an equal77th event does not belong to another source raid"
        );
        assert_eq!(third.phase, Some(1));
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_persisted_cursor_preserves_exact_next_event_and_source_identity()
    -> Result<(), Refusal> {
        for raid in Raid::ALL {
            let mut owner = Controller::new(raid);
            let to = spec(raid).sand[0];
            let from = Position {
                y: if raid == Raid::Second {
                    to.y + 1
                } else {
                    to.y - 1
                },
                ..to
            };
            assert!(owner.start(from, to, 1000, 100)?);
            let event = owner.prepare(2000).ok_or(Refusal::StaleEvent)?;
            let bytes = owner.snapshot()?;
            let mut retained = Controller::from_snapshot(raid, &bytes)?;
            assert_eq!(retained.prepare(2000), Some(event));
            retained.commit(event, 2000, 101)?;
            owner.commit(event, 2000, 101)?;
            assert_eq!(retained.snapshot()?, owner.snapshot()?);
            let other = if raid == Raid::First {
                Raid::Third
            } else {
                Raid::First
            };
            assert!(Controller::from_snapshot(other, &bytes).is_err());
        }
        Ok(())
    }
    #[test]
    fn srx3_claustrophobic_persisted_cursor_refuses_duplicate_cues_and_unknown_coordinates()
    -> Result<(), Refusal> {
        let raid = Raid::First;
        let to = spec(raid).sand[0];
        let mut owner = Controller::new(raid);
        assert!(owner.start(Position { y: to.y - 1, ..to }, to, 0, 100)?);
        let mut encoded: serde_json::Value =
            serde_json::from_slice(&owner.snapshot()?).map_err(|_| Refusal::Overflow)?;
        let events = encoded["events"].as_array_mut().ok_or(Refusal::Overflow)?;
        events.push(events[0].clone());
        assert!(
            Controller::from_snapshot(
                raid,
                &serde_json::to_vec(&encoded).map_err(|_| Refusal::Overflow)?
            )
            .is_err()
        );
        let mut encoded: serde_json::Value =
            serde_json::from_slice(&owner.snapshot()?).map_err(|_| Refusal::Overflow)?;
        encoded["events"][0]["effect"]["Warning"]["at"]["x"] = serde_json::json!(1);
        assert!(
            Controller::from_snapshot(
                raid,
                &serde_json::to_vec(&encoded).map_err(|_| Refusal::Overflow)?
            )
            .is_err()
        );
        Ok(())
    }
}
