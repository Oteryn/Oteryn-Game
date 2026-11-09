//! Persisted heterogeneous slot assignments; every active action uses an existing command path.
use crate::play::{
    PlayLink, SettingsState,
    mailbox::{Mailbox, lock},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU32};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PassiveAbility {
    BattleInstinct,
    Harmony,
    Serene,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum BarAction {
    Equip {
        definition: NonZeroU32,
    },
    Text {
        text: String,
    },
    Use {
        definition: NonZeroU32,
        mode: super::item_actions::ItemUseMode,
    },
    Passive {
        ability: PassiveAbility,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BarActions(pub BTreeMap<usize, BarAction>);
impl BarActions {
    pub fn valid(&self) -> bool {
        self.0.iter().all(|(slot, action)| {
            *slot < oteryn_client_runtime::spell::ACTION_BAR_SLOTS
                && match action {
                    BarAction::Text { text } => {
                        !text.trim().is_empty() && text.len() <= 1020 && !text.contains('\0')
                    }
                    _ => true,
                }
        })
    }
}
#[derive(Debug, Default)]
pub(crate) struct BarActionsState {
    pub saved: BarActions,
    pub layout: super::layout::BarLayout,
    pub keys: super::hotkeys::ActionKeys,
    pub reserved: Vec<(
        oteryn_input_actions::KeyCode,
        oteryn_input_actions::Modifiers,
    )>,
    pub own_actor: Option<oteryn_session::WorldSpatialEntity>,
    pub dirty: bool,
    pub edit: u64,
}
impl BarActionsState {
    pub(super) fn edited(&mut self) {
        self.dirty = true;
        self.edit = crate::play::feature_0017_attack::view::next_edit();
    }
}
pub(in crate::play) fn clear_extra(mailbox: &mut Mailbox, slot: usize) {
    if mailbox.bar_actions.saved.0.remove(&slot).is_some() {
        mailbox.bar_actions.edited();
    }
}
impl PlayLink {
    pub fn assign_bar_action(&self, slot: usize, action: BarAction) -> bool {
        set_action(&mut lock(&self.mailbox), slot, action)
    }

    pub fn clear_bar_action(&self, slot: usize) {
        clear_extra(&mut lock(&self.mailbox), slot);
    }
}
pub(in crate::play) fn set_action(mailbox: &mut Mailbox, slot: usize, action: BarAction) -> bool {
    if let BarAction::Equip { definition } = &action
        && !mailbox.hotkey_equip.items.contains(&definition.get())
    {
        return false;
    }
    let mut next = mailbox.bar_actions.saved.clone();
    next.0.insert(slot, action);
    if !next.valid() {
        return false;
    }
    mailbox.bar_actions.saved = next;
    mailbox.bar_actions.edited();
    true
}
impl SettingsState {
    pub fn sync_bar_actions(&mut self, link: &PlayLink) {
        let mut mailbox = lock(&link.mailbox);
        let state = &mut mailbox.bar_actions;
        state.reserved = super::hotkeys::reservations(&self.preferences);
        if state.dirty && state.edit > self.bar_actions_edit {
            let edit = state.edit;
            let mut preferences = self.preferences.clone();
            preferences.bar_actions = state.saved.clone();
            preferences.bar_layout = state.layout.clone();
            preferences.action_keys = state.keys.clone();
            state.dirty = false;
            drop(mailbox);
            self.apply(preferences);
            self.bar_actions_edit = edit;
        } else {
            state.dirty = false;
            state.saved.clone_from(&self.preferences.bar_actions);
            state.layout.clone_from(&self.preferences.bar_layout);
            state.keys.clone_from(&self.preferences.action_keys);
        }
    }
}

pub(in crate::play) fn activate(link: &PlayLink, slot: NonZeroU32) -> bool {
    activate_shared(&mut lock(&link.mailbox), &link.commands, slot)
}
pub(in crate::play) fn activate_shared(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    slot: NonZeroU32,
) -> bool {
    let Some(action) = mailbox
        .bar_actions
        .saved
        .0
        .get(&(slot.get() as usize - 1))
        .cloned()
    else {
        return false;
    };
    if mailbox.world_input_blocked() {
        return true;
    }
    match action {
        BarAction::Use { definition, mode } => {
            super::item_actions::use_item(mailbox, commands, definition, mode)
        }
        BarAction::Equip { definition } => {
            mailbox.hotkey_equip.item_definition = definition.get();
            if let Some(command) =
                oteryn_client_ui::hotkey_equip::control_request(&mut mailbox.hotkey_equip)
                && commands
                    .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                    .is_err()
            {
                mailbox.hotkey_equip.pending = false;
                mailbox.spell_bar.feedback = "Session unavailable".into();
            }
        }
        BarAction::Text { text } => {
            let draft = std::mem::replace(&mut mailbox.chat.text, text);
            crate::play::feature_0007_chat::input::enqueue(
                mailbox,
                commands,
                oteryn_client_ui::chat::Mode::Say,
                "",
            );
            mailbox.chat.text = draft;
        }
        BarAction::Passive { .. } => {
            mailbox.spell_bar.feedback = "Passive status: no cast command is sent".into();
        }
    }
    true
}

pub(in crate::play) fn label(mailbox: &Mailbox, action: &BarAction) -> String {
    match action {
        BarAction::Use { definition, mode } => format!(
            "Use {definition} on {}\nVisible: {}",
            mode.label(),
            super::amount::visible_count(mailbox, definition.get())
                .map_or_else(|| "?".into(), |count| count.to_string())
        ),
        BarAction::Equip { definition } => format!(
            "Equip {definition}\n{}\nVisible: {}",
            if mailbox.hotkey_equip.items.contains(&definition.get()) {
                "Carried"
            } else {
                "Unavailable"
            },
            super::amount::visible_count(mailbox, definition.get())
                .map_or_else(|| "?".into(), |count| count.to_string())
        ),
        BarAction::Text { text } => format!("Say: {text}"),
        BarAction::Passive { ability } => match ability {
            PassiveAbility::BattleInstinct => mailbox.battle_instinct_status.snapshot.map_or_else(
                || "Battle Instinct\nUnknown".into(),
                |state| {
                    format!(
                        "Battle Instinct\n{}",
                        if state.active { "Active" } else { "Inactive" }
                    )
                },
            ),
            PassiveAbility::Harmony => mailbox.vitals.map_or_else(
                || "Harmony\nUnknown".into(),
                |vitals| format!("Harmony {}/5", vitals.harmony),
            ),
            PassiveAbility::Serene => mailbox.vitals.map_or_else(
                || "Serene\nUnknown".into(),
                |vitals| {
                    format!(
                        "Serene\n{}",
                        if vitals.serene { "Active" } else { "Inactive" }
                    )
                },
            ),
        },
    }
}
