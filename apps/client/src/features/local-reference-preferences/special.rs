//! Client shortcuts shared by action buttons and custom hotkeys.
use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_ui::battle::{self, Action};
use oteryn_client_ui::chat::ComposerMode;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialAction {
    ToggleChat,
    AttackNext,
    AttackPrevious,
    ToggleChase,
}

impl SpecialAction {
    pub const ALL: [Self; 4] = [
        Self::ToggleChat,
        Self::AttackNext,
        Self::AttackPrevious,
        Self::ToggleChase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ToggleChat => "Toggle WASD/chat mode",
            Self::AttackNext => "Attack next battle target",
            Self::AttackPrevious => "Attack previous battle target",
            Self::ToggleChase => "Toggle chase mode",
        }
    }

    pub(in crate::play) fn execute(
        self,
        mailbox: &mut Mailbox,
        commands: &UnboundedSender<PlayCommand>,
    ) -> bool {
        if mailbox.world_input_blocked() || mailbox.custom_hotkeys_ended {
            return false;
        }
        if self == Self::ToggleChat {
            let console = &mut mailbox.chat.console;
            console.mode = if console.can_send() {
                ComposerMode::Off
            } else {
                ComposerMode::On
            };
            console.auto_focus = false;
            return true;
        }
        if mailbox.properties.pvp_expert.pending {
            return false;
        }
        let Some(combat) = mailbox.battle.combat else {
            return false;
        };
        let action = if self == Self::ToggleChase {
            use oteryn_client_runtime::attack::ChaseMode;
            Action::Chase(match combat.modes.chase {
                ChaseMode::Stand => ChaseMode::Chase,
                ChaseMode::Chase => ChaseMode::Stand,
            })
        } else {
            let Some(list) = mailbox.battle_lists.preferences.first() else {
                return false;
            };
            let rows =
                list.rows_with_metadata(&mailbox.battle.rows, &mailbox.battle.metadata.values);
            if rows.is_empty() {
                return false;
            }
            let index = rows
                .iter()
                .position(|row| Some(row.entity) == combat.target)
                .map_or(0, |index| {
                    if self == Self::AttackPrevious {
                        (index + rows.len() - 1) % rows.len()
                    } else {
                        (index + 1) % rows.len()
                    }
                });
            Action::Attack(rows[index].entity)
        };
        let Some(command) = battle::request(&mut mailbox.battle, action) else {
            return false;
        };
        mailbox.cancel_walk_for_combat(command);
        if commands.send(PlayCommand::Battle(command)).is_err() {
            mailbox.battle.pending = false;
            mailbox.battle.pending_command = None;
            return false;
        }
        true
    }
}
