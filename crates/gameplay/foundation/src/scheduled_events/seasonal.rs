//! Annual gameplay windows at the accepted Europe/Berlin 06:00 server save.
use oteryn_protocol_oteryn::world_event::{SeasonalEventKind, SeasonalEventState};

/// Upcoming or active occurrence of each seasonal event from committed content.
/// W05d accepted dates in schedule.json use server save, not UTC midnight. The
/// Europe/Berlin 06:00 boundary is unambiguous even on a daylight-saving change.
pub fn seasonal_events_at(unix_seconds: i64) -> Vec<SeasonalEventState> {
    if !(0..=253_402_300_799).contains(&unix_seconds) {
        return Vec::new();
    }
    let approximate_year = 1970 + unix_seconds / 31_556_952;
    let mut result = Vec::new();
    for source in &super::content_schedule().seasonal {
        let event = match source.event.as_str() {
            "halloween" => SeasonalEventKind::Halloween,
            "christmas" => SeasonalEventKind::Christmas,
            "piece_of_cake" => SeasonalEventKind::BirthdayCake,
            // Other seasonal mechanics have their own consumers (e.g. Autumn Vintage).
            _ => continue,
        };
        let mut next: Option<(i64, i64)> = None;
        for year in (approximate_year - 1).max(1970)..=(approximate_year + 2).min(9999) {
            for window in &source.windows {
                let end_year = year
                    + i64::from(
                        (window.end_exclusive.month, window.end_exclusive.day)
                            < (window.start.month, window.start.day),
                    );
                let (Some(start), Some(end)) = (
                    server_save(year, &window.start),
                    server_save(end_year, &window.end_exclusive),
                ) else {
                    continue;
                };
                if end <= unix_seconds || next.is_some_and(|(_, best_end)| best_end <= end) {
                    continue;
                }
                next = Some((start, end));
            }
        }
        if let Some((start, end)) = next {
            result.push(SeasonalEventState {
                event,
                active: start <= unix_seconds && unix_seconds < end,
                start_unix: start as u64,
                end_unix: end as u64,
            });
        }
    }
    result
}

fn server_save(year: i64, date: &super::SourceMonthDay) -> Option<i64> {
    let midnight = super::utc_instant(&format!("{}/{}/{year}", date.month, date.day), "00:00")?;
    let last_sunday = |month| {
        let day31 = super::utc_instant(&format!("{month}/31/{year}"), "00:00")?;
        Some(31 - (day31 / 86_400 + 4).rem_euclid(7))
    };
    let summer = match date.month {
        4..=9 => true,
        3 => i64::from(date.day) >= last_sunday(3)?,
        10 => i64::from(date.day) < last_sunday(10)?,
        _ => false,
    };
    Some(midnight + if summer { 4 * 3_600 } else { 5 * 3_600 })
}

/// Windows observed by production callers at the current wall-clock instant.
pub fn current_seasonal_events() -> Vec<SeasonalEventState> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(now) = super::test_clock::instant() {
        return seasonal_events_at(now);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0);
    seasonal_events_at(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srv7_seasonal_windows_use_server_save_and_roll_into_next_year() {
        // 2026-10-31 04:59:59Z; Berlin is back on CET, save is 05:00Z.
        let before = seasonal_events_at(1_793_422_799);
        assert_eq!(before.len(), 3);
        let halloween = before
            .iter()
            .find(|e| e.event == SeasonalEventKind::Halloween);
        assert_eq!(
            halloween.map(|e| (e.active, e.start_unix, e.end_unix)),
            Some((false, 1_793_422_800, 1_793_682_000))
        );
        for (instant, active) in [
            (1_793_422_800, true),
            (1_793_681_999, true),
            (1_793_682_000, false),
        ] {
            let events = seasonal_events_at(instant);
            assert_eq!(
                events
                    .iter()
                    .find(|e| e.event == SeasonalEventKind::Halloween)
                    .map(|e| e.active),
                Some(active)
            );
        }
        let christmas = seasonal_events_at(1_798_347_600)
            .into_iter()
            .find(|e| e.event == SeasonalEventKind::Christmas);
        assert_eq!(
            christmas.map(|e| (e.active, e.start_unix, e.end_unix)),
            Some((true, 1_796_101_200, 1_798_779_600))
        );
    }

    #[test]
    fn srv7_cake_calendar_is_not_a_rate_event_and_rejects_invalid_month_days() {
        let events = seasonal_events_at(1_771_650_000);
        assert_eq!(
            events
                .iter()
                .find(|e| e.event == SeasonalEventKind::BirthdayCake)
                .map(|e| e.active),
            Some(true)
        );
        assert_eq!(
            super::super::content_schedule().rates_at(1_771_650_000),
            super::super::EventRates::default()
        );
        for bad in [
            r#"{"events":[],"seasonal":[{"event":"halloween","source":"test","windows":[{"start":{"month":13,"day":1},"end_exclusive":{"month":11,"day":3}}]}]}"#,
            r#"{"events":[],"seasonal":[{"event":"halloween","source":"test","windows":[{"start":{"month":2,"day":30},"end_exclusive":{"month":3,"day":3}}]}]}"#,
        ] {
            assert!(super::super::Schedule::parse(bad).is_err());
        }
    }
}

/// Current-Tibia Santa/Messenger window: December 12 server save through
/// December 31 server save (exclusive). The broad Christmas event has a
/// separate calendar. Return the occurrence start as its annual entitlement key.
/// Source: Santa Claus Wiki article4372, mirror acc9994e2ad8868696bc10ef08070108e2baba92.
pub fn santa_occurrence_at(unix_seconds: i64) -> Option<(i64, i64)> {
    if !(0..=253_402_300_799).contains(&unix_seconds) {
        return None;
    }
    let approximate_year = 1970 + unix_seconds / 31_556_952;
    for year in (approximate_year - 1).max(1970)..=(approximate_year + 1).min(9999) {
        let start = server_save(year, &super::SourceMonthDay { month: 12, day: 12 })?;
        let end = server_save(year, &super::SourceMonthDay { month: 12, day: 31 })?;
        if start <= unix_seconds && unix_seconds < end {
            return Some((start, end));
        }
    }
    None
}

#[cfg(test)]
mod santa_window_tests {
    use super::*;

    #[test]
    fn santa_current_tibia_window_has_independent_server_save_boundaries()
    -> Result<(), &'static str> {
        let start = server_save(2026, &super::super::SourceMonthDay { month: 12, day: 12 })
            .ok_or("valid server-save date")?;
        let end = server_save(2026, &super::super::SourceMonthDay { month: 12, day: 31 })
            .ok_or("valid server-save date")?;
        assert_eq!(santa_occurrence_at(start - 1), None);
        assert_eq!(santa_occurrence_at(start), Some((start, end)));
        assert_eq!(santa_occurrence_at(end - 1), Some((start, end)));
        assert_eq!(santa_occurrence_at(end), None);
        assert_eq!(
            santa_occurrence_at(end + 86_400 - 1),
            None,
            "Jan1 before save is not a new entitlement"
        );
        assert!(
            seasonal_events_at(start - 1)
                .iter()
                .any(|event| event.event == SeasonalEventKind::Christmas && event.active)
        );
        Ok(())
    }

    #[test]
    fn santa_entitlement_key_is_occurrence_start_and_changes_once_per_season()
    -> Result<(), &'static str> {
        let start = server_save(2026, &super::super::SourceMonthDay { month: 12, day: 12 })
            .ok_or("valid server-save date")?;
        let next = server_save(2027, &super::super::SourceMonthDay { month: 12, day: 12 })
            .ok_or("valid server-save date")?;
        assert_eq!(
            santa_occurrence_at(start).ok_or("active current season")?.0,
            santa_occurrence_at(start + 86_400)
                .ok_or("active following day")?
                .0
        );
        assert_ne!(
            santa_occurrence_at(start).ok_or("active current season")?.0,
            santa_occurrence_at(next)
                .ok_or("active following season")?
                .0
        );
        assert_eq!(santa_occurrence_at(-1), None);
        assert_eq!(santa_occurrence_at(i64::MAX), None);
        Ok(())
    }
}
