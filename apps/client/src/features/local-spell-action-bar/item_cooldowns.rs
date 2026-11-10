//! One bounded owner read before a multiaction can use a food/potion alternative.
use super::{BarAction, ItemCooldownRead};
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use std::num::NonZeroU32;
use tokio::sync::mpsc::UnboundedSender;

fn definitions(mailbox: &Mailbox, action: &BarAction) -> Vec<u32> {
    let BarAction::Multi { actions } = action else {
        return Vec::new();
    };
    actions
        .iter()
        .filter_map(|action| match action {
            BarAction::Use { definition, .. }
                if super::item_actions::item_source(mailbox, *definition, None).is_some()
                    && !mailbox
                        .spell_catalog
                        .catalog
                        .entries()
                        .iter()
                        .any(|entry| entry.rune_item == Some(definition.get())) =>
            {
                Some(definition.get())
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn send(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>, request: ItemCooldownRead) {
    let payload = oteryn_client_ui::action_bar_cooldowns::request(&request.definitions);
    if let Some(payload) = payload
        && commands
            .send(PlayCommand::Feature(
                oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability: 463,
                    kind: 6147,
                    payload,
                },
            ))
            .is_ok()
    {
        mailbox.bar_actions.item_cooldown_read = Some(request);
    } else {
        mailbox.spell_bar.feedback = "Item cooldown query unavailable; no action sent".into();
    }
}
pub(super) fn begin(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    slot: NonZeroU32,
    action: &BarAction,
    activate: bool,
) -> bool {
    let definitions = definitions(mailbox, action);
    if definitions.is_empty() {
        return false;
    }
    if !mailbox.bar_actions.item_cooldowns_selected {
        mailbox.spell_bar.feedback = "Item cooldown data unavailable in this session".into();
        return true;
    }
    let request = ItemCooldownRead {
        slot,
        action: action.clone(),
        edit: mailbox.bar_actions.edit,
        definitions,
        activate,
        ready: false,
        cursor: mailbox.bar_actions.cursor.clone(),
    };
    if let Some(pending) = &mut mailbox.bar_actions.item_cooldown_read {
        if activate && !pending.activate {
            if pending.slot == slot && pending.action == *action && pending.edit == request.edit {
                pending.activate = true;
                pending.cursor = request.cursor;
            } else {
                mailbox.bar_actions.deferred_item_read = Some(request);
            }
        }
    } else {
        send(mailbox, commands, request);
    }
    true
}
fn current(mailbox: &Mailbox, request: &ItemCooldownRead) -> bool {
    !mailbox.world_input_blocked()
        && request.edit == mailbox.bar_actions.edit
        && mailbox
            .bar_actions
            .saved
            .0
            .get(&(request.slot.get() as usize - 1))
            == Some(&request.action)
}
impl PlayLink {
    pub(in crate::play) fn bar_item_read_pending(&self) -> bool {
        lock(&self.mailbox).bar_actions.item_cooldown_read.is_some()
    }
    pub(in crate::play) fn bar_activation_pending(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        mailbox
            .bar_actions
            .item_cooldown_read
            .as_ref()
            .is_some_and(|read| read.activate)
            || mailbox.bar_actions.deferred_item_read.is_some()
    }
    pub(in crate::play) fn pump_bar_item_cooldowns(&self, preview: bool) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox
            .bar_actions
            .item_cooldown_read
            .as_ref()
            .is_some_and(|read| !read.ready)
        {
            return;
        }
        let request = mailbox.bar_actions.item_cooldown_read.take();
        if let Some(request) = request
            && request.activate
            && current(&mailbox, &request)
        {
            let cursor = std::mem::replace(&mut mailbox.bar_actions.cursor, request.cursor);
            super::assignments::activate_without_item_read(
                &mut mailbox,
                &self.commands,
                request.slot,
            );
            mailbox.bar_actions.cursor = cursor;
        }
        if let Some(request) = mailbox.bar_actions.deferred_item_read.take() {
            if current(&mailbox, &request) {
                send(&mut mailbox, &self.commands, request);
            }
            return;
        }
        if !preview
            || !mailbox.bar_actions.item_cooldowns_selected
            || mailbox.world_input_blocked()
            || mailbox.world_context.pending
        {
            return;
        }
        let next = mailbox
            .bar_actions
            .saved
            .0
            .iter()
            .find_map(|(slot, action)| {
                definitions(&mailbox, action)
                    .iter()
                    .any(|id| {
                        !mailbox.bar_actions.item_cooldowns.contains_key(id)
                            && !mailbox.bar_actions.item_cooldown_rejected.contains(id)
                    })
                    .then(|| NonZeroU32::new(*slot as u32 + 1).map(|slot| (slot, action.clone())))
                    .flatten()
            });
        if let Some((slot, action)) = next {
            begin(&mut mailbox, &self.commands, slot, &action, false);
        }
    }
}
