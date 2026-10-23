//! Open-PvP skull consequences. All instants are PostgreSQL epoch microseconds.

pub const DAY_US: i64 = 86_400_000_000;
pub const FRAG_WINDOWS_US: [i64; 3] = [DAY_US, 7 * DAY_US, 30 * DAY_US];
pub const RED_THRESHOLDS_MILLI: [u64; 3] = [3_000, 5_000, 10_000];
pub const BLACK_THRESHOLDS_MILLI: [u64; 3] = [6_000, 10_000, 20_000];
pub const RED_DURATION_US: i64 = 30 * DAY_US;
pub const BLACK_DURATION_US: i64 = 45 * DAY_US;
pub const REVENGE_DURATION_US: i64 = 7 * DAY_US;
pub const KILL_BLOCK_US: i64 = 900_000_000;
pub const LEDGER_WINDOW_US: i64 = 300_000_000;
pub const POINT_RETENTION_US: i64 = 45 * DAY_US;
pub const MAX_CONTRIBUTORS: usize = 80;
pub const ASSIST_POINTS_MILLI: u16 = 500;
pub const DAMAGE_POINTS_MILLI: u16 = 1_000;
pub const FULL_DAMAGE_CONTRIBUTORS: usize = 5;

/// Stored red/black state; white remains a separate deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PvpSkull {
    None,
    Red,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpContributorKind {
    Damage,
    Assist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkullState {
    pub skull: PvpSkull,
    pub until_us: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnjustifiedPoint {
    pub committed_at_us: i64,
    pub points_milli: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSkullInput;

/// Bounded-memory rolling totals, fed by the adapter's paged historical read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkullHistory {
    now_us: i64,
    totals: [u64; 3],
}

impl SkullHistory {
    pub fn new(now_us: i64) -> Result<Self, InvalidSkullInput> {
        if now_us < 0 {
            return Err(InvalidSkullInput);
        }
        Ok(Self {
            now_us,
            totals: [0; 3],
        })
    }

    pub fn include(&mut self, point: UnjustifiedPoint) -> Result<(), InvalidSkullInput> {
        if point.committed_at_us < 0
            || point.committed_at_us > self.now_us
            || !(1..=DAMAGE_POINTS_MILLI).contains(&point.points_milli)
        {
            return Err(InvalidSkullInput);
        }
        let age = self.now_us - point.committed_at_us;
        for (total, window) in self.totals.iter_mut().zip(FRAG_WINDOWS_US) {
            if age < window {
                *total = total
                    .checked_add(u64::from(point.points_milli))
                    .ok_or(InvalidSkullInput)?;
            }
        }
        Ok(())
    }
}

/// One new positive increment, separate from historical rows, counts exactly once.
/// D1443: a historical point at the window's lower endpoint no longer counts.
pub fn evaluate_skull(
    now_us: i64,
    stored: SkullState,
    history: &SkullHistory,
    increment_milli: u16,
) -> Result<SkullState, InvalidSkullInput> {
    if now_us < 0
        || history.now_us != now_us
        || !(1..=DAMAGE_POINTS_MILLI).contains(&increment_milli)
        || (stored.skull == PvpSkull::None) != stored.until_us.is_none()
        || stored.until_us.is_some_and(|until| until < 0)
    {
        return Err(InvalidSkullInput);
    }
    let mut totals = history.totals;
    for total in &mut totals {
        *total = total
            .checked_add(u64::from(increment_milli))
            .ok_or(InvalidSkullInput)?;
    }
    let reached = |thresholds: [u64; 3]| {
        totals
            .iter()
            .zip(thresholds)
            .any(|(total, limit)| *total >= limit)
    };
    let awarded = if reached(BLACK_THRESHOLDS_MILLI) {
        PvpSkull::Black
    } else if reached(RED_THRESHOLDS_MILLI) {
        PvpSkull::Red
    } else {
        PvpSkull::None
    };
    let current = if stored.until_us.is_some_and(|until| until > now_us) {
        stored.skull
    } else {
        PvpSkull::None
    };
    let skull = current.max(awarded);
    let duration = match skull {
        PvpSkull::None => {
            return Ok(SkullState {
                skull,
                until_us: None,
            });
        }
        PvpSkull::Red => RED_DURATION_US,
        PvpSkull::Black => BLACK_DURATION_US,
    };
    let until = now_us.checked_add(duration).ok_or(InvalidSkullInput)?;
    Ok(SkullState {
        skull,
        until_us: Some(if current == PvpSkull::None {
            until
        } else {
            until.max(stored.until_us.ok_or(InvalidSkullInput)?)
        }),
    })
}

/// The 5/N split counts all damage contributors, including justified ones.
pub fn contributor_points(kind: PvpContributorKind, damage_count: usize) -> u16 {
    match kind {
        PvpContributorKind::Assist => ASSIST_POINTS_MILLI,
        PvpContributorKind::Damage if damage_count > FULL_DAMAGE_CONTRIBUTORS => u16::try_from(
            usize::from(DAMAGE_POINTS_MILLI) * FULL_DAMAGE_CONTRIBUTORS / damage_count,
        )
        .unwrap_or(0),
        PvpContributorKind::Damage => DAMAGE_POINTS_MILLI,
    }
}
