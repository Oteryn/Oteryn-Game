//! Calendar admission uses the same UTC epoch as the existing durable NPC owner.
use super::NpcRuleError;

pub(super) fn valid_date(month: u8, day: u8) -> bool {
    let maximum = match month {
        2 => 29, // Annual Feb 29 is eligible only when that date actually occurs.
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    (1..=maximum).contains(&day)
}

/// UTC scalar projections from the owner's transaction epoch, never from the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcUtcCalendar {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl NpcUtcCalendar {
    pub fn from_epoch(seconds: i64) -> Result<Self, NpcRuleError> {
        // Civil-from-days, as in durability/quest_state/npc_program.rs::utc_calendar_month.
        // Crystal 201189f data-global/npc/tamoril.lua:45-54 uses inclusive month/day bounds.
        // The existing Hot Cuisine source contract defines donor os.date as UTC.
        let days = seconds.div_euclid(86_400) + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let convert = |value| {
            u8::try_from(value).map_err(|_| NpcRuleError::Arithmetic("calendar date".into()))
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Ok(Self {
            year: i32::try_from(year)
                .map_err(|_| NpcRuleError::Arithmetic("calendar year".into()))?,
            month: convert(month)?,
            day: convert(day)?,
        })
    }
}

pub(super) fn within(seconds: i64, start: (u8, u8), end: (u8, u8)) -> Result<bool, NpcRuleError> {
    let calendar = NpcUtcCalendar::from_epoch(seconds)?;
    let date = (calendar.month, calendar.day);
    Ok(if start <= end {
        start <= date && date <= end
    } else {
        date >= start || date <= end
    })
}
