//! Item uses preserve the current session handle and the current server target generation.
use crate::play::mailbox::Mailbox;
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemUseMode {
    SelfActor,
    Target,
    Cursor,
}
impl ItemUseMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::SelfActor => "self",
            Self::Target => "current target",
            Self::Cursor => "cursor",
        }
    }
}
pub(super) fn use_item(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    definition: NonZeroU32,
    mode: ItemUseMode,
) {
    if !mailbox.world_context.use_with || mailbox.world_context.pending {
        mailbox.spell_bar.feedback = "Item use unavailable or awaiting the server".into();
        return;
    }
    let inventory = mailbox.inventory.rows();
    let source = inventory
        .iter()
        .chain(
            mailbox
                .containers
                .views
                .iter()
                .flat_map(|views| &views.views)
                .flat_map(|view| &view.entries),
        )
        .filter(|item| item.item_definition_ref == definition)
        .min_by_key(|item| (item.count, item.handle))
        .map(|item| item.handle);
    let Some(source) = source else {
        mailbox.spell_bar.feedback =
            "That item is not visible in inventory or an open container".into();
        return;
    };
    if mode == ItemUseMode::Cursor {
        mailbox.world_context.begin_targeting(source);
        mailbox.world_context.rune_targeting = false;
        return;
    }
    let target = match mode {
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
        ItemUseMode::Cursor => None,
    };
    let Some(target) = target else {
        mailbox.spell_bar.feedback = "No current server target is available".into();
        return;
    };
    let command = Command {
        target: Target::Creature(target.entity),
        action: Action::UseOn(source),
        selection: Some(Selection {
            position: target.position,
            binding: None,
        }),
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
