//! Party join and leave notices (domain 57, capability 576) refresh the Party window through
//! the existing Party query; the notice itself carries only an opaque member handle.
use crate::party::{PartyPanel, party::COMMAND_TYPE_PARTY_QUERY};
use std::time::{Duration, Instant};

const MAX_ATTEMPTS: u8 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub struct Refresh {
    attempts_left: u8,
    retry_at: Option<Instant>,
    automatic_pending: bool,
    newer_notice: bool,
}

impl Refresh {
    pub fn automatic_pending(&self) -> bool {
        self.automatic_pending
    }

    /// Releases the pending query when its command is discarded or its scope is retired.
    pub fn cancel_query(&mut self) {
        self.automatic_pending = false;
        self.newer_notice = false;
    }

    /// Accounts for a query after a result or a confirmed send interrupted by a handoff.
    pub fn finish_query(&mut self, accepted: bool, retry_from: Instant) {
        if self.automatic_pending {
            if !self.newer_notice {
                self.attempts_left -= 1;
            }
            self.retry_at = Some(retry_from + RETRY_DELAY);
        }
        if accepted {
            self.retry_at = None;
            if !self.newer_notice {
                self.attempts_left = 0;
            }
        }
        self.cancel_query();
    }
}

/// Coalesce notices and renew the retry budget. An older query cannot cover a newer notice.
pub fn observed(panel: &PartyPanel, refresh: &mut Refresh, notices: usize) {
    if notices > 0 {
        refresh.attempts_left = MAX_ATTEMPTS;
        refresh.newer_notice = panel
            .pending
            .as_ref()
            .is_some_and(|command| command.kind == COMMAND_TYPE_PARTY_QUERY);
    }
}

/// Reserves a query when Party is available and idle. A session without the Party
/// capability drops the refresh.
pub fn take_refresh(panel: &PartyPanel, refresh: &mut Refresh, now: Instant) -> bool {
    if !panel.selected {
        *refresh = Refresh::default();
        return false;
    }
    if panel.pending.is_some()
        || refresh.automatic_pending
        || refresh.attempts_left == 0
        || refresh.retry_at.is_some_and(|retry_at| now < retry_at)
    {
        return false;
    }
    refresh.automatic_pending = true;
    true
}

#[cfg(test)]
#[test]
fn notices_owe_one_refresh_until_the_party_query_is_free() {
    let now = Instant::now();
    let mut panel = PartyPanel {
        selected: true,
        ..PartyPanel::default()
    };
    let mut refresh = Refresh::default();
    observed(&panel, &mut refresh, 0);
    assert!(!take_refresh(&panel, &mut refresh, now));
    observed(&panel, &mut refresh, 2);
    observed(&panel, &mut refresh, 0);
    panel.pending = Some(crate::party::PartyCommand::query());
    assert!(!take_refresh(&panel, &mut refresh, now));
    assert_eq!(refresh.attempts_left, MAX_ATTEMPTS);
    panel.pending = None;
    assert!(take_refresh(&panel, &mut refresh, now));
    assert!(!take_refresh(&panel, &mut refresh, now));
    refresh.finish_query(true, now);
    assert!(!take_refresh(&panel, &mut refresh, now + RETRY_DELAY));
    panel.selected = false;
    observed(&panel, &mut refresh, 1);
    assert!(!take_refresh(&panel, &mut refresh, now));
    assert_eq!(refresh.attempts_left, 0);
}
