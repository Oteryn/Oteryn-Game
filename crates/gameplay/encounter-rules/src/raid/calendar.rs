use super::RaidError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarSelector {
    /// Monday = 1, Sunday = 7. Calendar conversion belongs to the reset owner.
    Weekday(u8),
    MonthDay(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDay {
    pub reset_at_ms: i64,
    pub weekday: u8,
    pub month_day: u8,
}

impl CalendarSelector {
    pub(super) fn validate(self) -> Result<(), RaidError> {
        match self {
            Self::Weekday(1..=7) | Self::MonthDay(1..=31) => Ok(()),
            _ => Err(RaidError::InvalidCalendar),
        }
    }

    pub(super) fn slot(self, day: CalendarDay, offset: u32) -> Result<Option<i64>, RaidError> {
        self.validate()?;
        if day.reset_at_ms < 0
            || !(1..=7).contains(&day.weekday)
            || !(1..=31).contains(&day.month_day)
        {
            return Err(RaidError::InvalidCalendar);
        }
        let matches = match self {
            Self::Weekday(value) => value == day.weekday,
            Self::MonthDay(value) => value == day.month_day,
        };
        matches
            .then(|| {
                day.reset_at_ms
                    .checked_add(i64::from(offset))
                    .ok_or(RaidError::Exhausted)
            })
            .transpose()
    }
}
