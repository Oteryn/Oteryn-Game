use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::account_achievements::AchievementsPanel;
impl PlayLink {
    #[must_use]
    pub fn achievements_panel(&self) -> AchievementsPanel {
        lock(&self.mailbox).achievements.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{open_character, open_window};

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn achievements_open_bound(&self, expected: &AchievementsPanel) -> bool {
        let mut m = lock(&self.mailbox);
        if self.commands.is_closed()
            || m.server_modal.is_active()
            || m.achievements != *expected
            || !m.achievements.selected
            || m.achievements.open
        {
            return false;
        }
        let mut next = m.achievements.clone();
        next.open = true;
        if next.pending.is_none() {
            let Some(page) = oteryn_client_ui::account_achievements::request(
                &mut next,
                oteryn_client_ui::account_achievements::Intent::Refresh,
            ) else {
                return false;
            };
            if self
                .commands
                .send(crate::combat_input::PlayCommand::AccountAchievements(page))
                .is_err()
            {
                return false;
            }
        }
        m.achievements = next;
        true
    }
    pub fn achievements_browse_bound(
        &self,
        expected: &AchievementsPanel,
        search: &str,
        filter: oteryn_client_ui::account_achievements::AchievementFilter,
        sort: oteryn_client_ui::account_achievements::AchievementSort,
        page_delta: i32,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if self.commands.is_closed()
            || m.server_modal.is_active()
            || m.achievements != *expected
            || !m.achievements.open
            || !m.achievements.selected
        {
            return false;
        }
        let p = &mut m.achievements;
        p.catalog.set_options(search, filter, sort);
        let count = oteryn_client_ui::account_achievements::catalog_rows(&p.catalog).len();
        if page_delta < 0 {
            p.catalog.page = p.catalog.page.saturating_sub(1);
        }
        if page_delta > 0 && p.catalog.page.saturating_add(1).saturating_mul(64) < count {
            p.catalog.page += 1;
        }
        true
    }
    pub fn achievements_continue_bound(&self, expected: &AchievementsPanel) -> bool {
        let mut m = lock(&self.mailbox);
        if self.commands.is_closed()
            || m.server_modal.is_active()
            || m.achievements != *expected
            || !m.achievements.open
            || !m.achievements.selected
        {
            return false;
        }
        let mut next = m.achievements.clone();
        let Some(page) = oteryn_client_ui::account_achievements::catalog_next_request(&mut next)
        else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::AccountAchievements(page))
            .is_err()
        {
            return false;
        }
        m.achievements = next;
        true
    }
    /// Compare the rendered owner snapshot and enqueue under the same lock.
    pub fn achievements_dispatch_bound(
        &self,
        expected: &AchievementsPanel,
        intent: Option<oteryn_client_ui::account_achievements::Intent>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        // Closing presentation preserves every in-flight operation and receipt.
        if intent.is_none() {
            if self.commands.is_closed() || mailbox.quest_shutdown || mailbox.custom_hotkeys_ended {
                return false;
            }
            mailbox.achievements.open = false;
            return true;
        }

        if self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.achievements != *expected
        {
            return false;
        }
        let panel = &mut mailbox.achievements;
        if !panel.selected || !panel.open {
            return false;
        }
        let Some(intent) = intent else {
            panel.open = false;
            return true;
        };
        if panel.pending.is_some() {
            return false;
        }
        let mut next = panel.clone();
        if matches!(
            intent,
            oteryn_client_ui::account_achievements::Intent::Refresh
        ) {
            next.catalog.enabled = true;
        }
        let Some(page) = oteryn_client_ui::account_achievements::request(&mut next, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::AccountAchievements(page))
            .is_err()
        {
            return false;
        }
        *panel = next;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bevy_tests {
    use oteryn_client_ui::account_achievements::Intent;
    #[test]
    fn achievements_actions_are_bound_and_confirmed() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.achievements_initialize();
        let before = link.achievements_panel();
        assert!(link.achievements_open_bound(&before));
        assert!(matches!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::AccountAchievements(0))
        ));
        outbox.achievements_result(
            &oteryn_client_runtime::account_achievements::Response::Page(
                oteryn_client_runtime::account_achievements::AccountAchievementsResult {
                    page: 0,
                    fact_count: 0,
                    total_points: 0,
                    has_more: false,
                    rows: vec![],
                },
            ),
        );
        let stale = link.achievements_panel();
        assert!(link.achievements_browse_bound(
            &stale,
            "changed",
            oteryn_client_ui::account_achievements::AchievementFilter::All,
            oteryn_client_ui::account_achievements::AchievementSort::Name,
            0
        ));
        let before = link.achievements_panel();
        assert!(!link.achievements_dispatch_bound(&stale, Some(Intent::Refresh)));
        assert_eq!(before, link.achievements_panel());
        assert!(link.achievements_dispatch_bound(&before, Some(Intent::Refresh)));
        assert!(matches!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::AccountAchievements(0))
        ));
        assert_eq!(link.achievements_panel().page, before.page);
        let pending = link.achievements_panel();
        assert!(!link.achievements_dispatch_bound(&pending, Some(Intent::Refresh)));
        assert_eq!(pending, link.achievements_panel());
        assert!(!link.achievements_dispatch_bound(&before, Some(Intent::Refresh)));
    }
    #[test]
    fn achievements_stale_replies_are_dropped_after_session_end() {
        let (link, _commands, outbox) = crate::play::play_channel();
        outbox.achievements_initialize();
        assert!(link.achievements_open_bound(&link.achievements_panel()));
        outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
        let ended = link.achievements_panel();
        outbox.achievements_result(
            &oteryn_client_runtime::account_achievements::Response::Page(
                oteryn_client_runtime::account_achievements::AccountAchievementsResult {
                    page: 0,
                    fact_count: 0,
                    total_points: 0,
                    has_more: false,
                    rows: vec![],
                },
            ),
        );
        assert_eq!(ended, link.achievements_panel());
        assert!(!link.achievements_open_bound(&ended));
        assert_eq!(ended, link.achievements_panel());
    }
    #[test]
    fn achievements_rejected_when_session_replaced_or_target_changed() {
        let (link, commands, outbox) = crate::play::play_channel();
        outbox.achievements_initialize();
        let old = link.achievements_panel();
        assert!(link.achievements_open_bound(&old));
        assert!(!link.achievements_open_bound(&old));
        drop(commands);
        let before = link.achievements_panel();
        assert!(!link.achievements_dispatch_bound(&before, Some(Intent::Refresh)));
        assert_eq!(before, link.achievements_panel());
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_achievements_initialize(&self) {
        self.achievements_initialize();
    }
    pub(crate) fn bevy_achievements_result(
        &self,
        response: &oteryn_client_runtime::account_achievements::Response,
    ) {
        self.achievements_result(response);
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_achievements_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
