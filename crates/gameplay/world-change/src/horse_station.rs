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
                Self::LooseUntil(now_ms.saturating_add(4 * 24 * 60 * 60 * 1_000))
            }
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
