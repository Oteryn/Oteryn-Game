//! Custom actions use the same live session callers as inventory, chat and cooldown panels.
use super::{CustomAction, ObjectTarget};
use crate::play::feature_local_spell_action_bar::view::{item_source, item_target, visible_items};
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};

impl PlayLink {
    pub fn custom_hotkey_choices(&self) -> Vec<(String, CustomAction)> {
        let mailbox = lock(&self.mailbox);
        let mut choices = Vec::new();
        for item in visible_items(&mailbox) {
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
        for (label, ability) in [
            (
                "Passive: Battle Instinct",
                crate::play::PassiveAbility::BattleInstinct,
            ),
            ("Passive: Harmony", crate::play::PassiveAbility::Harmony),
            ("Passive: Serene", crate::play::PassiveAbility::Serene),
        ] {
            choices.push((label.into(), CustomAction::PassiveStatus { ability }));
        }
        for slot in 1..=oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
            if let Ok(number) = u32::try_from(slot) {
                choices.push((
                    format!(
                        "Action bar row {} / slot {}",
                        (slot - 1) / 12 + 1,
                        (slot - 1) % 12 + 1
                    ),
                    CustomAction::ActionBar { slot: number },
                ));
            }
        }
        choices.extend(
            super::UtilityAction::ALL
                .into_iter()
                .map(|action| (action.label().into(), CustomAction::Utility { action })),
        );
        choices
    }

    /// Session-level utility actions; preference actions run in `SettingsState::run_utility`.
    pub fn utility_action(&self, action: super::UtilityAction) -> bool {
        use super::UtilityAction;
        match action {
            UtilityAction::ClearOldestMessage => {
                let mut mailbox = lock(&self.mailbox);
                if mailbox.chat.lines.is_empty() {
                    return false;
                }
                mailbox.chat.lines.remove(0);
                true
            }
            UtilityAction::ShowIgnoreList => {
                self.chat_show_lists();
                true
            }
            UtilityAction::ToggleServerMessages => {
                self.chat_console_filter();
                true
            }
            UtilityAction::ChangeCharacter => {
                self.request_logout();
                true
            }
            UtilityAction::BugReport => {
                self.bug_report_open();
                true
            }
            UtilityAction::QuickLootArea => {
                self.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll)
            }
            UtilityAction::CloseCurrentChannel => {
                self.chat_close_current();
                true
            }
            UtilityAction::NextChannel | UtilityAction::PreviousChannel => {
                self.chat_channel_cycle(action == UtilityAction::NextChannel);
                true
            }
            UtilityAction::OpenChannelList => {
                self.chat_channel_dialog_open();
                true
            }
            UtilityAction::ToggleLensHelp => {
                let active = self.lens_toggle();
                lock(&self.mailbox).chat.feedback =
                    Some(oteryn_client_ui::lens_help::mode_text(active).into());
                self.open_chat();
                true
            }
            UtilityAction::OpenHelpChannel => {
                self.chat_open_help();
                self.open_chat();
                true
            }
            UtilityAction::ToggleCreatureHuds
            | UtilityAction::ToggleFpsLagIndicator
            | UtilityAction::NextHotkeyPreset => false,
        }
    }

    pub fn custom_hotkey_action(&self, _view: &PlayView, action: &CustomAction) -> bool {
        if !action.valid() {
            return false;
        }
        match action {
            CustomAction::PassiveStatus { ability } => {
                let mut mailbox = lock(&self.mailbox);
                if mailbox.quest_shutdown || mailbox.custom_hotkeys_ended {
                    return false;
                }
                mailbox.tracked_custom_passive = Some(*ability);
                mailbox.tracked_custom_ability = None;
                mailbox.custom_hotkey_feedback = None;
                true
            }
            CustomAction::ActionBar { slot } => {
                let Some(slot) = std::num::NonZeroU32::new(*slot) else {
                    return false;
                };
                {
                    let mailbox = lock(&self.mailbox);
                    if mailbox.quest_shutdown
                        || mailbox.custom_hotkeys_ended
                        || mailbox.world_input_blocked()
                        || (mailbox.spell_bar.bar.spell(slot).is_none()
                            && !mailbox
                                .bar_actions
                                .saved
                                .0
                                .contains_key(&(slot.get() as usize - 1)))
                    {
                        return false;
                    }
                }
                self.cast_spell_slot(slot);
                true
            }
            CustomAction::Utility { action } => self.utility_action(*action),
            CustomAction::Text { text, send } => {
                use oteryn_client_ui::chat::{ComposerMode, Mode};
                let mut mailbox = lock(&self.mailbox);
                if mailbox.custom_hotkeys_ended
                    || mailbox.chat_public_shutdown
                    || !mailbox.chat.selected
                    || mailbox.chat.pending
                {
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
                    || mailbox.custom_hotkeys_ended
                    || !mailbox
                        .spell_catalog
                        .catalog
                        .entries()
                        .iter()
                        .any(|entry| entry.cast_index == *cast_index)
                {
                    return false;
                }
                mailbox.tracked_custom_passive = None;
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
                let Some(definition_id) = std::num::NonZeroU32::new(*definition) else {
                    return false;
                };
                let Some(item) = item_source(&mailbox, definition_id, Some(*sub_type)) else {
                    mailbox.custom_hotkey_feedback =
                        Some("Custom hotkey: item is no longer visible in inventory or an open container".into());
                    return false;
                };
                if *target != ObjectTarget::Use && !mailbox.world_context.use_with {
                    return false;
                }
                let mut selection = None;
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
                    ObjectTarget::Equip => return false,
                    ObjectTarget::SelfActor | ObjectTarget::CurrentTarget => {
                        let mode = if *target == ObjectTarget::SelfActor {
                            crate::play::ItemUseMode::SelfActor
                        } else {
                            crate::play::ItemUseMode::Target
                        };
                        let Some(actor) = item_target(&mailbox, mode) else {
                            return false;
                        };
                        selection = Some(Selection {
                            position: actor.position,
                            binding: None,
                        });
                        Target::Creature(actor.entity)
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
                    selection,
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
        if mailbox.quest_shutdown || mailbox.custom_hotkeys_ended {
            return String::new();
        }
        if let Some(ability) = mailbox.tracked_custom_passive {
            return crate::play::feature_local_spell_action_bar::view::action_label(
                &mailbox,
                &crate::play::BarAction::Passive { ability },
            );
        }
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

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn end_custom_hotkeys(&mut self) {
        self.custom_hotkeys_ended = true;
        self.custom_chat_paste = None;
        self.tracked_custom_passive = None;
        self.tracked_custom_ability = None;
        self.custom_hotkey_feedback = None;
    }
}
