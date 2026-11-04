//! UTC calendar qualification for the held Hot Cuisine source program.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
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
