//! Custom actions use the same live session callers as inventory, chat and cooldown panels.
use super::{CustomAction, ObjectTarget};
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::world_interaction::{Action, Command, Target};

impl PlayLink {
    pub fn custom_hotkey_choices(&self) -> Vec<(String, CustomAction)> {
        let mailbox = lock(&self.mailbox);
        let mut choices = Vec::new();
        for item in mailbox.inventory.rows() {
            let action = CustomAction::Object {
                definition: item.item_definition_ref.get(),
                sub_type: item.sub_type,
                target: ObjectTarget::Use,
            };
            if !choices.iter().any(|(_, existing)| *existing == action) {
                choices.push((
                    format!(
                        "Inventory item {} (subtype {})",
                        item.item_definition_ref, item.sub_type
                    ),
                    action,
                ));
            }
        }
        for definition in &mailbox.hotkey_equip.items {
            choices.push((
                format!("Equip / unequip item {definition}"),
                CustomAction::Object {
                    definition: *definition,
                    sub_type: 0,
                    target: ObjectTarget::Equip,
                },
            ));
        }
        for entry in mailbox.spell_catalog.catalog.entries() {
            choices.push((
                format!(
                    "Cooldown: {}{}",
                    entry.name,
                    if entry.learned { "" } else { " (not learned)" }
                ),
                CustomAction::Passive {
                    cast_index: entry.cast_index,
                },
            ));
        }
        choices
    }

    pub fn custom_hotkey_action(&self, view: &PlayView, action: &CustomAction) -> bool {
        if !action.valid() {
            return false;
        }
        match action {
            CustomAction::Text { text, send } => {
                use oteryn_client_ui::chat::{ComposerMode, Mode};
                let mut mailbox = lock(&self.mailbox);
                if mailbox.chat_public_shutdown || !mailbox.chat.selected || mailbox.chat.pending {
                    return false;
                }
                if *send {
                    let old = std::mem::replace(&mut mailbox.chat.text, text.clone());
                    let mode = mailbox.chat.console.mode;
                    if mode == ComposerMode::Off {
                        mailbox.chat.console.mode = ComposerMode::Temporary;
                    }
                    crate::play::feature_0007_chat::input::enqueue(
                        &mut mailbox,
                        &self.commands,
                        Mode::Say,
                        "",
                    );
                    mailbox.chat.text = old;
                    mailbox.chat.console.mode = mode;
                    mailbox.chat.pending
                } else {
                    mailbox.chat.text.clone_from(text);
                    mailbox.custom_chat_paste = Some(text.clone());
                    if mailbox.chat.console.mode == ComposerMode::Off {
                        mailbox.chat.console.mode = ComposerMode::Temporary;
                    }
                    drop(mailbox);
                    self.open_chat();
                    true
                }
            }
            CustomAction::Passive { cast_index } => {
                let mut mailbox = lock(&self.mailbox);
                if mailbox.quest_shutdown
                    || !mailbox
                        .spell_catalog
                        .catalog
                        .entries()
                        .iter()
                        .any(|entry| entry.cast_index == *cast_index)
                {
                    return false;
                }
                mailbox.tracked_custom_ability = Some(*cast_index);
                mailbox.custom_hotkey_feedback = None;
                true
            }
            CustomAction::Object {
                definition,
                sub_type,
                target,
            } => {
                let mut mailbox = lock(&self.mailbox);
                if mailbox.inventory_shutdown
                    || mailbox.world_input_blocked()
                    || mailbox.world_context.pending
                    || mailbox.world_context.targeting.is_some()
                {
                    return false;
                }
                if *target == ObjectTarget::Equip {
                    if !mailbox.hotkey_equip.items.contains(definition) {
                        return false;
                    }
                    let panel = &mut mailbox.hotkey_equip;
                    let old = std::mem::replace(&mut panel.item_definition, *definition);
                    let command = oteryn_client_ui::hotkey_equip::control_request(panel);
                    panel.item_definition = old;
                    let Some(command) = command else {
                        return false;
                    };
                    if self
                        .commands
                        .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                        .is_err()
                    {
                        panel.pending = false;
                        return false;
                    }
                    return true;
                }
                let Some(item) = mailbox.inventory.rows().into_iter().find(|item| {
                    item.item_definition_ref.get() == *definition && item.sub_type == *sub_type
                }) else {
                    mailbox.custom_hotkey_feedback =
                        Some("Custom hotkey: item is no longer in inventory".into());
                    return false;
                };
                if *target != ObjectTarget::Use && !mailbox.world_context.use_with {
                    return false;
                }
                let target = match target {
                    ObjectTarget::Use => Target::Item(item.handle),
                    ObjectTarget::Cursor => {
                        let rune = mailbox
                            .spell_catalog
                            .catalog
                            .entries()
                            .iter()
                            .any(|entry| entry.rune_item == Some(*definition));
                        mailbox.world_context.begin_targeting(item.handle);
                        mailbox.world_context.rune_targeting = rune;
                        return true;
                    }
                    ObjectTarget::SelfActor => Target::Position(oteryn_session::ActorPosition {
                        x: view.own.x,
                        y: view.own.y,
                        floor: view.floor,
                    }),
                    ObjectTarget::Equip => return false,
                    ObjectTarget::CurrentTarget => {
                        let Some(reference) =
                            view.scene.target().and_then(|target| target.entity).filter(
                                |reference| {
                                    view.entities
                                        .iter()
                                        .any(|entity| entity.entity == *reference)
                                },
                            )
                        else {
                            return false;
                        };
                        Target::Creature(reference)
                    }
                };
                let command = Command {
                    target,
                    action: if matches!(
                        action,
                        CustomAction::Object {
                            target: ObjectTarget::Use,
                            ..
                        }
                    ) {
                        Action::Use
                    } else {
                        Action::UseOn(item.handle)
                    },
                    selection: None,
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::WorldContext(command))
                    .is_err()
                {
                    return false;
                }
                mailbox.world_context.pending = true;
                mailbox.custom_hotkey_feedback = None;
                true
            }
        }
    }

    /// Track server-owned cooldowns without claiming an unlocked or executable ability.
    pub fn custom_hotkey_status(&self) -> String {
        let mailbox = lock(&self.mailbox);
        if let Some(feedback) = &mailbox.custom_hotkey_feedback {
            return feedback.clone();
        }
        let Some(index) = mailbox.tracked_custom_ability else {
            return String::new();
        };
        let Some(entry) = mailbox
            .spell_catalog
            .catalog
            .entries()
            .iter()
            .find(|entry| entry.cast_index == index)
        else {
            return "Tracked ability unavailable in this session".into();
        };
        let Some(spell) = std::num::NonZeroU32::new(index) else {
            return String::new();
        };
        if mailbox.spell_bar.bar.cooldowns.is_none() {
            return format!("{}: cooldown unknown", entry.name);
        }
        format!(
            "{}: server cooldown {} ms",
            entry.name,
            mailbox
                .spell_bar
                .bar
                .remaining_ms(spell, &entry.cooldown_groups)
        )
    }
}
