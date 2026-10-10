//! SPAWN-1a (CREATURE-AI-0 §6.2, §6.3): the pure spawn schedule of one Channel.
//!
//! Activation realizes points in canonical order (source, point ordinal), at most
//! [`REALIZATIONS_PER_WINDOW_MAX`] per owner window. Respawn keeps at most one pending occurrence
//! per point ([`RESPAWN_PENDING_PER_CHANNEL_MAX`] in all) and walks the §6.3 chain: blocked by
//! player interest (blockable points only) with a successor one full delay later; otherwise the
//! warning, then admission [`SPAWN_WARNING_MICROS`] later; an occupied cell retries
//! [`OCCUPANCY_RETRIES_MAX`] times one retry interval apart (each retry repeats the warning), then
//! the chain ends skipped with a successor one full delay later. Times are owner semantic
//! microseconds. Nothing here is durable (§6.5).

use std::collections::BTreeMap;

/// `CREATUREAI0-RL-12`: pending respawn occurrences per Channel (one per point).
pub const RESPAWN_PENDING_PER_CHANNEL_MAX: usize = 131_072;
/// `CREATUREAI0-RL-14`: realizations per 50 ms owner window at activation.
pub const REALIZATIONS_PER_WINDOW_MAX: usize = 4_096;
/// One owner window (§7).
pub const OWNER_WINDOW_MICROS: u64 = 50_000;
/// The spawn warning precedes admission by 4,200 ms (Canary 3 × 1,400 ms, D115).
pub const SPAWN_WARNING_MICROS: u64 = 4_200_000;
/// The accepted Occupied chain: three retries, then skipped (D115).
pub const OCCUPANCY_RETRIES_MAX: u8 = 3;

/// One spawn point: its source's ordinal and its point ordinal within the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpawnPoint {
    pub source: u16,
    pub ordinal: u8,
}

/// One point's content-declared timing and its `blocked_by_nearby_players` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointTiming {
    pub respawn_delay_micros: u64,
    pub occupancy_retry_interval_micros: u64,
    pub blockable: bool,
}

/// The activation plan: `points` in canonical order, split into owner windows of at most
/// [`REALIZATIONS_PER_WINDOW_MAX`].
#[must_use]
pub fn realization_windows(points: &[SpawnPoint]) -> Vec<Vec<SpawnPoint>> {
    let mut ordered = points.to_vec();
    ordered.sort_unstable();
    ordered.dedup();
    ordered
        .chunks(REALIZATIONS_PER_WINDOW_MAX)
        .map(<[SpawnPoint]>::to_vec)
        .collect()
}

/// The step a due occurrence runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnStep {
    /// The occurrence is due: a blockable point checks player interest, then warns.
    Due,
    /// A retry of the Occupied chain: it repeats the warning (never blocked).
    Retry,
    /// The warning has run; admission checks occupancy now.
    Admit,
}

/// One pending occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingRespawn {
    pub due_micros: u64,
    pub step: RespawnStep,
    /// Occupied retries already run in this chain (`0..=OCCUPANCY_RETRIES_MAX`).
    pub attempt: u8,
    /// Bumped by each `BLOCKED` or `SKIPPED` end; a successor chain starts over.
    pub successor: u32,
}

/// What a due occurrence decided; the caller performs the admission or the warning effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnOutcome {
    /// Player interest blocked the point; the successor is due one full delay later.
    Blocked { successor_due_micros: u64 },
    /// The warning runs now; admission is due [`SPAWN_WARNING_MICROS`] later.
    Warned { admit_due_micros: u64 },
    /// The cell was occupied at admission; the retry is due one retry interval later.
    Retrying { attempt: u8, retry_due_micros: u64 },
    /// The last retry also found the cell occupied; the successor is due one full delay later.
    Skipped { successor_due_micros: u64 },
    /// The creature was admitted; the point has nothing pending.
    Admitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnScheduleError {
    /// The point already has its one pending occurrence.
    AlreadyPending,
    /// `CREATUREAI0-RL-12` is reached.
    CapacityExceeded,
    /// No pending occurrence of this point is due in this step.
    NotDue,
    /// The owner clock cannot represent the next deadline.
    TimeOverflow,
}

/// The pending respawn occurrences of one Channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RespawnSchedule {
    pending: BTreeMap<SpawnPoint, PendingRespawn>,
}

impl RespawnSchedule {
    #[must_use]
    pub fn pending(&self, point: SpawnPoint) -> Option<PendingRespawn> {
        self.pending.get(&point).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The point's creature is gone at `now`: its one occurrence is due one full delay later.
    pub fn creature_gone(
        &mut self,
        point: SpawnPoint,
        timing: PointTiming,
        now: u64,
    ) -> Result<u64, SpawnScheduleError> {
        if self.pending.contains_key(&point) {
            return Err(SpawnScheduleError::AlreadyPending);
        }
        if self.pending.len() >= RESPAWN_PENDING_PER_CHANNEL_MAX {
            return Err(SpawnScheduleError::CapacityExceeded);
        }
        let due_micros = after(now, timing.respawn_delay_micros)?;
        self.pending.insert(
            point,
            PendingRespawn {
                due_micros,
                step: RespawnStep::Due,
                attempt: 0,
                successor: 0,
            },
        );
        Ok(due_micros)
    }

    /// The occurrences due at `now`, by deadline then point.
    #[must_use]
    pub fn due(&self, now: u64) -> Vec<(SpawnPoint, PendingRespawn)> {
        let mut due = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.due_micros <= now)
            .map(|(point, pending)| (*point, *pending))
            .collect::<Vec<_>>();
        due.sort_by_key(|(point, pending)| (pending.due_micros, *point));
        due
    }

    /// Runs a due `Due` or `Retry` step: blocked (blockable `Due` only), else the warning.
    pub fn run_warning_step(
        &mut self,
        point: SpawnPoint,
        timing: PointTiming,
        interest_blocks: bool,
        now: u64,
    ) -> Result<RespawnOutcome, SpawnScheduleError> {
        let pending = self.due_pending(point, now)?;
        match pending.step {
            RespawnStep::Due if timing.blockable && interest_blocks => {
                let successor_due_micros = after(now, timing.respawn_delay_micros)?;
                pending.due_micros = successor_due_micros;
                pending.successor = pending.successor.saturating_add(1);
                Ok(RespawnOutcome::Blocked {
                    successor_due_micros,
                })
            }
            RespawnStep::Due | RespawnStep::Retry => {
                let admit_due_micros = after(now, SPAWN_WARNING_MICROS)?;
                pending.due_micros = admit_due_micros;
                pending.step = RespawnStep::Admit;
                Ok(RespawnOutcome::Warned { admit_due_micros })
            }
            RespawnStep::Admit => Err(SpawnScheduleError::NotDue),
        }
    }

    /// Runs a due `Admit` step with the caller's occupancy result at `now`.
    pub fn run_admit_step(
        &mut self,
        point: SpawnPoint,
        timing: PointTiming,
        occupied: bool,
        now: u64,
    ) -> Result<RespawnOutcome, SpawnScheduleError> {
        let pending = self.due_pending(point, now)?;
        if pending.step != RespawnStep::Admit {
            return Err(SpawnScheduleError::NotDue);
        }
        if !occupied {
            self.pending.remove(&point);
            return Ok(RespawnOutcome::Admitted);
        }
        if pending.attempt < OCCUPANCY_RETRIES_MAX {
            let retry_due_micros = after(now, timing.occupancy_retry_interval_micros)?;
            pending.attempt += 1;
            pending.due_micros = retry_due_micros;
            pending.step = RespawnStep::Retry;
            return Ok(RespawnOutcome::Retrying {
                attempt: pending.attempt,
                retry_due_micros,
            });
        }
        let successor_due_micros = after(now, timing.respawn_delay_micros)?;
        pending.attempt = 0;
        pending.successor = pending.successor.saturating_add(1);
        pending.due_micros = successor_due_micros;
        pending.step = RespawnStep::Due;
        Ok(RespawnOutcome::Skipped {
            successor_due_micros,
        })
    }

    fn due_pending(
        &mut self,
        point: SpawnPoint,
        now: u64,
    ) -> Result<&mut PendingRespawn, SpawnScheduleError> {
        self.pending
            .get_mut(&point)
            .filter(|pending| pending.due_micros <= now)
            .ok_or(SpawnScheduleError::NotDue)
    }
}

fn after(now: u64, delay: u64) -> Result<u64, SpawnScheduleError> {
    now.checked_add(delay)
        .ok_or(SpawnScheduleError::TimeOverflow)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const DELAY: u64 = 60_000_000;
    const RETRY: u64 = 5_000_000;
    const POINT: SpawnPoint = SpawnPoint {
        source: 0,
        ordinal: 0,
    };

    fn timing(blockable: bool) -> PointTiming {
        PointTiming {
            respawn_delay_micros: DELAY,
            occupancy_retry_interval_micros: RETRY,
            blockable,
        }
    }

    #[test]
    fn realization_windows_are_canonical_and_bounded_by_rl_14() {
        let points = (0..=REALIZATIONS_PER_WINDOW_MAX)
            .rev()
            .map(|index| SpawnPoint {
                source: u16::try_from(index / 2).unwrap(),
                ordinal: u8::try_from(index % 2).unwrap(),
            })
            .collect::<Vec<_>>();
        let windows = realization_windows(&points);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].len(), REALIZATIONS_PER_WINDOW_MAX);
        assert_eq!(windows[1].len(), 1);
        let flat = windows.concat();
        assert!(flat.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(flat[0], SpawnPoint { source: 0, ordinal: 0 });
    }

    #[test]
    fn a_gone_creature_warns_one_full_delay_later_and_admits_after_the_warning() {
        let mut schedule = RespawnSchedule::default();
        assert_eq!(schedule.creature_gone(POINT, timing(false), 10).unwrap(), 10 + DELAY);
        assert!(schedule.due(10 + DELAY - 1).is_empty());
        assert_eq!(
            schedule.run_warning_step(POINT, timing(false), true, 10 + DELAY - 1),
            Err(SpawnScheduleError::NotDue)
        );
        // A non-blockable point is never blocked by interest.
        assert_eq!(
            schedule.run_warning_step(POINT, timing(false), true, 10 + DELAY),
            Ok(RespawnOutcome::Warned {
                admit_due_micros: 10 + DELAY + SPAWN_WARNING_MICROS
            })
        );
        let admit = 10 + DELAY + SPAWN_WARNING_MICROS;
        assert_eq!(
            schedule.run_admit_step(POINT, timing(false), false, admit - 1),
            Err(SpawnScheduleError::NotDue)
        );
        assert_eq!(
            schedule.run_admit_step(POINT, timing(false), false, admit),
            Ok(RespawnOutcome::Admitted)
        );
        assert!(schedule.is_empty());
    }

    #[test]
    fn interest_blocks_a_blockable_point_with_a_successor_one_full_delay_later() {
        let mut schedule = RespawnSchedule::default();
        schedule.creature_gone(POINT, timing(true), 0).unwrap();
        assert_eq!(
            schedule.run_warning_step(POINT, timing(true), true, DELAY),
            Ok(RespawnOutcome::Blocked {
                successor_due_micros: 2 * DELAY
            })
        );
        let pending = schedule.pending(POINT).unwrap();
        assert_eq!((pending.step, pending.successor), (RespawnStep::Due, 1));
        assert_eq!(schedule.len(), 1);
        assert_eq!(
            schedule.run_warning_step(POINT, timing(true), false, 2 * DELAY),
            Ok(RespawnOutcome::Warned {
                admit_due_micros: 2 * DELAY + SPAWN_WARNING_MICROS
            })
        );
    }

    #[test]
    fn an_occupied_cell_retries_three_times_then_skips_to_a_successor() {
        let mut schedule = RespawnSchedule::default();
        schedule.creature_gone(POINT, timing(false), 0).unwrap();
        let mut now = DELAY;
        schedule
            .run_warning_step(POINT, timing(false), false, now)
            .unwrap();
        now += SPAWN_WARNING_MICROS;
        for attempt in 1..=OCCUPANCY_RETRIES_MAX {
            assert_eq!(
                schedule.run_admit_step(POINT, timing(false), true, now),
                Ok(RespawnOutcome::Retrying {
                    attempt,
                    retry_due_micros: now + RETRY
                })
            );
            now += RETRY;
            // Each retry repeats the warning, never blocked.
            assert_eq!(
                schedule.run_warning_step(POINT, timing(true), true, now),
                Ok(RespawnOutcome::Warned {
                    admit_due_micros: now + SPAWN_WARNING_MICROS
                })
            );
            now += SPAWN_WARNING_MICROS;
        }
        assert_eq!(
            schedule.run_admit_step(POINT, timing(false), true, now),
            Ok(RespawnOutcome::Skipped {
                successor_due_micros: now + DELAY
            })
        );
        let pending = schedule.pending(POINT).unwrap();
        assert_eq!(
            (pending.step, pending.attempt, pending.successor),
            (RespawnStep::Due, 0, 1)
        );
    }

    #[test]
    fn at_most_one_pending_occurrence_per_point() {
        let mut schedule = RespawnSchedule::default();
        schedule.creature_gone(POINT, timing(false), 0).unwrap();
        assert_eq!(
            schedule.creature_gone(POINT, timing(false), 1),
            Err(SpawnScheduleError::AlreadyPending)
        );
        assert_eq!(schedule.len(), 1);
    }

    #[test]
    fn pending_occurrences_are_bounded_by_rl_12() {
        let mut schedule = RespawnSchedule::default();
        for index in 0..RESPAWN_PENDING_PER_CHANNEL_MAX {
            let point = SpawnPoint {
                source: u16::try_from(index / 256).unwrap(),
                ordinal: u8::try_from(index % 256).unwrap(),
            };
            schedule.creature_gone(point, timing(false), 0).unwrap();
        }
        assert_eq!(
            schedule.creature_gone(
                SpawnPoint {
                    source: u16::MAX,
                    ordinal: 0
                },
                timing(false),
                0
            ),
            Err(SpawnScheduleError::CapacityExceeded)
        );
    }

    #[test]
    fn due_occurrences_run_by_deadline_then_point() {
        let mut schedule = RespawnSchedule::default();
        let late = SpawnPoint {
            source: 0,
            ordinal: 1,
        };
        schedule.creature_gone(late, timing(false), 0).unwrap();
        schedule.creature_gone(POINT, timing(false), 0).unwrap();
        let other = SpawnPoint {
            source: 1,
            ordinal: 0,
        };
        schedule.creature_gone(other, timing(false), 1).unwrap();
        let due = schedule
            .due(DELAY + 1)
            .into_iter()
            .map(|(point, _)| point)
            .collect::<Vec<_>>();
        assert_eq!(due, vec![POINT, late, other]);
    }
}
