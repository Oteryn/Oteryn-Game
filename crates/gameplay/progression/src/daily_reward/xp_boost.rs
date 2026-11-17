//! Winter Update 2026 (thread 565) XP boost lines. The Daily Reward boost and the Store
//! boost are separate lines whose percentages add; a newer Daily Reward boost replaces
//! any remaining older one, and never extends or shortens the Store boost.

/// One stored grant, with its start and end as milliseconds relative to `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpBoostGrant {
    pub started_ms: i64,
    pub expires_ms: i64,
    pub percent: i16,
}

/// An active line: its percentage and remaining milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpBoostLine {
    pub percent: i16,
    pub remaining_ms: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XpBoostLines {
    pub daily: Option<XpBoostLine>,
    pub store: Option<XpBoostLine>,
}

impl XpBoostLines {
    /// The XP formula's boost: the active lines' percentages added together.
    pub fn percent(&self) -> i16 {
        [self.daily, self.store]
            .iter()
            .flatten()
            .map(|line| line.percent)
            .sum()
    }
}

fn active(grant: &XpBoostGrant, now_ms: i64) -> bool {
    grant.started_ms <= now_ms && grant.expires_ms > now_ms && grant.percent > 0
}

/// `daily_newest_first` lists Daily Reward grants by claim, newest first: only the newest
/// counts. Every active Store grant counts as one line at its highest percentage and
/// latest end.
pub fn active_xp_boost_lines(
    daily_newest_first: &[XpBoostGrant],
    store: &[XpBoostGrant],
    now_ms: i64,
) -> XpBoostLines {
    let daily = daily_newest_first
        .first()
        .filter(|grant| active(grant, now_ms))
        .map(|grant| XpBoostLine {
            percent: grant.percent,
            remaining_ms: grant.expires_ms - now_ms,
        });
    let store = store.iter().filter(|grant| active(grant, now_ms)).fold(
        None,
        |line: Option<XpBoostLine>, grant| {
            let remaining_ms = grant.expires_ms - now_ms;
            Some(match line {
                Some(line) => XpBoostLine {
                    percent: line.percent.max(grant.percent),
                    remaining_ms: line.remaining_ms.max(remaining_ms),
                },
                None => XpBoostLine {
                    percent: grant.percent,
                    remaining_ms,
                },
            })
        },
    );
    XpBoostLines { daily, store }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: i64 = 60_000;

    fn grant(started_ms: i64, expires_ms: i64, percent: i16) -> XpBoostGrant {
        XpBoostGrant {
            started_ms,
            expires_ms,
            percent,
        }
    }

    #[test]
    fn daily_and_store_xp_boost_lines_add() {
        let lines = active_xp_boost_lines(
            &[grant(0, 30 * MINUTE, 50)],
            &[grant(-10 * MINUTE, 50 * MINUTE, 50)],
            0,
        );
        assert_eq!(
            lines.daily,
            Some(XpBoostLine {
                percent: 50,
                remaining_ms: 30 * MINUTE
            })
        );
        // The Store line keeps its own timer; the daily grant does not extend it.
        assert_eq!(
            lines.store,
            Some(XpBoostLine {
                percent: 50,
                remaining_ms: 50 * MINUTE
            })
        );
        assert_eq!(lines.percent(), 100);
        assert_eq!(active_xp_boost_lines(&[], &[], 0).percent(), 0);
    }

    #[test]
    fn second_daily_xp_boost_replaces_the_first() {
        // A 25% daily boost claimed while 20 minutes of an earlier 50% daily boost remain.
        let earlier = grant(-10 * MINUTE, 20 * MINUTE, 50);
        let newer = grant(0, 30 * MINUTE, 25);
        let store = [grant(-5 * MINUTE, 55 * MINUTE, 50)];
        let lines = active_xp_boost_lines(&[newer, earlier], &store, 0);
        assert_eq!(
            lines.daily,
            Some(XpBoostLine {
                percent: 25,
                remaining_ms: 30 * MINUTE
            })
        );
        assert_eq!(lines.percent(), 75);
        assert_eq!(lines.store, active_xp_boost_lines(&[], &store, 0).store);
        // The replaced remainder never returns after the newer boost ends.
        assert_eq!(
            active_xp_boost_lines(&[newer, earlier], &[], 31 * MINUTE),
            XpBoostLines::default()
        );
        assert_eq!(
            active_xp_boost_lines(&[newer, earlier], &[], 15 * MINUTE).percent(),
            25
        );
    }
}
