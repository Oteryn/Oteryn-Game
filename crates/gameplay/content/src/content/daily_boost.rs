//! Deterministic daily creature and boss selection plus race-scoped reward policy.
//!
//! Selection is keyed by the durable World reset epoch rather than process time.  The caller
//! persists the returned keys; reconstructing a server process with the same epoch and pools
//! therefore cannot redraw either slot.

use sha2::{Digest, Sha256};

/// The two independently selected creature definition keys for one server-save day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyBoostSelection {
    pub reset_epoch: u64,
    pub creature: String,
    pub boss: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyBoostError {
    EmptyCreaturePool,
    EmptyBossPool,
    InvalidRate,
    Overflow,
}

/// Select from canonical, non-empty eligible pools. Pool order is deliberately normalized so
/// content loading order cannot change a day's choice.
pub fn select_daily_boost(
    world: &[u8; 16],
    reset_epoch: u64,
    creatures: impl IntoIterator<Item = String>,
    bosses: impl IntoIterator<Item = String>,
) -> Result<DailyBoostSelection, DailyBoostError> {
    fn pick(
        world: &[u8; 16],
        epoch: u64,
        label: &[u8],
        values: impl IntoIterator<Item = String>,
        empty: DailyBoostError,
    ) -> Result<String, DailyBoostError> {
        let mut values: Vec<_> = values.into_iter().collect();
        values.sort_unstable();
        values.dedup();
        if values.is_empty() {
            return Err(empty);
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:daily-boost:v1\0");
        hash.update(world);
        hash.update(epoch.to_be_bytes());
        hash.update(label);
        let digest: [u8; 32] = hash.finalize().into();
        let draw = u64::from_be_bytes([
            digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
        ]);
        Ok(values[(draw % values.len() as u64) as usize].clone())
    }

    Ok(DailyBoostSelection {
        reset_epoch,
        creature: pick(
            world,
            reset_epoch,
            b"creature",
            creatures,
            DailyBoostError::EmptyCreaturePool,
        )?,
        boss: pick(
            world,
            reset_epoch,
            b"boss",
            bosses,
            DailyBoostError::EmptyBossPool,
        )?,
    })
}

/// Apply a configured rational multiplier only when the killed/spawned definition key is the
/// selected key. Kill capture uses this for the base XP award.
pub fn apply_daily_boost(
    amount: u64,
    race: &str,
    selected_race: &str,
    rate: (u32, u32),
) -> Result<u64, DailyBoostError> {
    if rate.1 == 0 {
        return Err(DailyBoostError::InvalidRate);
    }
    if race != selected_race {
        return Ok(amount);
    }
    u128::from(amount)
        .checked_mul(u128::from(rate.0))
        .map(|value| value / u128::from(rate.1))
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(DailyBoostError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_is_stable_across_restart_and_pool_order() -> Result<(), DailyBoostError> {
        let first = select_daily_boost(
            &[7; 16],
            42,
            ["wolf", "rat"].map(str::to_owned),
            ["orshabaal", "ferumbras"].map(str::to_owned),
        )?;
        let restarted = select_daily_boost(
            &[7; 16],
            42,
            ["rat", "wolf", "rat"].map(str::to_owned),
            ["ferumbras", "orshabaal"].map(str::to_owned),
        )?;
        assert_eq!(first, restarted);
        Ok(())
    }

    #[test]
    fn multiplier_is_applied_only_to_the_selected_race() {
        assert_eq!(apply_daily_boost(100, "rat", "rat", (3, 2)), Ok(150));
        assert_eq!(apply_daily_boost(100, "wolf", "rat", (3, 2)), Ok(100));
    }
}
