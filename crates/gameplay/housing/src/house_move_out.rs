//! Absolute owner notices. Time is sampled once by the durable writer, in milliseconds.
pub const NOTICE_MIN_MS: i64 = 86_400_000;
pub const NOTICE_MAX_MS: i64 = 2_592_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseMoveOutCommand {
    Notice { due_at_unix_ms: i64 },
    Cancel,
}

pub fn valid_deadline(now: i64, due: i64) -> bool {
    now >= 0
        && due
            .checked_sub(now)
            .is_some_and(|remaining| (NOTICE_MIN_MS..=NOTICE_MAX_MS).contains(&remaining))
}

#[cfg(test)]
#[path = "house_move_out_tests.rs"]
mod tests;
