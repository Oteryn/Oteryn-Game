//! BOSS_RAID0 §4.1: data-only fixed-slot resolution from caller-resolved reset facts.
//! Calendar interpretation, timezone and current WorldReset authority stay with the caller.

use super::{RaidError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarSelector {
    Weekday(Weekday),
    DayOfMonth(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FixedCalendarRule {
    pub(crate) selector: CalendarSelector,
    /// Internal milliseconds, normalized by the content consumer; no content encoding is added.
    pub(crate) offset_after_reset_ms: i64,
}

/// The calendar day to which the supplied reset belongs, resolved by its real owner.
/// This is proposed data, not proof of current reset, time, epoch or content admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResetCalendarFacts {
    pub(crate) reset_at_ms: i64,
    pub(crate) weekday: Weekday,
    pub(crate) day_of_month: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResolvedFixedSlot {
    /// Complete supplied calendar facts say this rule has no slot for this reset.
    Absent,
    At(i64),
}

pub(super) fn resolve(
    rule: &FixedCalendarRule,
    facts: &ResetCalendarFacts,
) -> Result<ResolvedFixedSlot> {
    if facts.reset_at_ms < 0
        || !(1..=31).contains(&facts.day_of_month)
        || rule.offset_after_reset_ms < 0
        || matches!(rule.selector, CalendarSelector::DayOfMonth(day) if !(1..=31).contains(&day))
    {
        return Err(RaidError::InvalidInput);
    }
    // Validate representability even on a nonmatching date. The accepted contract
    // gives no offset cap; do not invent one or truncate offsets at a day boundary.
    let slot = facts
        .reset_at_ms
        .checked_add(rule.offset_after_reset_ms)
        .ok_or(RaidError::Overflow)?;
    let matches = match rule.selector {
        CalendarSelector::Weekday(weekday) => weekday == facts.weekday,
        CalendarSelector::DayOfMonth(day) => day == facts.day_of_month,
    };
    Ok(if matches {
        ResolvedFixedSlot::At(slot)
    } else {
        ResolvedFixedSlot::Absent
    })
}
