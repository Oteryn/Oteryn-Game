//! Own offers end on the server without a push: an expired offer only moves to the history
//! (`serve/outbound.rs` answers market commands, it sends nothing on expiry). Like Tibia Global,
//! the open market therefore re-queries the shown own-offer or history list when it opens and,
//! because a row carries no expiry time, once a minute while that list stays shown.
use super::*;
use std::time::Duration;

pub(super) const INTERVAL: Duration = Duration::from_secs(60);
#[derive(Resource, Default)]
pub(super) struct Refresh {
    open: bool,
    due: bool,
    queried: Option<Duration>,
}
pub(super) fn own(owner: NonSend<HostOwner>, time: Res<Time>, mut refresh: ResMut<Refresh>) {
    let Some(panel) = owner
        .play
        .as_ref()
        .map(|play| play.link.market_panel())
        .filter(|panel| panel.open)
    else {
        *refresh = Refresh::default();
        return;
    };
    if !refresh.open {
        refresh.open = true;
        refresh.due = true;
    }
    if !matches!(panel.view, MarketView::Own | MarketView::History) {
        refresh.due = false;
        refresh.queried = None;
        return;
    }
    let now = time.elapsed();
    if let Some(queried) = refresh.queried {
        refresh.due |= now.saturating_sub(queried) >= INTERVAL;
    } else if !refresh.due {
        // The player chose this list just now; its own reply is the fresh copy.
        refresh.queried = Some(now);
    }
    if refresh.due
        && panel.selected
        && panel.pending.is_none()
        && let Some(play) = &owner.play
    {
        play.link.market_request(Intent::Refresh);
        refresh.due = false;
        refresh.queried = Some(now);
    }
}
