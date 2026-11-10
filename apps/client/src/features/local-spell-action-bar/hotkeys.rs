//! Custom slot chords share the normal action executor and respect live chat focus.
use crate::play::{
    PlayLink, ReferencePreferences,
    mailbox::{Mailbox, lock},
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
type Chord = (KeyCode, Modifiers);
fn parse(chord: &str) -> Option<Chord> {
    crate::play::feature_local_reference_preferences::settings::parse_action_chord(chord)
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionKeys(pub BTreeMap<usize, String>);
pub(super) fn reservations(preferences: &ReferencePreferences) -> Vec<Chord> {
    let mut chords = vec!["ESCAPE", "ENTER", "KEYPADENTER"]
        .into_iter()
        .filter_map(parse)
        .collect::<Vec<_>>();
    if let Ok(bindings) = crate::play::preference_bindings() {
        chords.extend(
            bindings
                .iter()
                .filter_map(|binding| parse(binding.default_chord)),
        );
        chords.extend(
            bindings
                .iter()
                .filter(|binding| {
                    !binding
                        .id
                        .strip_prefix("local-spell-action-bar-spell-")
                        .and_then(|slot| slot.parse::<usize>().ok())
                        .is_some_and(|slot| slot > 4)
                })
                .filter_map(|binding| parse(preferences.binding_chord(binding))),
        );
    }
    chords.extend(
        preferences
            .secondary_keybindings
            .values()
            .filter_map(|value| parse(value)),
    );
    for preset in preferences.hotkey_contexts.presets.values() {
        for context in [&preset.chat_on, &preset.chat_off] {
            chords.extend(
                context
                    .primary
                    .values()
                    .chain(context.secondary.values())
                    .filter_map(|value| parse(value)),
            );
        }
    }
    chords.extend(
        preferences
            .custom_hotkeys
            .keys()
            .filter_map(|key| parse(key)),
    );
    chords
}
impl ActionKeys {
    pub fn valid(&self, preferences: &ReferencePreferences) -> bool {
        let blocked = reservations(preferences);
        let mut seen = Vec::new();
        self.0.iter().all(|(slot, value)| {
            if *slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS || value.len() > 64 {
                return false;
            }
            let Some(chord) = parse(value) else {
                return false;
            };
            if blocked.contains(&chord) || seen.contains(&chord) {
                return false;
            }
            seen.push(chord);
            true
        })
    }
}
pub(in crate::play) fn bind(mailbox: &mut Mailbox, slot: usize, value: &str) -> bool {
    if slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS || value.len() > 64 {
        return false;
    }
    if value.trim().is_empty() {
        mailbox.bar_actions.keys.0.remove(&slot);
        mailbox.bar_actions.edited();
        return true;
    }
    let Some(chord) = parse(value) else {
        return false;
    };
    if mailbox.bar_actions.reserved.contains(&chord)
        || ["ESCAPE", "ENTER", "KEYPADENTER"]
            .iter()
            .filter_map(|value| parse(value))
            .any(|reserved| reserved == chord)
        || mailbox
            .bar_actions
            .keys
            .0
            .iter()
            .any(|(other, value)| *other != slot && parse(value) == Some(chord))
    {
        return false;
    }
    mailbox
        .bar_actions
        .keys
        .0
        .insert(slot, value.trim().to_ascii_uppercase());
    mailbox.bar_actions.edited();
    true
}
impl PlayLink {
    pub fn bind_action_key(&self, slot: usize, chord: &str) -> bool {
        bind(&mut lock(&self.mailbox), slot, chord)
    }
    pub fn custom_hotkeys_input(&self, events: &[NormalizedInputEvent]) -> bool {
        let slots = {
            let mailbox = lock(&self.mailbox);
            events
                .iter()
                .filter_map(|event| {
                    let NormalizedInputEvent::Key {
                        code,
                        state: ButtonState::Pressed,
                        modifiers,
                        repeat: false,
                    } = event
                    else {
                        return None;
                    };
                    if mailbox.chat.open
                        && ((4..=39).contains(&code.get()) || code.get() == 44)
                        && ![Modifier::Control, Modifier::Alt, Modifier::Super]
                            .iter()
                            .any(|modifier| modifiers.contains(*modifier))
                    {
                        return None;
                    }
                    mailbox
                        .bar_actions
                        .keys
                        .0
                        .iter()
                        .find(|(_, value)| parse(value) == Some((*code, *modifiers)))
                        .map(|(slot, _)| *slot)
                })
                .collect::<Vec<_>>()
        };
        for slot in &slots {
            if let Some(slot) = u32::try_from(slot + 1)
                .ok()
                .and_then(std::num::NonZeroU32::new)
            {
                self.cast_spell_slot(slot);
            }
        }
        !slots.is_empty()
    }
}

/// Battle-list child windows deliver native key messages rather than winit events.
#[cfg(any(windows, test))]
pub(in crate::play) fn native_key(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    virtual_key: u32,
    modifiers: Modifiers,
) -> bool {
    let code = match virtual_key {
        0x41..=0x5a => virtual_key - 0x41 + 4,
        0x31..=0x39 => virtual_key - 0x31 + 30,
        0x30 => 39,
        0x70..=0x7b => virtual_key - 0x70 + 58,
        0x7c..=0x87 => virtual_key - 0x7c + 104,
        0x20 => 44,
        0x25 => 80,
        0x26 => 82,
        0x27 => 79,
        0x28 => 81,
        _ => return false,
    };
    let Ok(code) = KeyCode::new(code as u16) else {
        return false;
    };
    let slot = mailbox
        .bar_actions
        .keys
        .0
        .iter()
        .find(|(_, value)| parse(value) == Some((code, modifiers)))
        .map(|(slot, _)| *slot)
        .or_else(|| {
            crate::play::preference_bindings()
                .ok()?
                .iter()
                .find_map(|binding| {
                    let slot = binding
                        .id
                        .strip_prefix("local-spell-action-bar-spell-")?
                        .parse::<usize>()
                        .ok()?
                        .checked_sub(1)?;
                    let preferences = &mailbox.local_preferences;
                    (parse(preferences.binding_chord(binding)) == Some((code, modifiers))
                        || preferences
                            .binding_secondary(binding)
                            .is_some_and(|chord| parse(chord) == Some((code, modifiers))))
                    .then_some(slot)
                })
        });
    let Some(slot) = slot
        .and_then(|slot| u32::try_from(slot + 1).ok())
        .and_then(std::num::NonZeroU32::new)
    else {
        return false;
    };
    super::super::input::submit(mailbox, commands, slot.get() as usize, &[]);
    true
}
