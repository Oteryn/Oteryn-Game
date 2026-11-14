//! Atomic shared-frontend requests; native input keeps its existing behavior.
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::boss_rewards::{BossRewardsPanel, Intent, request};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone)]
pub(crate) struct BossRewardChestBound {
    session: Weak<Mutex<Mailbox>>,
    pub panel: BossRewardsPanel,
    pub live: bool,
}
impl BossRewardChestBound {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.session, &other.session)
    }
    pub(crate) fn matches(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.session, &other.session)
            && self.panel == other.panel
            && self.live == other.live
    }
}
#[derive(Clone, Copy)]
pub(crate) enum BossRewardChestAction {
    Toggle,
    Close,
    Request {
        intent: Intent,
        death: [u8; 16],
        ordinal: u16,
        confirmed: bool,
    },
}
impl PlayLink {
    pub(crate) fn boss_reward_chest_bound(&self) -> BossRewardChestBound {
        let mailbox = lock(&self.mailbox);
        BossRewardChestBound {
            session: Arc::downgrade(&self.mailbox),
            panel: mailbox.boss_rewards.clone(),
            live: !mailbox.boss_rewards_shutdown
                && !mailbox.custom_hotkeys_ended
                && !self.commands.is_closed(),
        }
    }
    pub(crate) fn boss_reward_chest_dispatch_bound(
        &self,
        expected: &BossRewardChestBound,
        action: BossRewardChestAction,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !Weak::ptr_eq(&expected.session, &Arc::downgrade(&self.mailbox))
            || mailbox.boss_rewards_shutdown
            || mailbox.custom_hotkeys_ended
            || mailbox.server_modal.is_active()
            || mailbox.boss_rewards != expected.panel
            || self.commands.is_closed()
        {
            return false;
        }
        let panel = &mut mailbox.boss_rewards;
        match action {
            BossRewardChestAction::Toggle => {
                panel.open = !panel.open;
                true
            }
            BossRewardChestAction::Close => {
                panel.open = false;
                true
            }
            BossRewardChestAction::Request {
                intent,
                death,
                ordinal,
                confirmed,
            } => {
                if !panel.open
                    || !panel.selected
                    || panel.pending
                    || (intent != Intent::Browse && !confirmed)
                {
                    return false;
                }
                // Validate on a draft: an invalid target never mutates the owner.
                let mut draft = panel.clone();
                draft.death = death;
                draft.selected_ordinal = ordinal;
                let Some(command) = request(&mut draft, intent) else {
                    return false;
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::BossRewards(command))
                    .is_err()
                {
                    return false;
                }
                *panel = draft;
                true
            }
        }
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_boss_reward_chest_selected(&self) {
        self.boss_rewards_initialize(true);
    }
    pub(crate) fn bevy_boss_reward_chest_reply(
        &self,
        response: &oteryn_client_runtime::boss_rewards::BossRewardsResponse,
    ) {
        self.boss_rewards_result(response);
    }
    pub(crate) fn bevy_boss_reward_chest_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
