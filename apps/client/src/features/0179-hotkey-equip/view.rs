#[path = "preferences.rs"]
mod preferences;
use crate::play::{PlayLink, mailbox::lock};
pub use preferences::ItemHotkeys;
pub(super) use preferences::edited;
pub(crate) use preferences::supported_capabilities;
impl PlayLink {
    #[must_use]
    pub fn hotkey_equip_panel(&self) -> oteryn_client_ui::hotkey_equip::HotkeyEquipPanel {
        lock(&self.mailbox).hotkey_equip.clone()
    }
}
#[path = "native.rs"]
pub(in crate::play) mod native;

/// Item hotkeys for the shared frontend; the native key and item lists are row numbers here.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::hotkey_equip::{ITEM_HOTKEY_SLOTS, assign, control_request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Item hotkeys — Ctrl+Shift+E"),
        labels: &["Key number (1 is F1)", "Item row number", "Item definition to equip"],
        buttons: &[
            "Assign item",
            "Clear key",
            "Use key",
            "Smart equip (applies at next login)",
            "Equip definition",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.hotkey_equip;
            let mut text = String::new();
            for (slot, definition) in panel.bindings.iter().enumerate() {
                let item = definition.map_or_else(
                    || tr!("Unassigned").to_owned(),
                    |value| lfmt!("Item {value}", value = value),
                );
                text.push_str(&format!("F{}: {item}\n", slot + 1));
            }
            for (row, definition) in panel.items.iter().enumerate() {
                text.push_str(&format!(
                    "{}. {}\n",
                    row + 1,
                    lfmt!("Item {definition}", definition = definition)
                ));
            }
            let smart = if panel.smart { "[x]" } else { "[ ]" };
            text.push_str(&format!("{smart} Smart equip\n"));
            text.push_str(&native::panel_text(panel));
            (panel.open, panel.pending, vec![String::new(); 3], text)
        },
        set_open: |mailbox, value| mailbox.hotkey_equip.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let panel = &mut mailbox.hotkey_equip;
            // A key must be typed explicitly: an empty field never stands for F1.
            let slot = field(&values, 0)
                .parse::<usize>()
                .ok()
                .and_then(|key| key.checked_sub(1))
                .filter(|slot| *slot < ITEM_HOTKEY_SLOTS);
            let command = match (action, slot) {
                (0..=2, None) => {
                    panel.feedback = Some(tr!("Enter a key number from 1 to 12").into());
                    None
                }
                (0, Some(slot)) => {
                    if let Some(&definition) = field(&values, 1)
                        .parse::<usize>()
                        .ok()
                        .and_then(|row| row.checked_sub(1))
                        .and_then(|row| panel.items.get(row))
                        && assign(panel, slot, definition)
                    {
                        panel.feedback = None;
                        edited(panel);
                    } else {
                        panel.feedback = Some(tr!("Enter an item row from the list").into());
                    }
                    None
                }
                (1, Some(slot)) => {
                    if assign(panel, slot, 0) {
                        panel.feedback = None;
                        edited(panel);
                    }
                    None
                }
                (2, Some(slot)) => {
                    if let Some(definition) = panel.bindings[slot] {
                        panel.item_definition = definition.get();
                        control_request(panel)
                    } else {
                        panel.feedback = Some(tr!("No item is assigned to this key").into());
                        None
                    }
                }
                (3, _) => {
                    panel.smart = !panel.smart;
                    edited(panel);
                    None
                }
                (4, _) => native::submit(panel, field(&values, 2)),
                _ => None,
            };
            if let Some(command) = command
                && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some(tr!("The session has ended").into());
            }
        },
    }
}
