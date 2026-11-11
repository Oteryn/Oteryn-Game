//! Persisted heterogeneous slot assignments; every active action uses an existing command path.
use super::item_actions::item_source;
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellAim {
    Facing,
    Cursor,
    #[default]
    Target,
}
impl SpellAim {
    /// Dialog order of "Target Options".
    pub const ALL: [Self; 3] = [Self::Target, Self::Facing, Self::Cursor];
    pub fn option(self) -> oteryn_client_ui::spell_action_bar::TargetOption {
        use oteryn_client_ui::spell_action_bar::TargetOption;
        match self {
            Self::Target => TargetOption::TowardsTarget,
            Self::Facing => TargetOption::FacingDirection,
            Self::Cursor => TargetOption::TowardsCursor,
        }
    }
}
/// Cardinal direction from the caster to the cursor tile: east/west win ties over north/south.
pub(crate) fn cursor_direction(
    own: (i32, i32),
    cursor: (i32, i32),
) -> Option<oteryn_session::StepDirection> {
    use oteryn_session::StepDirection;
    let (dx, dy) = (cursor.0 - own.0, cursor.1 - own.1);
    if dx == 0 && dy == 0 {
        None
    } else if dx.abs() >= dy.abs() {
        Some(if dx < 0 {
            StepDirection::West
        } else {
            StepDirection::East
        })
    } else {
        Some(if dy < 0 {
            StepDirection::North
        } else {
            StepDirection::South
        })
    }
}
#[cfg(test)]
mod cursor_direction_tests {
    use super::*;
    use oteryn_session::StepDirection;
    #[test]
    fn cursor_direction_prefers_east_west_on_ties_and_the_dominant_axis() {
        assert_eq!(
            cursor_direction((10, 10), (12, 12)),
            Some(StepDirection::East)
        );
        assert_eq!(
            cursor_direction((10, 10), (8, 8)),
            Some(StepDirection::West)
        );
        assert_eq!(
            cursor_direction((10, 10), (11, 13)),
            Some(StepDirection::South)
        );
        assert_eq!(
            cursor_direction((10, 10), (9, 6)),
            Some(StepDirection::North)
        );
        assert_eq!(cursor_direction((10, 10), (10, 10)), None);
    }
    #[test]
    fn spell_aim_options_follow_the_dialog_order_and_labels() {
        assert_eq!(
            SpellAim::ALL.map(|aim| aim.option().label()),
            [
                "Cast towards target",
                "Cast in facing direction",
                "Cast towards cursor position"
            ]
        );
    }
}
/// Highest item upgrade tier an Equip hotkey can be bound to.
pub const MAX_UPGRADE_TIER: u8 = 10;
fn send_by_default() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum BarAction {
    /// Live catalog identity, resolved again on every activation.
    Spell {
        name: String,
        words: String,
        #[serde(default)]
        aim: SpellAim,
    },
    Multi {
        actions: Vec<BarAction>,
    },
    Equip {
        definition: NonZeroU32,
        /// Upgrade tier the hotkey is bound to (0 = any); the equip request carries no tier yet.
        #[serde(default)]
        upgrade_tier: u8,
    },
    Text {
        text: String,
        /// `true` sends the text at once, `false` inserts it into the chat input.
        #[serde(default = "send_by_default")]
        send_automatically: bool,
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
            *slot < oteryn_client_runtime::spell::ACTION_BAR_SLOTS && valid_action(action)
        })
    }
}
fn valid_leaf(action: &BarAction) -> bool {
    match action {
        BarAction::Spell { name, words, .. } => {
            !name.is_empty()
                && name.len() <= 128
                && words.len() <= 128
                && !name.chars().chain(words.chars()).any(char::is_control)
        }
        BarAction::Text { text, .. } => {
            !text.trim().is_empty() && text.len() <= 1020 && !text.contains('\0')
        }
        BarAction::Equip { upgrade_tier, .. } => *upgrade_tier <= MAX_UPGRADE_TIER,
        BarAction::Multi { .. } | BarAction::Passive { .. } => false,
        _ => true,
    }
}
fn valid_action(action: &BarAction) -> bool {
    match action {
        BarAction::Multi { actions } => {
            (1..=3).contains(&actions.len())
                && actions
                    .iter()
                    .all(|action| valid_leaf(action) && !matches!(action, BarAction::Text { .. }))
        }
        BarAction::Passive { .. } => true,
        _ => valid_leaf(action),
    }
}
#[derive(Debug)]
pub(crate) struct ItemCooldownRead {
    pub slot: NonZeroU32,
    pub action: BarAction,
    pub edit: u64,
    pub definitions: Vec<u32>,
    pub activate: bool,
    pub ready: bool,
    pub cursor: Option<(
        oteryn_client_runtime::world_interaction::Target,
        oteryn_client_runtime::world_interaction::Selection,
    )>,
}
#[derive(Debug, Default)]
pub(crate) struct BarActionsState {
    pub saved: BarActions,
    pub item_cooldowns_selected: bool,
    pub item_cooldowns: BTreeMap<u32, (std::time::Instant, u64)>,
    pub item_cooldown_read: Option<ItemCooldownRead>,
    pub deferred_item_read: Option<ItemCooldownRead>,
    pub item_cooldown_rejected: std::collections::BTreeSet<u32>,
    pub layout: super::layout::BarLayout,
    pub keys: super::hotkeys::ActionKeys,
    pub reserved: Vec<(
        oteryn_input_actions::KeyCode,
        oteryn_input_actions::Modifiers,
    )>,
    pub own_actor: Option<oteryn_session::WorldSpatialEntity>,
    pub cursor: Option<(
        oteryn_client_runtime::world_interaction::Target,
        oteryn_client_runtime::world_interaction::Selection,
    )>,
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
    if let BarAction::Equip { definition, .. } = &action
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
    let (handled, insert) = {
        let mut mailbox = lock(&link.mailbox);
        let handled = activate_shared(&mut mailbox, &link.commands, slot);
        (handled, mailbox.custom_chat_paste.is_some())
    };
    if insert {
        link.open_chat();
    }
    handled
}
pub(in crate::play) fn activate_shared(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    slot: NonZeroU32,
) -> bool {
    activate_inner(mailbox, commands, slot, true)
}
pub(super) fn activate_without_item_read(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    slot: NonZeroU32,
) -> bool {
    activate_inner(mailbox, commands, slot, false)
}
fn activate_inner(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    slot: NonZeroU32,
    read_items: bool,
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
    if read_items && super::item_cooldowns::begin(mailbox, commands, slot, &action, true) {
        return true;
    }
    let action = if let BarAction::Multi { actions } = action {
        let Some(action) = actions
            .iter()
            .find(|action| action_ready(mailbox, action))
            .cloned()
        else {
            mailbox.spell_bar.feedback =
                "All multiaction slots are cooling down or unavailable".into();
            return true;
        };
        action
    } else {
        action
    };
    match action {
        BarAction::Spell { name, words, aim } => {
            if let Some(entry) = mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .find(|entry| entry.name == name && entry.words == words && entry.available)
                && let Some(index) = NonZeroU32::new(entry.cast_index)
            {
                let command = match aim {
                    SpellAim::Target => crate::combat_input::CombatCommand::Cast(index),
                    SpellAim::Facing => crate::combat_input::CombatCommand::CastFacing {
                        spell: index,
                        direction: None,
                    },
                    SpellAim::Cursor => {
                        let Some((_, cursor)) = &mailbox.bar_actions.cursor else {
                            mailbox.spell_bar.feedback =
                                "Cursor is outside the game window or battle list".into();
                            return true;
                        };
                        let Some(actor) = mailbox.bar_actions.own_actor else {
                            return true;
                        };
                        let direction = cursor_direction(
                            (actor.position.x, actor.position.y),
                            (cursor.position.x, cursor.position.y),
                        );
                        crate::combat_input::CombatCommand::CastFacing {
                            spell: index,
                            direction,
                        }
                    }
                };
                let _ = commands.send(crate::combat_input::PlayCommand::Combat(command));
            } else {
                mailbox.spell_bar.feedback = "Spell unavailable in the live catalogue".into();
            }
        }
        BarAction::Multi { .. } => {}
        BarAction::Use { definition, mode } => {
            super::item_actions::use_item(mailbox, commands, definition, mode)
        }
        BarAction::Equip { definition, .. } => {
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
        BarAction::Text {
            text,
            send_automatically: false,
        } => {
            use oteryn_client_ui::chat::ComposerMode;
            mailbox.chat.text.clone_from(&text);
            mailbox.custom_chat_paste = Some(text);
            if mailbox.chat.console.mode == ComposerMode::Off {
                mailbox.chat.console.mode = ComposerMode::Temporary;
            }
        }
        BarAction::Text { text, .. } => {
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
        BarAction::Spell { name, .. } => name.clone(),
        BarAction::Multi { actions } => format!("Multiaction ({} actions)", actions.len()),
        BarAction::Use { definition, mode } => format!(
            "Use {definition} on {}\nVisible: {}",
            mode.label(),
            super::amount::visible_count(mailbox, definition.get())
                .map_or_else(|| "?".into(), |count| count.to_string())
        ),
        BarAction::Equip {
            definition,
            upgrade_tier,
        } => format!(
            "Equip {definition}{}\n{}\nVisible: {}",
            if *upgrade_tier == 0 {
                String::new()
            } else {
                format!(" (tier {upgrade_tier})")
            },
            if mailbox.hotkey_equip.items.contains(&definition.get()) {
                "Carried"
            } else {
                "Unavailable"
            },
            super::amount::visible_count(mailbox, definition.get())
                .map_or_else(|| "?".into(), |count| count.to_string())
        ),
        BarAction::Text {
            text,
            send_automatically,
        } => format!(
            "{}: {text}",
            if *send_automatically { "Say" } else { "Insert" }
        ),
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

pub(in crate::play) fn slot_label(mailbox: &Mailbox, _slot: usize, action: &BarAction) -> String {
    if let BarAction::Multi { actions } = action {
        return actions
            .iter()
            .enumerate()
            .find_map(|(index, action)| {
                if let BarAction::Use { definition, .. } = action
                    && item_source(mailbox, *definition, None).is_some()
                    && !mailbox
                        .spell_catalog
                        .catalog
                        .entries()
                        .iter()
                        .any(|entry| entry.rune_item == Some(definition.get()))
                    && !mailbox
                        .bar_actions
                        .item_cooldowns
                        .contains_key(&definition.get())
                {
                    return Some(
                        if !mailbox.bar_actions.item_cooldowns_selected
                            || mailbox
                                .bar_actions
                                .item_cooldown_rejected
                                .contains(&definition.get())
                        {
                            "Item cooldown data unavailable"
                        } else {
                            "Checking item cooldowns"
                        }
                        .into(),
                    );
                }
                action_ready(mailbox, action).then(|| {
                    format!(
                        "Next {}/{}: {}",
                        index + 1,
                        actions.len(),
                        label(mailbox, action)
                    )
                })
            })
            .unwrap_or_else(|| "Multiaction: cooling down".into());
    }
    label(mailbox, action)
}
fn action_ready(mailbox: &Mailbox, action: &BarAction) -> bool {
    let entry = match action {
        BarAction::Spell { name, words, .. } => mailbox
            .spell_catalog
            .catalog
            .entries()
            .iter()
            .find(|entry| entry.name == *name && entry.words == *words),
        BarAction::Use { definition, .. } => {
            if item_source(mailbox, *definition, None).is_none() || mailbox.world_context.pending {
                return false;
            }
            mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .find(|entry| entry.rune_item == Some(definition.get()))
        }
        BarAction::Equip { definition, .. } => {
            return mailbox.hotkey_equip.items.contains(&definition.get())
                && !mailbox.hotkey_equip.pending;
        }
        _ => return false,
    };
    let Some(entry) = entry else {
        return match action {
            BarAction::Use { definition, .. } => mailbox
                .bar_actions
                .item_cooldowns
                .get(&definition.get())
                .is_some_and(|(received, remaining)| {
                    u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX) >= *remaining
                }),
            _ => false,
        };
    };
    if !entry.available {
        return false;
    }
    let elapsed = mailbox
        .spell_bar
        .cooldown_received_at
        .map_or(0, |received| {
            u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
    NonZeroU32::new(entry.cast_index).is_some_and(|index| {
        mailbox
            .spell_bar
            .bar
            .remaining_ms(index, &entry.cooldown_groups)
            <= elapsed
    })
}
