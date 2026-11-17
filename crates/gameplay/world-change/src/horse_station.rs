/// The donor's grass movement ID is common outside the two stable areas.
pub fn enters_stable(item_id: u16, x: i32, y: i32, floor: i16) -> bool {
    item_id == 31159
        && floor == 7
        && (((32437..=32448).contains(&x) && (32230..=32239).contains(&y))
            || ((32846..=32850).contains(&x) && (32114..=32120).contains(&y)))
}

const HOUR_MS: i64 = 60 * 60 * 1_000;
const LOOSE_DURATION_MS: i64 = 4 * 24 * HOUR_MS;
pub const RENTAL_PRICE_GOLD: u64 = 500;
const RENTAL_MOUNTS: [u16; 3] = [22, 25, 26];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HorseRental {
    pub mount_id: u16,
    pub expires_at_ms: i64,
}

impl HorseRental {
    pub fn is_current(self, now_ms: i64) -> bool {
        now_ms < self.expires_at_ms
    }
}

/// World-wide Horse Station phase, with the loose deadline in Unix milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseStation {
    Confined,
    LooseUntil(i64),
}

impl HorseStation {
    /// Crystal's rental escape draw is 3 in 10. The catalogue task specifies
    /// a four-day deadline, applied at the first server save at or after it.
    pub fn after_rental(self, now_ms: i64, draw: u8) -> Self {
        match self {
            Self::Confined if draw < 3 => {
                Self::LooseUntil(now_ms.saturating_add(LOOSE_DURATION_MS))
            }
            phase => phase,
        }
    }

    /// Crystal palomino.lua:183-190, 206-219 and appaloosa.lua's matching offer.
    /// The returned lease is contingent on payment in the caller's transaction.
    pub fn rental(
        self,
        now_ms: i64,
        previous: Option<HorseRental>,
        available_gold: u64,
        mount_draw: usize,
    ) -> Option<HorseRental> {
        if self != Self::Confined
            || previous.is_some_and(|rental| rental.is_current(now_ms))
            || available_gold < RENTAL_PRICE_GOLD
        {
            return None;
        }
        Some(HorseRental {
            mount_id: *RENTAL_MOUNTS.get(mount_draw)?,
            expires_at_ms: now_ms.checked_add(24 * HOUR_MS)?,
        })
    }

    /// Crystal palomino.lua:104-124, 145-160. Escape starts an immediate
    /// wave, followed by one wave per three hours until confinement.
    pub fn wave_due(self, now_ms: i64, previous_wave_ms: Option<i64>) -> bool {
        matches!(self, Self::LooseUntil(deadline) if now_ms < deadline)
            && previous_wave_ms.is_none_or(|last| {
                matches!(self, Self::LooseUntil(deadline) if last < deadline.saturating_sub(LOOSE_DURATION_MS))
                    || now_ms.saturating_sub(last) >= 3 * HOUR_MS
            })
    }

    pub fn after_lure(self, now_ms: i64, draw: u8) -> Self {
        match self {
            Self::LooseUntil(deadline) if now_ms < deadline && draw == 0 => Self::Confined,
            phase => phase,
        }
    }

    pub fn at_save(self, now_ms: i64) -> Self {
        match self {
            Self::LooseUntil(deadline) if now_ms >= deadline => Self::Confined,
            phase => phase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rental_requires_500_gold_and_confined_horses_and_expires_after_one_day() {
        let confined = HorseStation::Confined;
        assert_eq!(confined.rental(50, None, 499, 0), None);
        assert_eq!(confined.rental(50, None, 500, 3), None);
        assert_eq!(HorseStation::LooseUntil(51).rental(50, None, 500, 0), None);
        for (draw, mount_id) in [22, 25, 26].into_iter().enumerate() {
            let rental = HorseRental {
                mount_id,
                expires_at_ms: 86_400_050,
            };
            assert_eq!(confined.rental(50, None, 500, draw), Some(rental));
            assert!(rental.is_current(86_400_049));
            assert!(!rental.is_current(86_400_050));
            assert_eq!(confined.rental(86_400_049, Some(rental), 500, draw), None);
            assert!(
                confined
                    .rental(86_400_050, Some(rental), 500, draw)
                    .is_some()
            );
        }
    }

    #[test]
    fn loose_horses_wave_immediately_then_every_three_hours_until_confinement() {
        let loose = HorseStation::LooseUntil(4 * 24 * 3_600_000);
        assert!(loose.wave_due(100, None));
        assert!(loose.wave_due(100, Some(-1)));
        assert!(!loose.wave_due(10_800_099, Some(100)));
        assert!(loose.wave_due(10_800_100, Some(100)));
        assert!(!HorseStation::Confined.wave_due(10_800_100, Some(100)));
        assert!(!loose.wave_due(4 * 24 * 3_600_000, None));
    }

    #[test]
    fn lure_requires_stable_grass_and_a_one_in_thirty_draw() {
        assert!(enters_stable(31159, 32437, 32230, 7));
        assert!(enters_stable(31159, 32850, 32120, 7));
        assert!(!enters_stable(31159, 32851, 32120, 7));
        assert!(!enters_stable(31159, 32850, 32120, 6));
        assert!(!enters_stable(31158, 32850, 32120, 7));
        let loose = HorseStation::LooseUntil(100);
        assert_eq!(loose.after_lure(99, 0), HorseStation::Confined);
        for draw in 1..30 {
            assert_eq!(loose.after_lure(99, draw), loose);
        }
        assert_eq!(loose.after_lure(100, 0), loose);
    }

    #[test]
    fn loose_horses_revert_at_first_save_on_or_after_four_days() {
        let escaped = 1_000;
        let deadline = escaped + 4 * 24 * 60 * 60 * 1_000;
        let loose = HorseStation::Confined.after_rental(escaped, 2);
        assert_eq!(loose, HorseStation::LooseUntil(deadline));
        assert_eq!(
            HorseStation::Confined.after_rental(escaped, 3),
            HorseStation::Confined
        );
        assert_eq!(loose.after_rental(escaped + 1, 0), loose);
        assert_eq!(loose.at_save(deadline - 1), loose);
        assert_eq!(loose.at_save(deadline), HorseStation::Confined);
        assert_eq!(loose.at_save(deadline + 1), HorseStation::Confined);
        assert_eq!(
            HorseStation::Confined.at_save(deadline),
            HorseStation::Confined
        );
    }
}
