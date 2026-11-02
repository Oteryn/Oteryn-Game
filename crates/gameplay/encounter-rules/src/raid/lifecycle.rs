use super::RaidError;

/// Source calendar dates are inclusive and interpreted by the existing World
/// reset/calendar owner. This type never chooses a timezone or reset hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidCalendarWindow {
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
}

fn valid_month_day(month: u8, day: u8) -> bool {
    let maximum = match month {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    (1..=maximum).contains(&day)
}

impl RaidCalendarWindow {
    pub fn validate(self) -> Result<(), RaidError> {
        if !valid_month_day(self.start_month, self.start_day)
            || !valid_month_day(self.end_month, self.end_day)
        {
            return Err(RaidError::InvalidCalendar);
        }
        Ok(())
    }

    pub fn contains(self, month: u8, day: u8) -> Result<bool, RaidError> {
        self.validate()?;
        if !valid_month_day(month, day) {
            return Err(RaidError::InvalidCalendar);
        }
        let date = (month, day);
        let start = (self.start_month, self.start_day);
        let end = (self.end_month, self.end_day);
        Ok(if start <= end {
            start <= date && date <= end
        } else {
            date >= start || date <= end
        })
    }
}

/// Additional legacy XML scheduler prerequisites. Probability and persistent
/// restart semantics remain the accepted BOSS-RAID-0 §4.2 native rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyRaidGate {
    pub margin_ms: u64,
    pub repeat: bool,
}

impl LegacyRaidGate {
    /// Crystal 201189f src/lua/creature/raids.cpp:111,125-141,262-267:
    /// one active legacy raid, catalogue order, margin after the previous raid
    /// ends (or startup), and no second execution when repeat is false.
    pub fn allows(
        self,
        now_ms: i64,
        started_at_ms: i64,
        last_raid_end_ms: Option<i64>,
        raid_running: bool,
        executed_in_epoch: bool,
    ) -> Result<bool, RaidError> {
        if now_ms < 0
            || started_at_ms < 0
            || now_ms < started_at_ms
            || last_raid_end_ms.is_some_and(|at| at < 0 || at > now_ms)
        {
            return Err(RaidError::InvalidTime);
        }
        let origin = last_raid_end_ms.map_or(started_at_ms, |at| at.max(started_at_ms));
        let elapsed = u64::try_from(now_ms - origin).map_err(|_| RaidError::InvalidTime)?;
        Ok(!raid_running && (self.repeat || !executed_in_epoch) && elapsed >= self.margin_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidEndPolicy {
    /// BOSS-RAID-0 §4.1,4.4: close after the last event; surviving
    /// monsters remain in the Channel overlay until the next reset.
    UntilReset,
    DespawnAfter {
        after_last_wave_ms: u32,
    },
    BossDeathOrTimeout {
        after_last_wave_ms: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidEndDecision {
    Running,
    Complete { cleanup: bool },
}

impl RaidEndPolicy {
    pub fn decide(
        self,
        last_wave_at_ms: i64,
        now_ms: i64,
        events_complete: bool,
        bosses_placed: u16,
        live_bosses: u16,
    ) -> Result<RaidEndDecision, RaidError> {
        if last_wave_at_ms < 0 || now_ms < 0 {
            return Err(RaidError::InvalidTime);
        }
        if live_bosses > bosses_placed {
            return Err(RaidError::InvalidState);
        }
        let lifetime = match self {
            Self::UntilReset => 0,
            Self::DespawnAfter { after_last_wave_ms }
            | Self::BossDeathOrTimeout { after_last_wave_ms } => {
                if after_last_wave_ms == 0 {
                    return Err(RaidError::InvalidDefinition);
                }
                after_last_wave_ms
            }
        };
        let deadline = last_wave_at_ms
            .checked_add(i64::from(lifetime))
            .ok_or(RaidError::Exhausted)?;
        if !events_complete || now_ms < last_wave_at_ms {
            return Ok(RaidEndDecision::Running);
        }
        let bosses_dead = matches!(self, Self::BossDeathOrTimeout { .. })
            && bosses_placed > 0
            && live_bosses == 0;
        Ok(if now_ms >= deadline || bosses_dead {
            RaidEndDecision::Complete {
                cleanup: !matches!(self, Self::UntilReset),
            }
        } else {
            RaidEndDecision::Running
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid_calendar_inclusive_and_year_wrapping_windows() -> Result<(), RaidError> {
        let june = RaidCalendarWindow {
            start_month: 6,
            start_day: 1,
            end_month: 6,
            end_day: 30,
        };
        assert!(!june.contains(5, 31)?);
        assert!(june.contains(6, 1)?);
        assert!(june.contains(6, 30)?);
        assert!(!june.contains(7, 1)?);
        let winter = RaidCalendarWindow {
            start_month: 12,
            start_day: 25,
            end_month: 1,
            end_day: 6,
        };
        assert!(winter.contains(12, 25)?);
        assert!(winter.contains(1, 6)?);
        assert!(!winter.contains(1, 7)?);
        assert!(!winter.contains(12, 24)?);
        assert_eq!(winter.contains(4, 31), Err(RaidError::InvalidCalendar));
        Ok(())
    }

    #[test]
    fn raid_legacy_margin_is_from_global_end_and_repeat_guard_survives_checks()
    -> Result<(), RaidError> {
        let gate = LegacyRaidGate {
            margin_ms: 3_600_000,
            repeat: false,
        };
        assert!(!gate.allows(3_600_099, 100, None, false, false)?);
        assert!(gate.allows(3_600_100, 100, None, false, false)?);
        assert!(!gate.allows(7_200_000, 100, Some(4_000_000), false, false)?);
        assert!(gate.allows(7_600_000, 100, Some(4_000_000), false, false)?);
        assert!(!gate.allows(7_600_000, 100, Some(4_000_000), true, false)?);
        assert!(!gate.allows(7_600_000, 100, Some(4_000_000), false, true)?);
        assert_eq!(
            gate.allows(99, 100, None, false, false),
            Err(RaidError::InvalidTime)
        );
        Ok(())
    }

    #[test]
    fn raid_lifetime_preserves_persistent_creatures_and_cleans_at_timeout() -> Result<(), RaidError>
    {
        assert_eq!(
            RaidEndPolicy::UntilReset.decide(200, 200, true, 0, 0)?,
            RaidEndDecision::Complete { cleanup: false }
        );
        let timed = RaidEndPolicy::DespawnAfter {
            after_last_wave_ms: 1_000,
        };
        assert_eq!(
            timed.decide(200, 1_199, true, 0, 0)?,
            RaidEndDecision::Running
        );
        assert_eq!(
            timed.decide(200, 1_200, true, 0, 0)?,
            RaidEndDecision::Complete { cleanup: true }
        );
        Ok(())
    }

    #[test]
    fn raid_boss_end_requires_a_placed_boss_and_all_events_or_timeout() -> Result<(), RaidError> {
        let boss = RaidEndPolicy::BossDeathOrTimeout {
            after_last_wave_ms: 1_000,
        };
        assert_eq!(
            boss.decide(200, 300, true, 1, 0)?,
            RaidEndDecision::Complete { cleanup: true }
        );
        assert_eq!(
            boss.decide(200, 300, false, 1, 0)?,
            RaidEndDecision::Running
        );
        assert_eq!(boss.decide(200, 300, true, 0, 0)?, RaidEndDecision::Running);
        assert_eq!(
            boss.decide(200, 1_200, true, 1, 1)?,
            RaidEndDecision::Complete { cleanup: true }
        );
        assert_eq!(
            boss.decide(200, 300, true, 0, 1),
            Err(RaidError::InvalidState)
        );
        Ok(())
    }
}
