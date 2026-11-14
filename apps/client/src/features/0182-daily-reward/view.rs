use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_reward::DailyRewardPanel;
impl PlayLink {
    #[must_use]
    pub fn daily_reward_panel(&self) -> DailyRewardPanel {
        lock(&self.mailbox).daily_reward.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    pub fn open_returner_reward(&self) {
        self.returner_reward_request(false);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            returner_native::open_reward(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).daily_reward.returner_open = true;
        }
    }
}
#[cfg(windows)]
#[path = "returner_native.rs"]
mod returner_native;
#[cfg(windows)]
pub(in crate::play) use returner_native::notify as notify_returner;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_daily_state(&self, panel: DailyRewardPanel) {
        lock(&self.0).daily_reward = panel;
    }
    pub(crate) fn bevy_daily_result(
        &self,
        response: &oteryn_client_runtime::daily_reward::DailyRewardResponse,
    ) {
        self.daily_reward_result(response);
    }
    pub(crate) fn bevy_daily_returner_result(
        &self,
        kind: u32,
        response: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.daily_reward_returner_result(kind, response);
    }
    pub(crate) fn bevy_daily_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

/// A projection is valid only for the physical session that produced it.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct DailyRewardBinding {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub(crate) panel: DailyRewardPanel,
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DailyRewardControl {
    Refresh,
    Claim,
    Details,
    OpenReturner,
    ReturnerRefresh,
    ReturnerClaim,
    Close,
    ReturnerClose,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn daily_reward_binding(&self) -> DailyRewardBinding {
        DailyRewardBinding {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: self.daily_reward_panel(),
        }
    }
    pub(crate) fn daily_reward_dispatch_bound(
        &self,
        expected: &DailyRewardBinding,
        control: DailyRewardControl,
    ) -> bool {
        use crate::combat_input::PlayCommand;
        use DailyRewardControl as C;
        use oteryn_client_ui::daily_reward::{Intent, request, request_details, request_returner};
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.daily_reward != expected.panel
            || mailbox.daily_reward_shutdown
            || mailbox.depot_shutdown
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
        {
            return false;
        }
        let returner = matches!(
            control,
            C::ReturnerClaim | C::ReturnerRefresh | C::ReturnerClose
        );
        if if returner {
            !expected.panel.returner_open
        } else {
            !expected.panel.open
        } {
            return false;
        }
        let mut prepared = mailbox.daily_reward.clone();
        match control {
            C::Close => prepared.open = false,
            C::ReturnerClose => prepared.returner_open = false,
            C::OpenReturner => {
                prepared.returner_open = true;
                if prepared.returner_pending.is_none() && prepared.selected {
                    let Some(command) = request_returner(&mut prepared, false) else {
                        return false;
                    };
                    if self.commands.send(PlayCommand::Feature(command)).is_err() {
                        return false;
                    }
                }
            }
            C::Refresh | C::Claim | C::Details | C::ReturnerRefresh | C::ReturnerClaim => {
                if !prepared.selected {
                    return false;
                }
                if matches!(control, C::ReturnerRefresh | C::ReturnerClaim) {
                    if prepared.returner_pending.is_some()
                        || prepared.returner.pending
                        || control == C::ReturnerClaim
                            && (prepared.returner.offer.is_none() || prepared.returner.is_claimed())
                    {
                        return false;
                    }
                } else if prepared.pending
                    || control == C::Claim
                        && prepared.wall.is_none_or(|w| !w.claim_enabled || w.claimed)
                {
                    return false;
                }
                let command = match control {
                    C::Refresh | C::Claim => request(
                        &mut prepared,
                        if control == C::Claim {
                            Intent::Claim
                        } else {
                            Intent::Refresh
                        },
                    )
                    .map(PlayCommand::DailyReward),
                    C::Details => request_details(&mut prepared).map(PlayCommand::Feature),
                    _ => request_returner(&mut prepared, control == C::ReturnerClaim)
                        .map(PlayCommand::Feature),
                };
                let Some(command) = command else {
                    return false;
                };
                if self.commands.send(command).is_err() {
                    return false;
                }
            }
        }
        mailbox.daily_reward = prepared;
        true
    }
}

#[cfg(feature = "bevy-shell")]
impl DailyRewardBinding {
    pub(crate) fn same_projection(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use super::*;
    use crate::play::play_channel;
    use DailyRewardControl as C;
    use oteryn_client_runtime::daily_reward::daily_reward::DailyRewardWall;
    fn ready() -> DailyRewardPanel {
        DailyRewardPanel {
            selected: true,
            open: true,
            wall: Some(DailyRewardWall {
                lane_day: 1,
                streak: 9,
                jokers: 0,
                claimed: false,
                claim_enabled: true,
            }),
            ..Default::default()
        }
    }
    #[test]
    fn daily_reward_owner_rejects_stale_closed_and_duplicate_without_mutation() {
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_daily_state(ready());
        let bound = link.daily_reward_binding();
        let (other, mut other_commands, other_outbox) = play_channel();
        other_outbox.bevy_daily_state(ready());
        assert!(!other.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(other.daily_reward_panel(), ready());
        assert!(other_commands.try_recv().is_err());
        let mut changed = ready();
        if let Some(wall) = &mut changed.wall {
            wall.streak = 10;
        }
        outbox.bevy_daily_state(changed.clone());
        assert!(!link.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(link.daily_reward_panel(), changed);
        outbox.bevy_daily_state(ready());
        assert!(link.daily_reward_dispatch_bound(&bound, C::Claim));
        let pending = link.daily_reward_panel();
        assert!(!link.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(link.daily_reward_panel(), pending);
        assert!(commands.try_recv().is_ok());
        assert!(commands.try_recv().is_err());
        outbox.bevy_daily_state(ready());
        assert!(link.daily_reward_dispatch_bound(&bound, C::Close));
        let closed = link.daily_reward_panel();
        assert!(!link.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(link.daily_reward_panel(), closed);
        outbox.bevy_daily_state(ready());
        outbox.bevy_daily_end();
        let ended = link.daily_reward_panel();
        assert!(!link.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(link.daily_reward_panel(), ended);
        assert!(commands.try_recv().is_err());
        let (closed, closed_commands, closed_outbox) = play_channel();
        closed_outbox.bevy_daily_state(ready());
        let bound = closed.daily_reward_binding();
        drop(closed_commands);
        assert!(!closed.daily_reward_dispatch_bound(&bound, C::Claim));
        assert_eq!(closed.daily_reward_panel(), ready());
        drop(commands);
    }
}
