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
    let Some(source) = source else {
        mailbox.spell_bar.feedback =
            "That item is not visible in inventory or an open container".into();
        return;
    };
    if mode == ItemUseMode::Cursor
        || (mode == ItemUseMode::Target && item_target(mailbox, mode).is_none())
    {
        mailbox.world_context.begin_targeting(source);
        mailbox.world_context.rune_targeting = false;
        return;
    }
    let command = if mode == ItemUseMode::Use {
        Command {
            target: Target::Item(source),
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
            action: Action::UseOn(source),
            selection: Some(selection),
        }
    } else {
        let Some(target) = item_target(mailbox, mode) else {
            mailbox.spell_bar.feedback = "No current server target is available".into();
            return;
        };
        Command {
            target: Target::Creature(target.entity),
            action: Action::UseOn(source),
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

pub(in crate::play) fn visible_items(
    mailbox: &Mailbox,
) -> impl Iterator<Item = oteryn_session::ItemEntry> + '_ {
    mailbox.inventory.rows().into_iter().chain(
        mailbox
            .containers
            .views
            .iter()
            .flat_map(|views| &views.views)
            .flat_map(|view| view.entries.iter())
            .copied(),
    )
}
pub(in crate::play) fn item_source(
    mailbox: &Mailbox,
    definition: NonZeroU32,
    sub_type: Option<u32>,
) -> Option<oteryn_session::ItemEntry> {
    visible_items(mailbox)
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
    visible_items(mailbox)
        .filter(move |item| seen.insert((item.item_definition_ref, item.sub_type)))
}
