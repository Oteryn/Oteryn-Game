//! Crystal201189f goshnars_malice.lua:139-147 and lib/quests/soul_war.lua:979-1147.
use crate::bosses::rotten_blood::RoomPosition;
use oteryn_gameplay_foundation::foundation::ThinkOccurrence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaliceEffect {
    SpawnCage,
    RemoveCage,
    BuffMalice,
    CreateWhiteTiles,
    RevertTilesAndDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaliceRefusal {
    Capacity,
    DeadlineOverflow,
    Allocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CageTimerKind {
    Spawn,
    Remove,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CageTimer {
    due: u64,
    kind: CageTimerKind,
}
// Source room duration is20min and each next Cage birth waits23s. These
// bounds retain every original death/timer, refusing before a new claim.
const MAX_CAGE_TIMERS: usize = 128;
const MAX_CAGE_DEATHS: usize = 64;

/// The source closure's timers, scoped to the accepted party Instance (D26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaliceRoom {
    accumulated_ms: u64,
    last_think: Option<ThinkOccurrence>,
    cage_timers: Vec<CageTimer>,
    cage_deaths: Vec<[u8; 32]>,
    white_revert: Option<u64>,
}
impl MaliceRoom {
    /// Boss birth and lever use are separate source callbacks.
    pub fn at_birth(_now_ms: u64) -> Self {
        Self {
            accumulated_ms: 0,
            last_think: None,
            cage_timers: Vec::new(),
            cage_deaths: Vec::new(),
            white_revert: None,
        }
    }
    pub fn lever_pulled_with_elapsed(&mut self, now_ms: u64, elapsed_ms: u64) {
        if let Some(due) = now_ms.checked_add(23_000_u64.saturating_sub(elapsed_ms)) {
            self.cage_timers.push(CageTimer {
                due,
                kind: CageTimerKind::Spawn,
            });
        }
    }
    pub fn entered(now_ms: u64) -> Self {
        Self::entered_with_elapsed(now_ms, 0)
    }
    /// Recovery preserves the original lever deadline even when this process's
    /// monotonic clock is younger than the durable activity. Overdue work runs
    /// on the first current owner pass; subsequent addEvent delays start there.
    pub fn entered_with_elapsed(now_ms: u64, elapsed_ms: u64) -> Self {
        let mut room = Self::at_birth(now_ms);
        room.lever_pulled_with_elapsed(now_ms, elapsed_ms);
        room
    }
    pub fn next_due_ms(&self) -> Option<u64> {
        self.cage_timers
            .iter()
            .map(|timer| timer.due)
            .chain(self.white_revert)
            .min()
    }
    pub fn on_think(
        &mut self,
        occurrence: ThinkOccurrence,
        interval_ms: u64,
        now_ms: u64,
    ) -> Vec<MaliceEffect> {
        if self.last_think.is_some_and(|last| {
            last.actor == occurrence.actor && last.sequence >= occurrence.sequence
        }) {
            return Vec::new();
        }
        let Some(accumulated) = self.accumulated_ms.checked_add(interval_ms) else {
            return Vec::new();
        };
        self.last_think = Some(occurrence);
        self.accumulated_ms = accumulated;
        if accumulated < 40_000 {
            return Vec::new();
        }
        self.accumulated_ms = 0;
        self.white_revert = now_ms.checked_add(3_000);
        vec![MaliceEffect::CreateWhiteTiles]
    }
    /// The copied SoulCageDeath callback claims its original death once.
    pub fn cage_died(&mut self, death: [u8; 32], now_ms: u64) -> Result<bool, MaliceRefusal> {
        if self.cage_deaths.contains(&death) {
            return Ok(false);
        }
        if self.cage_deaths.len() >= MAX_CAGE_DEATHS || self.cage_timers.len() >= MAX_CAGE_TIMERS {
            return Err(MaliceRefusal::Capacity);
        }
        let due = now_ms
            .checked_add(23_000)
            .ok_or(MaliceRefusal::DeadlineOverflow)?;
        self.cage_deaths
            .try_reserve(1)
            .map_err(|_| MaliceRefusal::Allocation)?;
        self.cage_timers
            .try_reserve(1)
            .map_err(|_| MaliceRefusal::Allocation)?;
        self.cage_deaths.push(death);
        self.cage_timers.push(CageTimer {
            due,
            kind: CageTimerKind::Spawn,
        });
        Ok(true)
    }
    /// Cage presence is read at the delayed callback's execution, never inferred
    /// from a prior successful create or a kill counter.
    pub fn advance(
        &mut self,
        now_ms: u64,
        cage_exists: bool,
        boss_exists: bool,
    ) -> Vec<MaliceEffect> {
        self.advance_through(now_ms, now_ms, cage_exists, boss_exists)
    }
    /// Select only an original eligible deadline. New addEvent delays begin at
    /// the actual callback execution, even when the selected work is overdue.
    pub fn advance_through(
        &mut self,
        through_ms: u64,
        now_ms: u64,
        cage_exists: bool,
        boss_exists: bool,
    ) -> Vec<MaliceEffect> {
        let eligible = through_ms.min(now_ms);
        // One actual callback per call: the owner applies its effects and
        // re-reads the CURRENT named Cage before executing the next callback.
        let mut effects = Vec::new();
        let next = self
            .cage_timers
            .iter()
            .enumerate()
            .filter(|(_, timer)| timer.due <= eligible)
            .min_by_key(|(index, timer)| (timer.due, *index))
            .map(|(index, timer)| (index, *timer));
        // For equal deadlines a Cage callback (23s/40s delay) was registered
        // before the white callback (10s delay). Within Cage callbacks the Vec
        // retains actual insertion order, including older removal handles.
        if let Some((index, timer)) = next
            && self
                .white_revert
                .is_none_or(|white| timer.due <= white || white > eligible)
        {
            self.cage_timers.remove(index);
            match timer.kind {
                CageTimerKind::Spawn if !cage_exists => {
                    effects.push(MaliceEffect::SpawnCage);
                    // Removal is scheduled even when createMonster returns nil.
                    if let Some(due) = now_ms.checked_add(40_000) {
                        self.cage_timers.push(CageTimer {
                            due,
                            kind: CageTimerKind::Remove,
                        });
                    }
                }
                CageTimerKind::Remove if cage_exists => {
                    effects.push(MaliceEffect::RemoveCage);
                    if let Some(due) = now_ms.checked_add(23_000) {
                        self.cage_timers.push(CageTimer {
                            due,
                            kind: CageTimerKind::Spawn,
                        });
                    }
                    if boss_exists {
                        effects.push(MaliceEffect::BuffMalice);
                    }
                }
                _ => {}
            }
        } else if self.white_revert.is_some_and(|due| due <= eligible) {
            self.white_revert = None;
            effects.push(MaliceEffect::RevertTilesAndDamage);
        }
        effects
    }
}

/// Preserve the donor's actual four groups (the declared target is eleven but
/// the while condition creates up to twelve). Drawing is supplied by the owner.
pub fn select_white_cells(
    center: RoomPosition,
    mut draw: impl FnMut(usize) -> Option<usize>,
) -> Option<Vec<RoomPosition>> {
    let mut positions = Vec::with_capacity(225);
    for x in center.x.checked_sub(7)?..=center.x.checked_add(7)? {
        for y in center.y.checked_sub(7)?..=center.y.checked_add(7)? {
            positions.push(RoomPosition {
                x,
                y,
                floor: center.floor,
            });
        }
    }
    let choose = |draw: &mut dyn FnMut(usize) -> Option<usize>, bound: usize| {
        let value = draw(bound)?;
        (value < bound).then_some(value)
    };
    let mut result = Vec::with_capacity(12);
    for _ in 0..4 {
        if positions.is_empty() {
            break;
        }
        let start = positions.remove(choose(&mut draw, positions.len())?);
        result.push(start);
        let mut last = start;
        let mut directions = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, -1),
            (-1, 1),
            (1, -1),
        ];
        for _ in 0..2 {
            for i in (1..directions.len()).rev() {
                let other = choose(&mut draw, i + 1)?;
                directions.swap(i, other);
            }
            let next = directions.iter().find_map(|(dx, dy)| {
                let position = RoomPosition {
                    x: last.x + dx,
                    y: last.y + dy,
                    floor: last.floor,
                };
                positions
                    .iter()
                    .position(|candidate| candidate == &position)
            });
            let Some(index) = next else {
                break;
            };
            last = positions.remove(index);
            result.push(last);
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bosses::rotten_blood::RoomPosition;
    #[test]
    fn srx3_malice_actual_birth_does_not_start_lever_timers() {
        let mut room = MaliceRoom::at_birth(100);
        assert_eq!(room.next_due_ms(), None, "onSpawn is not source onUseExtra");
        assert!(room.advance(60_000, false, true).is_empty());
        room.lever_pulled_with_elapsed(60_000, 5_000);
        assert_eq!(room.next_due_ms(), Some(78_000));
    }
    #[test]
    fn srx3_malice_cage_source_timer_stops_when_the_cage_was_killed() {
        let mut room = MaliceRoom::entered(100);
        assert!(room.advance(23_099, false, true).is_empty());
        assert_eq!(
            room.advance(23_100, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert!(
            room.advance(63_100, false, true).is_empty(),
            "source callback sees no Cage and schedules no successor"
        );
        assert!(room.advance(200_000, false, true).is_empty());
    }
    #[test]
    fn srx3_malice_cage_survival_buffs_and_restarts_exact_source_delays() {
        let mut room = MaliceRoom::entered(0);
        assert_eq!(
            room.advance(23_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert_eq!(
            room.advance(63_000, true, true),
            vec![MaliceEffect::RemoveCage, MaliceEffect::BuffMalice]
        );
        assert!(room.advance(85_999, false, true).is_empty());
        assert_eq!(
            room.advance(86_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
    }
    #[test]
    fn srx3_malice_recovered_entry_keeps_original_cage_deadline() {
        let mut pending = MaliceRoom::entered_with_elapsed(500, 20_000);
        assert!(pending.advance(3_499, false, true).is_empty());
        assert_eq!(
            pending.advance(3_500, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        let mut overdue = MaliceRoom::entered_with_elapsed(500, 30_000);
        assert_eq!(
            overdue.advance(500, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        // The successor is relative to actual callback execution, not a replay
        // of callbacks which never ran while the owner was unavailable.
        assert!(overdue.advance(40_499, true, true).is_empty());
        assert_eq!(
            overdue.advance(40_500, true, true),
            vec![MaliceEffect::RemoveCage, MaliceEffect::BuffMalice]
        );
    }
    #[test]
    fn srx3_malice_retained_timer_keeps_original_cutoff_and_current_successor_delays()
    -> Result<(), MaliceRefusal> {
        let mut room = MaliceRoom::entered(0);
        assert!(room.cage_died([9; 32], 24_000)?);
        let before = room.clone();
        assert!(
            room.advance_through(22_999, 100_000, false, true)
                .is_empty()
        );
        assert_eq!(
            room, before,
            "actual execution time cannot admit a newer original deadline"
        );
        assert_eq!(
            room.advance_through(23_000, 100_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert_eq!(room.next_due_ms(), Some(47_000));
        let before = room.clone();
        assert!(room.advance_through(23_000, 100_000, true, true).is_empty());
        assert_eq!(
            room, before,
            "the later death callback remains pending beyond this cutoff"
        );
        assert!(room.advance_through(47_000, 100_000, true, true).is_empty());
        assert_eq!(
            room.next_due_ms(),
            Some(140_000),
            "Cage lifetime starts at actual spawn execution"
        );
        let before = room.clone();
        assert!(
            room.advance_through(200_000, 139_999, true, true)
                .is_empty()
        );
        assert_eq!(
            room, before,
            "a future cutoff cannot make the real clock run early"
        );
        assert_eq!(
            room.advance_through(140_000, 200_000, true, true),
            vec![MaliceEffect::RemoveCage, MaliceEffect::BuffMalice]
        );
        assert_eq!(
            room.next_due_ms(),
            Some(223_000),
            "next Cage birth starts at actual removal execution"
        );
        Ok(())
    }
    #[test]
    fn srx3_malice_connected_groups_preserve_the_source_twelve_tiles() -> Result<(), String> {
        let center = RoomPosition {
            x: 33709,
            y: 31599,
            floor: 14,
        };
        let tiles = select_white_cells(center, |_| Some(0)).ok_or("bounded source draws")?;
        assert_eq!(
            tiles.len(),
            12,
            "source while groups*3 <11 creates four groups"
        );
        for group in tiles.chunks(3) {
            for pair in group.windows(2) {
                assert!((pair[0].x - pair[1].x).abs() <= 1 && (pair[0].y - pair[1].y).abs() <= 1);
            }
        }
        for (i, tile) in tiles.iter().enumerate() {
            assert!(!tiles[..i].contains(tile));
        }
        assert!(select_white_cells(center, Some).is_none());
        Ok(())
    }
    #[test]
    fn srx3_malice_copied_cage_death_keeps_both_source_remove_timers() -> Result<(), MaliceRefusal>
    {
        let mut room = MaliceRoom::entered(0);
        assert_eq!(
            room.advance(23_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert!(
            room.cage_died([1; 32], 24_000)?,
            "actual copied onDeath schedules its23s continuation"
        );
        assert!(
            !room.cage_died([1; 32], 25_000)?,
            "same original death cannot schedule twice"
        );
        assert!(room.advance(46_999, false, true).is_empty());
        assert_eq!(
            room.advance(47_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert_eq!(
            room.advance(63_000, true, true),
            vec![MaliceEffect::RemoveCage, MaliceEffect::BuffMalice],
            "older callback removes the NEW currently named Cage"
        );
        assert_eq!(
            room.advance(86_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert_eq!(
            room.advance(87_000, true, true),
            vec![MaliceEffect::RemoveCage, MaliceEffect::BuffMalice],
            "47s spawn's removal survives the63s callback"
        );
        Ok(())
    }
    #[test]
    fn srx3_malice_equal_deadlines_follow_actual_source_registration_order()
    -> Result<(), MaliceRefusal> {
        let mut room = MaliceRoom::entered(0);
        assert_eq!(
            room.advance(23_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        assert!(room.cage_died([2; 32], 40_000)?);
        assert_eq!(room.next_due_ms(), Some(63_000));
        assert!(
            room.advance(63_000, false, true).is_empty(),
            "remove registered at23s executes before spawn registered at40s"
        );
        assert_eq!(room.next_due_ms(), Some(63_000));
        assert_eq!(
            room.advance(63_000, false, true),
            vec![MaliceEffect::SpawnCage]
        );
        Ok(())
    }
}
