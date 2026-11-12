//! UTC calendar qualification for the held Hot Cuisine source program.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    /// UTC Gregorian month for bounded persisted Unix timestamps. No local
    /// timezone, fixed event month or process calendar participates in a recipe.
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        if !(-62_135_596_800..=253_402_300_799).contains(&seconds) {
            return None;
        }
        let days = seconds.div_euclid(86_400) + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let month = month_index + if month_index < 10 { 3 } else { -9 };
        Some(Self {
            year: i32::try_from(year + i64::from(month <= 2)).ok()?,
            month: u8::try_from(month).ok()?,
        })
    }
    #[must_use]
    pub fn is_valid(self) -> bool {
        (1..=12).contains(&self.month)
    }
}

/// `data-global/npc/jean_pierre.lua:139-161`. The date test is deliberately
/// asymmetric: August and a later year OR a non-August previous interaction.
#[must_use]
pub fn can_cook(
    now: YearMonth,
    last: Option<YearMonth>,
    bonus_year: Option<i32>,
    bonus_used: bool,
) -> bool {
    if !now.is_valid() || last.is_some_and(|date| !date.is_valid()) {
        return false;
    }
    last.is_none() || is_new_period(now, last) || (bonus_year == Some(now.year) && !bonus_used)
}

/// `data-global/npc/jean_pierre.lua:113-119`; a new period does not clear an old
/// bonus-used flag, it simply refrains from setting it again.
#[must_use]
pub fn spends_bonus(now: YearMonth, last: Option<YearMonth>) -> bool {
    last.is_some() && !is_new_period(now, last)
}

fn is_new_period(now: YearMonth, last: Option<YearMonth>) -> bool {
    now.month == 8 && last.is_some_and(|last| now.year > last.year || last.month != 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_months_are_utc_at_epoch_leap_day_and_year_boundaries() {
        for (seconds, year, month) in [
            (0, 1970, 1),
            (-1, 1969, 12),
            (1_709_164_800, 2024, 2),
            (1_709_251_200, 2024, 3),
            (1_785_542_400, 2026, 8),
            (1_798_761_599, 2026, 12),
            (1_798_761_600, 2027, 1),
        ] {
            assert_eq!(
                YearMonth::from_unix_seconds(seconds),
                Some(YearMonth { year, month })
            );
        }
        assert_eq!(YearMonth::from_unix_seconds(i64::MAX), None);
        assert_eq!(YearMonth::from_unix_seconds(i64::MIN), None);
    }

    #[test]
    fn hot_cuisine_preserves_august_or_bonus_rules() {
        let august = YearMonth {
            year: 2026,
            month: 8,
        };
        let same = Some(august);
        let prior = Some(YearMonth {
            year: 2025,
            month: 8,
        });
        assert!(can_cook(august, None, None, false));
        assert!(can_cook(august, prior, None, true));
        assert!(!can_cook(august, same, None, false));
        assert!(can_cook(august, same, Some(2026), false));
        assert!(!can_cook(august, same, Some(2026), true));
        assert!(!can_cook(
            YearMonth {
                year: 2027,
                month: 7
            },
            prior,
            Some(2026),
            false
        ));
    }

    #[test]
    fn cook_operation_has_no_extra_say_month_guard() {
        let september = YearMonth {
            year: 2026,
            month: 9,
        };
        assert!(can_cook(september, None, None, false));
        assert!(can_cook(
            september,
            Some(YearMonth {
                year: 2026,
                month: 8
            }),
            Some(2026),
            false
        ));
    }

    #[test]
    fn new_august_period_does_not_spend_the_bonus() {
        let august = YearMonth {
            year: 2026,
            month: 8,
        };
        assert!(!spends_bonus(august, None));
        assert!(!spends_bonus(
            august,
            Some(YearMonth {
                year: 2025,
                month: 8
            })
        ));
        assert!(!spends_bonus(
            august,
            Some(YearMonth {
                year: 2026,
                month: 7
            })
        ));
        assert!(spends_bonus(august, Some(august)));
    }
}
