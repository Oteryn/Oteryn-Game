//! Item uses preserve the current session handle and the current server target generation.
use crate::play::mailbox::Mailbox;
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemUseMode {
    Use,
    SelfActor,
    Target,
    Cursor,
    CursorPosition,
}
impl ItemUseMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Use => "use",
            Self::SelfActor => "self",
            Self::Target => "current target",
            Self::Cursor => "crosshair",
            Self::CursorPosition => "cursor position",
        }
    }
}
pub(super) fn use_item(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    definition: NonZeroU32,
    sub_type: Option<u32>,
    mode: ItemUseMode,
) {
    if (mode != ItemUseMode::Use && !mailbox.world_context.use_with)
        || mailbox.world_context.pending
    {
        mailbox.spell_bar.feedback = "Item use unavailable or awaiting the server".into();
        return;
    }
    let source = item_source(mailbox, definition, sub_type).map(|item| item.handle);
    if source.is_none() && (!mailbox.world_context.use_with || sub_type.unwrap_or(0) != 0) {
        mailbox.spell_bar.feedback =
            "That item is not visible in inventory or an open container".into();
        return;
    };
    if mode == ItemUseMode::Cursor
        || (mode == ItemUseMode::Target && item_target(mailbox, mode).is_none())
    {
        mailbox
            .world_context
            .begin_shortcut_targeting(source, source.is_none().then_some(definition));
        mailbox.world_context.rune_targeting = false;
        return;
    }
    let command = if mode == ItemUseMode::Use {
        Command {
            target: source
                .map(Target::Item)
                .unwrap_or(Target::Definition(definition)),
            action: Action::Use,
            selection: None,
        }
    } else if mode == ItemUseMode::CursorPosition {
        let Some((target, selection)) = mailbox.bar_actions.cursor.clone() else {
            mailbox.world_context.set_feedback(
                "Cursor is outside the game window or battle list".into(),
                std::time::Instant::now(),
            );
            mailbox.spell_bar.feedback = "Cursor is outside the game window or battle list".into();
            return;
        };
        Command {
            target,
            action: source
                .map(Action::UseOn)
                .unwrap_or(Action::UseDefinitionOn(definition)),
            selection: Some(selection),
        }
    } else {
        let Some(target) = item_target(mailbox, mode) else {
            mailbox.spell_bar.feedback = "No current server target is available".into();
            return;
        };
        Command {
            target: Target::Creature(target.entity),
            action: source
                .map(Action::UseOn)
                .unwrap_or(Action::UseDefinitionOn(definition)),
            selection: Some(Selection {
                position: target.position,
                binding: None,
            }),
        }
    };
    if commands
        .send(crate::combat_input::PlayCommand::WorldContext(command))
        .is_ok()
    {
        mailbox.world_context.pending = true;
    } else {
        mailbox.spell_bar.feedback = "Session unavailable".into();
    }
}

#[cfg(bevy_shell)]
impl crate::play::PlayLink {
    pub(crate) fn carried_shortcut_items(&self) -> Vec<oteryn_session::ItemEntry> {
        carried_items(&crate::play::mailbox::lock(&self.mailbox))
    }
}

/// A retained container is carried only when its handle or parent belongs to the inventory tree.
pub(in crate::play) fn carried_items(mailbox: &Mailbox) -> Vec<oteryn_session::ItemEntry> {
    let Some(inventory) = mailbox.inventory.inventory.as_ref() else {
        return Vec::new();
    };
    let mut items: Vec<_> = inventory
        .main_backpack
        .into_iter()
        .chain(inventory.entries.iter().copied())
        .chain(inventory.equipment.iter().map(|e| e.item))
        .collect();
    let mut handles: std::collections::BTreeSet<_> = items.iter().map(|item| item.handle).collect();
    if let Some(views) = mailbox.containers.views.as_ref() {
        let mut admitted = std::collections::BTreeSet::new();
        for _ in 0..views.views.len() {
            let before = admitted.len();
            for view in &views.views {
                if view.browse.is_none()
                    && !admitted.contains(&view.view_id)
                    && (handles.contains(&view.container_handle)
                        || view
                            .parent_handle
                            .is_some_and(|parent| handles.contains(&parent)))
                {
                    admitted.insert(view.view_id);
                    handles.insert(view.container_handle);
                    handles.extend(view.entries.iter().map(|item| item.handle));
                    items.extend_from_slice(&view.entries);
                }
            }
            if before == admitted.len() {
                break;
            }
        }
    }
    items.sort_by_key(|item| item.handle);
    items.dedup_by_key(|item| item.handle);
    items
}

pub(in crate::play) fn item_source(
    mailbox: &Mailbox,
    definition: NonZeroU32,
    sub_type: Option<u32>,
) -> Option<oteryn_session::ItemEntry> {
    carried_items(mailbox)
        .into_iter()
        .filter(|item| {
            item.item_definition_ref == definition
                && sub_type.is_none_or(|value| item.sub_type == value)
        })
        .min_by_key(|item| (item.count, item.handle))
}
pub(in crate::play) fn item_target(
    mailbox: &Mailbox,
    mode: ItemUseMode,
) -> Option<oteryn_session::WorldSpatialEntity> {
    match mode {
        ItemUseMode::SelfActor => mailbox.bar_actions.own_actor,
        ItemUseMode::Target => mailbox
            .battle
            .combat
            .and_then(|combat| combat.target)
            .and_then(|target| {
                mailbox
                    .battle
                    .rows
                    .iter()
                    .find(|row| row.entity == target)
                    .copied()
            }),
        ItemUseMode::Use | ItemUseMode::Cursor | ItemUseMode::CursorPosition => None,
    }
}

#[cfg(test)]
pub(in crate::play) fn editor_items(
    mailbox: &Mailbox,
) -> impl Iterator<Item = oteryn_session::ItemEntry> + '_ {
    let mut seen = std::collections::BTreeSet::new();
    carried_items(mailbox)
        .into_iter()
        .filter(move |item| seen.insert((item.item_definition_ref, item.sub_type)))
}
