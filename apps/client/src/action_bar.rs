//! Configurable action slots over existing session commands.
//!
//! Only layout and physical shortcuts persist. Spell-book indices and item handles belong to
//! one admitted session and are deliberately cleared on reconnect. A future generation-pinned
//! catalogue can supply durable semantic assignments without persisting these transient IDs.
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifiers, NormalizedInputEvent,
    RepeatPolicy,
};
use oteryn_session::{
    CastOutcome, ChatIntent, ChatOutcome, ItemHandle, ItemMoveDestination, ItemMoveIntent,
    ItemMoveOutcomeResult, Session, SessionError, SessionStream, SpellTarget, UseOutcome,
};
use serde::{Deserialize, Serialize};
use std::{io, num::NonZeroU32};

pub const ACTION_BAR_SLOTS: usize = 12;
const PREFIX: &str = "client.action-slot.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotShortcut {
    /// USB HID physical position, matching the platform adapter.
    pub key: u16,
    /// The input router's Shift/Control/Alt/Super bitset.
    pub modifiers: u8,
}

impl SlotShortcut {
    pub fn chord(self) -> Result<InputChord, InputError> {
        InputChord::new(
            Modifiers::from_bits(self.modifiers)?,
            vec![InputAtom::Key(KeyCode::new(self.key)?)],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionBarPreferences {
    pub visible: bool,
    /// Locks assignment editing, never command activation.
    pub locked: bool,
    pub shortcuts: [Option<SlotShortcut>; ACTION_BAR_SLOTS],
}

impl Default for ActionBarPreferences {
    fn default() -> Self {
        let keys = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        Self {
            visible: true,
            locked: false,
            shortcuts: keys.map(|key| Some(SlotShortcut { key, modifiers: 0 })),
        }
    }
}

impl ActionBarPreferences {
    /// Reserve application Escape/Enter/F10 and the configured movement chords. Reject
    /// conflicts before replacing any live bindings or saving preferences.
    pub fn validate(&self, movement_keys: &[u16]) -> io::Result<()> {
        for (index, shortcut) in self.shortcuts.iter().enumerate() {
            let Some(shortcut) = shortcut else { continue };
            if shortcut.chord().is_err()
                || matches!(shortcut.key, 40 | 41 | 67)
                || (shortcut.modifiers == 0 && movement_keys.contains(&shortcut.key))
                || self.shortcuts[..index].contains(&Some(*shortcut))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or conflicting action shortcut",
                ));
            }
        }
        Ok(())
    }
}

/// An actual existing command, never a locally simulated gameplay result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionBarCommand {
    CastSpell {
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    },
    /// USE_INTENT's item target currently opens a visible corpse. Do not present this as
    /// consumable-item use, which requires its own engine contract and command.
    OpenItem(ItemHandle),
    MoveToBackpack(ItemHandle),
    Chat(ChatIntent),
}

#[derive(Clone, Debug)]
pub enum ActionBarOutcome {
    Unavailable,
    Cast(CastOutcome),
    Used(UseOutcome),
    Moved(ItemMoveOutcomeResult),
    Chat(ChatOutcome),
}

impl ActionBarCommand {
    /// Call only on the owning session task. Session gates capabilities, sequences the
    /// command, validates its result and applies authoritative deltas.
    pub async fn execute<S: SessionStream>(
        &self,
        session: &mut Session<S>,
    ) -> Result<ActionBarOutcome, SessionError> {
        match self {
            Self::CastSpell {
                spell,
                target,
                aim_at_target,
            } => session
                .cast_spell(*spell, *target, *aim_at_target)
                .await
                .map(ActionBarOutcome::Cast),
            Self::OpenItem(handle) => session.use_item(*handle).await.map(ActionBarOutcome::Used),
            Self::MoveToBackpack(handle) => session
                .move_item(&ItemMoveIntent {
                    source: *handle,
                    destination: ItemMoveDestination::MainBackpack,
                })
                .await
                .map(ActionBarOutcome::Moved),
            Self::Chat(intent) => session.chat(intent).await.map(ActionBarOutcome::Chat),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotAssignment {
    pub label: String,
    pub command: ActionBarCommand,
}

/// Runtime assignments are recreated for each admission; preferences alone cross sessions.
#[derive(Debug)]
pub struct ActionBar {
    preferences: ActionBarPreferences,
    assignments: [Option<SlotAssignment>; ACTION_BAR_SLOTS],
    router: InputRouter,
    text: ContextId,
    modal: ContextId,
}

impl ActionBar {
    pub fn new(preferences: ActionBarPreferences, movement_keys: &[u16]) -> io::Result<Self> {
        preferences.validate(movement_keys)?;
        Self::build(preferences).map_err(io::Error::other)
    }

    fn build(preferences: ActionBarPreferences) -> Result<Self, InputError> {
        let gameplay = ContextId::new("actionbar-gameplay".into())?;
        let text = ContextId::new("actionbar-text".into())?;
        let modal = ContextId::new("actionbar-modal".into())?;
        let mut bindings = Vec::new();
        for (index, shortcut) in preferences.shortcuts.iter().enumerate() {
            if let Some(shortcut) = shortcut {
                bindings.push(Binding::new(
                    gameplay.clone(),
                    shortcut.chord()?,
                    ActionId::new(format!("{PREFIX}{index}"))?,
                    RepeatPolicy::Ignore,
                ));
            }
        }
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
                ContextDefinition::new(modal.clone(), ContextKind::Modal, 20),
            ],
            bindings,
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self {
            preferences,
            assignments: std::array::from_fn(|_| None),
            router,
            text,
            modal,
        })
    }

    #[must_use]
    pub const fn preferences(&self) -> &ActionBarPreferences {
        &self.preferences
    }

    #[must_use]
    pub fn assignment(&self, slot: usize) -> Option<&SlotAssignment> {
        self.assignments.get(slot).and_then(Option::as_ref)
    }

    pub fn assign(&mut self, slot: usize, assignment: Option<SlotAssignment>) -> io::Result<()> {
        if self.preferences.locked {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Action bar is locked",
            ));
        }
        let destination = self
            .assignments
            .get_mut(slot)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Unknown action slot"))?;
        if let Some(assignment) = &assignment
            && (assignment.label.trim().is_empty()
                || assignment.label.len() > 80
                || assignment.label.chars().any(char::is_control))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid action label",
            ));
        }
        *destination = assignment;
        Ok(())
    }

    /// A failed rebind leaves the previous keymap and assignments intact. Replacing a valid
    /// map cancels held input, so a held key cannot fire under the new binding.
    pub fn reconfigure(
        &mut self,
        preferences: ActionBarPreferences,
        movement_keys: &[u16],
    ) -> io::Result<()> {
        let mut replacement = Self::new(preferences, movement_keys)?;
        replacement.assignments = self.assignments.clone();
        *self = replacement;
        Ok(())
    }

    pub fn set_input_context(
        &mut self,
        text_active: bool,
        modal_active: bool,
    ) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, text_active)?;
        self.router.set_context_active(&self.modal, modal_active)?;
        Ok(())
    }

    /// Process focus/release events even while text/modal input suppresses gameplay.
    /// InputRouter suppresses repeats and fires once on the physical press.
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<ActionBarCommand> {
        let slots: Vec<usize> = events
            .iter()
            .filter(|event| match event {
                NormalizedInputEvent::Key { code, .. } => {
                    matches!(code.get(), 224..=231)
                        || self
                            .preferences
                            .shortcuts
                            .iter()
                            .flatten()
                            .any(|shortcut| shortcut.key == code.get())
                }
                NormalizedInputEvent::MouseButton { .. } | NormalizedInputEvent::Wheel { .. } => {
                    false
                }
                _ => true,
            })
            .flat_map(|event| self.router.process(event))
            .filter(|event| event.phase() == ActionPhase::Started)
            .filter_map(|event| event.action().as_str().strip_prefix(PREFIX)?.parse().ok())
            .collect();
        slots
            .into_iter()
            .filter_map(|slot| self.activate(slot))
            .collect()
    }

    /// Click activation uses the same assignment as its physical shortcut. Locking the bar
    /// locks editing, not clicking. The shell suppresses clicks while its modal is open.
    #[must_use]
    pub fn activate(&self, slot: usize) -> Option<ActionBarCommand> {
        self.assignment(slot)
            .map(|assignment| assignment.command.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::ButtonState;
    use oteryn_session::ChatRoom;

    fn event(state: ButtonState, repeat: bool) -> Result<NormalizedInputEvent, InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(30)?,
            state,
            modifiers: Modifiers::NONE,
            repeat,
        })
    }

    fn assigned_bar() -> io::Result<ActionBar> {
        let mut bar = ActionBar::new(ActionBarPreferences::default(), &[82, 79, 81, 80])?;
        bar.assign(
            0,
            Some(SlotAssignment {
                label: "Help".into(),
                command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help)),
            }),
        )?;
        Ok(bar)
    }

    #[test]
    fn typing_modal_and_repeat_never_submit_gameplay_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        bar.set_input_context(true, false)?;
        assert!(bar.route(&[event(ButtonState::Pressed, false)?]).is_empty());
        bar.route(&[event(ButtonState::Released, false)?]);
        bar.set_input_context(false, true)?;
        assert!(bar.route(&[event(ButtonState::Pressed, false)?]).is_empty());
        bar.route(&[event(ButtonState::Released, false)?]);
        bar.set_input_context(false, false)?;
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        assert!(bar.route(&[event(ButtonState::Pressed, true)?]).is_empty());
        bar.route(&[event(ButtonState::Released, false)?]);
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        Ok(())
    }

    #[test]
    fn movement_and_pointer_holds_do_not_block_an_action_shortcut()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let movement = NormalizedInputEvent::Key {
            code: KeyCode::new(82)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        let mouse = NormalizedInputEvent::MouseButton {
            button: oteryn_input_actions::MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        };
        assert!(bar.route(&[movement, mouse]).is_empty());
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        Ok(())
    }

    #[test]
    fn failed_rebind_preserves_assignment_and_current_keymap()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let mut conflict = ActionBarPreferences::default();
        conflict.shortcuts[1] = conflict.shortcuts[0];
        assert!(bar.reconfigure(conflict, &[]).is_err());
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        assert_eq!(bar.preferences(), &ActionBarPreferences::default());
        let mut conflict = ActionBarPreferences::default();
        conflict.shortcuts[0] = Some(SlotShortcut {
            key: 82,
            modifiers: 0,
        });
        assert!(bar.reconfigure(conflict, &[82, 79, 81, 80]).is_err());
        Ok(())
    }

    #[test]
    fn persistence_retains_shortcuts_and_lock_but_not_session_identifiers()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let mut preferences = bar.preferences().clone();
        preferences.locked = true;
        bar.reconfigure(preferences, &[])?;
        assert_eq!(
            bar.assign(0, None).err().map(|error| error.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
        assert!(bar.activate(0).is_some());
        let json = serde_json::to_string(bar.preferences())?;
        let reopened = ActionBar::new(serde_json::from_str(&json)?, &[])?;
        assert!(reopened.preferences().locked);
        assert!(reopened.activate(0).is_none());
        assert!(!json.contains("handle") && !json.contains("spell"));
        Ok(())
    }
}
