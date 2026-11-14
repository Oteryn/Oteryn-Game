//! Atomic presentation adapter over the three retained read-only status owners.
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Conditions,
    Rest,
    BattleInstinct,
}
impl Target {
    pub(crate) const ALL: [Self; 3] = [Self::Conditions, Self::Rest, Self::BattleInstinct];
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Refresh,
    Close,
}
#[derive(Clone, PartialEq, Eq)]
enum Panel {
    Conditions(oteryn_client_ui::condition_status::ConditionStatusPanel),
    Rest(oteryn_client_ui::rest_status::RestStatusPanel),
    BattleInstinct(oteryn_client_ui::battle_instinct_status::BattleInstinctStatusPanel),
}
#[derive(Clone)]
pub(crate) struct Snapshot {
    session: Weak<Mutex<Mailbox>>,
    pub(crate) target: Target,
    panel: Panel,
    live: bool,
}
impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.target == other.target
            && self.panel == other.panel
            && self.live == other.live
    }
}
impl Eq for Snapshot {}
impl Snapshot {
    pub(crate) fn open(&self) -> bool {
        self.live
            && match &self.panel {
                Panel::Conditions(panel) => panel.open,
                Panel::Rest(panel) => panel.open,
                Panel::BattleInstinct(panel) => panel.open,
            }
    }
    pub(crate) fn ready(&self) -> bool {
        self.live
            && match &self.panel {
                Panel::Conditions(panel) => panel.selected && !panel.pending,
                Panel::Rest(panel) => panel.selected && !panel.pending,
                Panel::BattleInstinct(panel) => panel.selected && !panel.pending,
            }
    }
    pub(crate) fn text(&self) -> String {
        match &self.panel {
            Panel::Conditions(panel) => oteryn_client_ui::condition_status::panel_text(panel),
            Panel::Rest(panel) => oteryn_client_ui::rest_status::panel_text(panel),
            Panel::BattleInstinct(panel) => {
                oteryn_client_ui::battle_instinct_status::panel_text(panel)
            }
        }
    }
}
fn panel(mailbox: &Mailbox, target: Target) -> Panel {
    match target {
        Target::Conditions => Panel::Conditions(mailbox.condition_status.clone()),
        Target::Rest => Panel::Rest(mailbox.rest_status.clone()),
        Target::BattleInstinct => Panel::BattleInstinct(mailbox.battle_instinct_status.clone()),
    }
}
impl PlayLink {
    pub(crate) fn condition_details_snapshot(&self, target: Target) -> Snapshot {
        let mailbox = lock(&self.mailbox);
        Snapshot {
            session: Arc::downgrade(&self.mailbox),
            target,
            panel: panel(&mailbox, target),
            live: !mailbox.quest_shutdown && !self.commands.is_closed(),
        }
    }
    /// Re-prove the session allocation, displayed target and complete owner view under
    /// the same lock that reserves and sends the query. No data is predicted here.
    pub(crate) fn condition_details_dispatch_bound(
        &self,
        expected: &Snapshot,
        action: Action,
        modal_blocked: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if modal_blocked
            || mailbox.server_modal.is_active()
            || mailbox.quest_shutdown
            || self.commands.is_closed()
            || !expected.live
            || !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || panel(&mailbox, expected.target) != expected.panel
            || (action != Action::Open && !expected.open())
            || (action == Action::Open && expected.open())
            || (action == Action::Refresh && !expected.ready())
        {
            return false;
        }
        macro_rules! dispatch {
            ($field:ident, $module:ident, $capability:expr, $kind:expr) => {{
                let panel = &mut mailbox.$field;
                let previous = panel.clone();
                if action == Action::Close {
                    panel.open = false;
                    return true;
                }
                if action == Action::Open {
                    panel.open = true;
                }
                let Some(payload) = oteryn_client_ui::$module::request(panel) else {
                    return action == Action::Open;
                };
                if self
                    .commands
                    .send(PlayCommand::Feature(FeatureCommand {
                        capability: $capability,
                        kind: $kind,
                        payload,
                    }))
                    .is_err()
                {
                    *panel = previous;
                    return false;
                }
                true
            }};
        }
        match expected.target {
            Target::Conditions => dispatch!(condition_status, condition_status, 276, 492),
            Target::Rest => dispatch!(rest_status, rest_status, 277, 493),
            Target::BattleInstinct => {
                dispatch!(battle_instinct_status, battle_instinct_status, 278, 494)
            }
        }
    }
}
#[cfg(test)]
impl PlayLink {
    pub(crate) fn condition_details_test_link(&self) -> Self {
        Self {
            owns_session: false,
            commands: self.commands.clone(),
            mailbox: self.mailbox.clone(),
        }
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn condition_details_test_selected(&self) {
        self.condition_status_initialize(true);
        self.rest_status_initialize(true);
        self.battle_instinct_status_initialize(true);
    }
    pub(crate) fn condition_details_test_changed(&self) {
        lock(&self.0).condition_status.hidden.push(1);
    }
    pub(crate) fn condition_details_test_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn condition_details_test_reply(&self, target: Target, payload: Vec<u8>) {
        let result = oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload,
        };
        match target {
            Target::Conditions => self.condition_status_result(&result),
            Target::Rest => self.rest_status_result(&result),
            Target::BattleInstinct => self.battle_instinct_status_result(&result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn condition_details_owner_rejects_changed_target_closed_channel_and_modal() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.condition_details_test_selected();
        for target in Target::ALL {
            let snapshot = link.condition_details_snapshot(target);
            assert!(!link.condition_details_dispatch_bound(&snapshot, Action::Open, true));
            assert!(link.condition_details_snapshot(target) == snapshot);
            let mut changed = snapshot.clone();
            changed.target = match target {
                Target::Conditions => Target::Rest,
                Target::Rest | Target::BattleInstinct => Target::Conditions,
            };
            assert!(!link.condition_details_dispatch_bound(&changed, Action::Open, false));
            assert!(link.condition_details_snapshot(target) == snapshot);
            assert!(commands.try_recv().is_err());
        }
        drop(commands);
        for target in Target::ALL {
            let snapshot = link.condition_details_snapshot(target);
            assert!(!link.condition_details_dispatch_bound(&snapshot, Action::Open, false));
            assert!(link.condition_details_snapshot(target) == snapshot);
        }
    }
}
