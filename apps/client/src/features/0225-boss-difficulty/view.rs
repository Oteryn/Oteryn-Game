use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_difficulty::BossDifficultyPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_difficulty_panel(&self) -> BossDifficultyPanel {
        lock(&self.mailbox).boss_difficulty.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "bound_tests.rs"]
mod bound_tests;

/// Shared-frontend requests compare the complete rendered owner snapshot under its lock.
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn boss_difficulty_set_open_bound(
        &self,
        expected: &BossDifficultyPanel,
        open: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.boss_difficulty != *expected
        {
            return false;
        }
        mailbox.boss_difficulty.open = open;
        true
    }

    pub fn boss_difficulty_dispatch_bound(
        &self,
        expected: &BossDifficultyPanel,
        intent: oteryn_client_ui::boss_difficulty::Intent,
        boss: &str,
        difficulty: u16,
        confirmed: bool,
    ) -> bool {
        use oteryn_client_ui::boss_difficulty::{Intent, request};
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.boss_difficulty != *expected
            || !expected.open
            || !expected.selected
            || expected.pending
            || matches!(intent, Intent::Start | Intent::Cancel) && !confirmed
        {
            return false;
        }
        // Validation and request construction use a draft; rejected actions never mutate truth.
        let mut draft = expected.clone();
        if intent == Intent::Open {
            draft.boss_key = boss.trim().to_owned();
        } else if draft.context.is_none() || boss.trim() != expected.boss_key {
            return false;
        } else if intent != Intent::Cancel {
            if difficulty > oteryn_client_runtime::boss_difficulty::MAX_DIFFICULTY {
                return false;
            }
            draft.difficulty = difficulty;
        }
        let Some(command) = request(&mut draft, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(command))
            .is_err()
        {
            return false;
        }
        // The draft choice is not a confirmed choice. Only the existing reply reducer
        // changes the displayed boss, context, difficulty, progress, and pity tracker.
        mailbox.boss_difficulty.pending = true;
        mailbox.boss_difficulty.feedback = None;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_boss_difficulty_panel(&self, panel: BossDifficultyPanel) {
        lock(&self.0).boss_difficulty = panel;
    }
    pub(crate) fn bevy_boss_difficulty_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.boss_difficulty_result(result);
    }
    pub(crate) fn bevy_boss_difficulty_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
