//! Configurable action slots over existing session commands.
//!
//! Only layout and physical shortcuts persist. Spell-book indices and item handles belong to
//! one admitted session and are deliberately cleared on reconnect. A future generation-pinned
//! catalogue can supply durable semantic assignments without persisting these transient IDs.
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ButtonState, ContextDefinition, ContextId,
    ContextKind, InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifiers,
    NormalizedInputEvent, RepeatPolicy,
};
use oteryn_session::{
    CastOutcome, ChatIntent, ChatOutcome, ItemHandle, ItemMoveDestination, ItemMoveIntent,
    ItemMoveOutcomeResult, Session, SessionError, SessionStream, SpellTarget, UseOutcome,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io, num::NonZeroU32};

/// The reference client exposes fifty independently bindable positions per row.
pub const ACTION_BAR_SLOTS: usize = 50;
pub const ACTION_BAR_ROWS: usize = 9;
pub const TOTAL_ACTION_BAR_SLOTS: usize = ACTION_BAR_ROWS * ACTION_BAR_SLOTS;
pub const ACTION_ROW_HEIGHT: f32 = 38.0;
pub const ACTION_COLUMN_WIDTH: f32 = 30.0;
pub const BOTTOM_ROWS: [usize; 3] = [0, 1, 2];
pub const LEFT_ROWS: [usize; 3] = [3, 4, 5];
pub const RIGHT_ROWS: [usize; 3] = [6, 7, 8];
const PREFIX: &str = "client.action-slot.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionBarEdge {
    Bottom,
    Left,
    Right,
}

#[must_use]
pub const fn row_position(row: usize) -> Option<(ActionBarEdge, usize)> {
    match row {
        0..=2 => Some((ActionBarEdge::Bottom, row)),
        3..=5 => Some((ActionBarEdge::Left, row - 3)),
        6..=8 => Some((ActionBarEdge::Right, row - 6)),
        _ => None,
    }
}

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
    #[serde(
        default = "empty_shortcuts",
        deserialize_with = "deserialize_shortcuts",
        serialize_with = "serialize_shortcuts"
    )]
    pub shortcuts: [Option<SlotShortcut>; ACTION_BAR_SLOTS],
    #[serde(
        default = "empty_shortcuts",
        deserialize_with = "deserialize_shortcuts",
        serialize_with = "serialize_shortcuts"
    )]
    pub secondary_shortcuts: [Option<SlotShortcut>; ACTION_BAR_SLOTS],
    /// Three bottom, three left, three right rows; the legacy fields remain bottom row 1.
    #[serde(default)]
    pub extra_rows: [ActionRowPreferences; ACTION_BAR_ROWS - 1],
    /// Independent bottom/left/right visibility masters; raw row selections and hotkeys
    /// remain intact. Legacy files retain their existing layout through enabled masters.
    #[serde(default = "default_edge_enabled")]
    pub edge_enabled: [bool; 3],
}

const fn default_edge_enabled() -> [bool; 3] {
    [true; 3]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRowPreferences {
    pub visible: bool,
    pub locked: bool,
    #[serde(
        default = "empty_shortcuts",
        deserialize_with = "deserialize_shortcuts",
        serialize_with = "serialize_shortcuts"
    )]
    pub shortcuts: [Option<SlotShortcut>; ACTION_BAR_SLOTS],
    #[serde(
        default = "empty_shortcuts",
        deserialize_with = "deserialize_shortcuts",
        serialize_with = "serialize_shortcuts"
    )]
    pub secondary_shortcuts: [Option<SlotShortcut>; ACTION_BAR_SLOTS],
}

const fn empty_shortcuts() -> [Option<SlotShortcut>; ACTION_BAR_SLOTS] {
    [None; ACTION_BAR_SLOTS]
}

impl Default for ActionRowPreferences {
    fn default() -> Self {
        Self {
            visible: false,
            locked: false,
            shortcuts: empty_shortcuts(),
            secondary_shortcuts: empty_shortcuts(),
        }
    }
}

fn serialize_shortcuts<S>(
    shortcuts: &[Option<SlotShortcut>; ACTION_BAR_SLOTS],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    shortcuts.as_slice().serialize(serializer)
}

fn deserialize_shortcuts<'de, D>(
    deserializer: D,
) -> Result<[Option<SlotShortcut>; ACTION_BAR_SLOTS], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let saved = Vec::<Option<SlotShortcut>>::deserialize(deserializer)?;
    if saved.len() > ACTION_BAR_SLOTS {
        return Err(serde::de::Error::custom("too many action-bar shortcuts"));
    }
    let mut shortcuts = empty_shortcuts();
    for (index, shortcut) in saved.into_iter().enumerate() {
        shortcuts[index] = shortcut;
    }
    Ok(shortcuts)
}

impl Default for ActionBarPreferences {
    fn default() -> Self {
        let keys = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        let mut shortcuts = [None; ACTION_BAR_SLOTS];
        for (slot, key) in keys.into_iter().enumerate() {
            shortcuts[slot] = Some(SlotShortcut { key, modifiers: 0 });
        }
        Self {
            visible: true,
            locked: false,
            shortcuts,
            secondary_shortcuts: empty_shortcuts(),
            extra_rows: [ActionRowPreferences::default(); ACTION_BAR_ROWS - 1],
            edge_enabled: default_edge_enabled(),
        }
    }
}

impl ActionBarPreferences {
    /// Effective rendering visibility, separate from the retained raw row selection.
    #[must_use]
    pub fn row_is_visible(&self, row: usize) -> bool {
        self.edge_enabled
            .get(row / 3)
            .is_some_and(|enabled| *enabled)
            && self.row(row).is_some_and(|preferences| preferences.visible)
    }

    #[must_use]
    pub fn visible_rows(&self) -> [u8; 3] {
        let mut counts = [0; 3];
        for row in 0..ACTION_BAR_ROWS {
            if self.row_is_visible(row) {
                counts[row / 3] += 1;
            }
        }
        counts
    }

    #[must_use]
    pub fn row(&self, row: usize) -> Option<ActionRowPreferences> {
        if row == 0 {
            Some(ActionRowPreferences {
                visible: self.visible,
                locked: self.locked,
                shortcuts: self.shortcuts,
                secondary_shortcuts: self.secondary_shortcuts,
            })
        } else {
            self.extra_rows.get(row - 1).copied()
        }
    }

    pub fn set_row(&mut self, row: usize, preferences: ActionRowPreferences) -> io::Result<()> {
        if row == 0 {
            self.visible = preferences.visible;
            self.locked = preferences.locked;
            self.shortcuts = preferences.shortcuts;
            self.secondary_shortcuts = preferences.secondary_shortcuts;
        } else {
            *self.extra_rows.get_mut(row - 1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Unknown action row")
            })? = preferences;
        }
        Ok(())
    }

    pub fn all_shortcuts(&self) -> impl Iterator<Item = &Option<SlotShortcut>> {
        self.shortcuts
            .iter()
            .chain(self.secondary_shortcuts.iter())
            .chain(
                self.extra_rows
                    .iter()
                    .flat_map(|row| row.shortcuts.iter().chain(row.secondary_shortcuts.iter())),
            )
    }

    /// Reserve application Escape/Enter/F10 and the configured movement chords. Reject
    /// conflicts before replacing any live bindings or saving preferences.
    pub fn validate(&self, movement_keys: &[u16]) -> io::Result<()> {
        let mut seen = Vec::new();
        for shortcut in self.all_shortcuts() {
            let Some(shortcut) = shortcut else { continue };
            if shortcut.chord().is_err()
                || matches!(shortcut.key, 40 | 41 | 67)
                || (shortcut.modifiers == 0 && movement_keys.contains(&shortcut.key))
                || seen.contains(shortcut)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or conflicting action shortcut",
                ));
            }
            seen.push(*shortcut);
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
    assignments: [Option<SlotAssignment>; TOTAL_ACTION_BAR_SLOTS],
    router: InputRouter,
    text: ContextId,
    modal: ContextId,
    held: BTreeSet<KeyCode>,
    quarantined: BTreeSet<KeyCode>,
    text_active: bool,
    modal_active: bool,
    focused: bool,
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
        for row in 0..ACTION_BAR_ROWS {
            if let Some(row_preferences) = preferences.row(row) {
                for (slot, shortcut) in row_preferences
                    .shortcuts
                    .iter()
                    .zip(row_preferences.secondary_shortcuts.iter())
                    .enumerate()
                    .flat_map(|(slot, pair)| [(slot, pair.0), (slot, pair.1)])
                {
                    if let Some(shortcut) = shortcut {
                        bindings.push(Binding::new(
                            gameplay.clone(),
                            shortcut.chord()?,
                            ActionId::new(format!("{PREFIX}{}", row * ACTION_BAR_SLOTS + slot))?,
                            RepeatPolicy::Ignore,
                        ));
                    }
                }
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
            held: BTreeSet::new(),
            quarantined: BTreeSet::new(),
            text_active: false,
            modal_active: false,
            focused: true,
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
        let row = self
            .preferences
            .row(slot / ACTION_BAR_SLOTS)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Unknown action slot"))?;
        if row.locked {
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

    pub fn clear_row(&mut self, row: usize) -> io::Result<()> {
        let preferences = self
            .preferences
            .row(row)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Unknown action row"))?;
        if preferences.locked {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Action row is locked",
            ));
        }
        self.assignments[row * ACTION_BAR_SLOTS..(row + 1) * ACTION_BAR_SLOTS].fill(None);
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
        replacement.held = self.held.clone();
        replacement.quarantined = self.held.union(&self.quarantined).copied().collect();
        replacement
            .set_input_context(self.text_active, self.modal_active)
            .map_err(io::Error::other)?;
        replacement.focused = self.focused;
        replacement
            .router
            .process(&NormalizedInputEvent::FocusChanged {
                focused: self.focused,
            });
        *self = replacement;
        Ok(())
    }

    /// Admission changes clear every session handle and spell index. Held keys need release.
    pub fn reset_session(&mut self) -> io::Result<()> {
        self.reconfigure(self.preferences.clone(), &[])?;
        self.assignments.fill(None);
        Ok(())
    }

    pub fn set_input_context(
        &mut self,
        text_active: bool,
        modal_active: bool,
    ) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, text_active)?;
        self.router.set_context_active(&self.modal, modal_active)?;
        self.text_active = text_active;
        self.modal_active = modal_active;
        Ok(())
    }

    /// Process focus/release events even while text/modal input suppresses gameplay.
    /// Physical held state suppresses repeats. Slots are independent one-shot actions:
    /// each accepted press/release pulse stays inside this private semantic router and
    /// does not change the physical state or the movement/platform input stream.
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<ActionBarCommand> {
        let mut slots = Vec::new();
        for event in events {
            match event {
                NormalizedInputEvent::Key {
                    code,
                    state,
                    repeat,
                    ..
                } => {
                    if *state == ButtonState::Released {
                        self.held.remove(code);
                        self.quarantined.remove(code);
                    } else {
                        if !self.focused {
                            continue;
                        }
                        let fresh = self.held.insert(*code);
                        if !fresh || *repeat || self.quarantined.contains(code) {
                            continue;
                        }
                    }
                    if !matches!(code.get(), 224..=231)
                        && !self
                            .preferences
                            .all_shortcuts()
                            .flatten()
                            .any(|s| s.key == code.get())
                    {
                        continue;
                    }
                }
                NormalizedInputEvent::MouseButton { .. } | NormalizedInputEvent::Wheel { .. } => {
                    continue;
                }
                NormalizedInputEvent::FocusChanged { focused } => {
                    self.focused = *focused;
                    if !focused {
                        self.held.clear();
                        self.quarantined.clear();
                    }
                }
                NormalizedInputEvent::DeviceLost => {
                    self.held.clear();
                    self.quarantined.clear();
                }
                _ => {}
            }
            slots.extend(
                self.router
                    .process(event)
                    .iter()
                    .filter(|event| event.phase() == ActionPhase::Started)
                    .filter_map(|event| {
                        event
                            .action()
                            .as_str()
                            .strip_prefix(PREFIX)?
                            .parse::<usize>()
                            .ok()
                    }),
            );
            if let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers,
                ..
            } = event
            {
                self.router.process(&NormalizedInputEvent::Key {
                    code: *code,
                    state: ButtonState::Released,
                    modifiers: *modifiers,
                    repeat: false,
                });
            }
        }
        slots
            .into_iter()
            .filter_map(|slot| self.activate(slot))
            .collect()
    }

    /// Hidden rows keep shortcuts active, matching the legacy visibility preference.
    /// Click activation uses the same assignment as its physical shortcut. Locking the row
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
        key_event(30, 0, state, repeat)
    }

    fn key_event(
        key: u16,
        modifiers: u8,
        state: ButtonState,
        repeat: bool,
    ) -> Result<NormalizedInputEvent, InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(key)?,
            state,
            modifiers: Modifiers::from_bits(modifiers)?,
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
    fn independent_slots_fire_while_another_action_key_remains_held()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let world = ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::World));
        bar.assign(
            1,
            Some(SlotAssignment {
                label: "World".into(),
                command: world.clone(),
            }),
        )?;
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        assert_eq!(
            bar.route(&[key_event(31, 0, ButtonState::Pressed, false)?]),
            vec![world]
        );
        assert!(
            bar.route(&[
                event(ButtonState::Pressed, false)?,
                key_event(31, 0, ButtonState::Pressed, false)?,
                event(ButtonState::Pressed, true)?,
                key_event(31, 0, ButtonState::Pressed, true)?,
            ])
            .is_empty()
        );
        bar.route(&[event(ButtonState::Released, false)?]);
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        assert!(
            bar.route(&[key_event(31, 0, ButtonState::Pressed, false)?])
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn independent_modified_slots_keep_physical_holds_across_context_and_focus_changes()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let mut preferences = bar.preferences().clone();
        preferences.shortcuts[1] = Some(SlotShortcut {
            key: 31,
            modifiers: 1,
        });
        bar.reconfigure(preferences, &[])?;
        bar.assign(1, bar.assignment(0).cloned())?;
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        assert_eq!(
            bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                .len(),
            1
        );
        assert!(
            bar.route(&[key_event(30, 1, ButtonState::Pressed, false)?])
                .is_empty()
        );
        bar.route(&[key_event(31, 1, ButtonState::Released, false)?]);
        for (text, modal) in [(true, false), (false, true)] {
            bar.set_input_context(text, modal)?;
            assert!(
                bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                    .is_empty()
            );
            bar.set_input_context(false, false)?;
            assert!(
                bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                    .is_empty()
            );
            bar.route(&[key_event(31, 1, ButtonState::Released, false)?]);
            assert_eq!(
                bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                    .len(),
                1
            );
            bar.route(&[key_event(31, 1, ButtonState::Released, false)?]);
        }
        bar.route(&[NormalizedInputEvent::FocusChanged { focused: false }]);
        assert!(
            bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                .is_empty()
        );
        bar.route(&[NormalizedInputEvent::FocusChanged { focused: true }]);
        assert!(
            bar.route(&[key_event(31, 1, ButtonState::Pressed, true)?])
                .is_empty()
        );
        assert!(
            bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                .is_empty()
        );
        bar.route(&[key_event(31, 1, ButtonState::Released, false)?]);
        assert_eq!(
            bar.route(&[key_event(31, 1, ButtonState::Pressed, false)?])
                .len(),
            1
        );
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

    #[test]
    fn secondary_shortcut_activates_the_same_action_slot() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut preferences = ActionBarPreferences::default();
        preferences.shortcuts[0] = None;
        preferences.secondary_shortcuts[0] = Some(SlotShortcut {
            key: 30,
            modifiers: 0,
        });
        let mut bar = ActionBar::new(preferences, &[])?;
        bar.assign(
            0,
            Some(SlotAssignment {
                label: "Help".into(),
                command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help)),
            }),
        )?;
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        Ok(())
    }

    #[test]
    fn legacy_json_keeps_primary_defaults_and_bounds_extra_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut json = serde_json::to_value(ActionBarPreferences::default())?;
        let object = json.as_object_mut().ok_or("Expected preferences object")?;
        object.remove("extra_rows");
        object.remove("edge_enabled");
        let old: ActionBarPreferences = serde_json::from_value(json)?;
        assert_eq!(old, ActionBarPreferences::default());
        assert_eq!(old.visible_rows(), [1, 0, 0]);
        assert_eq!(old.edge_enabled, [true; 3]);
        assert!(
            old.extra_rows
                .iter()
                .all(|r| !r.visible && r.shortcuts.iter().all(Option::is_none))
        );
        let mut json = serde_json::to_value(old)?;
        json["extra_rows"] = serde_json::json!([]);
        assert!(serde_json::from_value::<ActionBarPreferences>(json).is_err());
        Ok(())
    }

    #[test]
    fn edge_masters_retain_noncontiguous_rows_chords_locks_and_session_assignments()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let assignment = bar.assignment(0).cloned();
        bar.assign(4 * ACTION_BAR_SLOTS, assignment.clone())?;
        let mut preferences = bar.preferences().clone();
        preferences.extra_rows[1].visible = true;
        preferences.extra_rows[3].visible = true;
        preferences.extra_rows[3].locked = true;
        preferences.extra_rows[3].shortcuts[0] = Some(SlotShortcut {
            key: 4,
            modifiers: 2,
        });
        preferences.extra_rows[7].visible = true;
        let rows: Vec<_> = (0..ACTION_BAR_ROWS)
            .map(|row| preferences.row(row))
            .collect();
        for edge in 0..3 {
            let mut hidden = preferences.clone();
            hidden.edge_enabled[edge] = false;
            bar.reconfigure(hidden, &[])?;
            let mut expected = [2, 1, 1];
            expected[edge] = 0;
            assert_eq!(bar.preferences().visible_rows(), expected);
            assert_eq!(
                (0..ACTION_BAR_ROWS)
                    .map(|row| bar.preferences().row(row))
                    .collect::<Vec<_>>(),
                rows
            );
            assert!((edge * 3..edge * 3 + 3).all(|row| !bar.preferences().row_is_visible(row)));
            assert_eq!(bar.assignment(4 * ACTION_BAR_SLOTS), assignment.as_ref());
        }
        let mut hidden = preferences.clone();
        hidden.edge_enabled = [false; 3];
        bar.reconfigure(hidden, &[])?;
        assert_eq!(bar.preferences().visible_rows(), [0; 3]);
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        bar.route(&[event(ButtonState::Released, false)?]);
        bar.reconfigure(preferences.clone(), &[])?;
        assert_eq!(bar.preferences().visible_rows(), [2, 1, 1]);
        assert_eq!(bar.preferences(), &preferences);
        assert_eq!(bar.assignment(4 * ACTION_BAR_SLOTS), assignment.as_ref());
        assert!(!preferences.row_is_visible(ACTION_BAR_ROWS));
        assert!(!preferences.row_is_visible(usize::MAX));
        assert_eq!(
            serde_json::from_slice::<ActionBarPreferences>(&serde_json::to_vec(&preferences)?)?,
            preferences
        );
        for invalid in [
            serde_json::json!([true, false]),
            serde_json::json!([true, true, true, true]),
            serde_json::json!(null),
            serde_json::json!([true, 1, false]),
        ] {
            let mut json = serde_json::to_value(&preferences)?;
            json["edge_enabled"] = invalid;
            assert!(serde_json::from_value::<ActionBarPreferences>(json).is_err());
        }
        Ok(())
    }

    #[test]
    fn shortcuts_conflict_across_hidden_rows() -> io::Result<()> {
        let mut preferences = ActionBarPreferences::default();
        preferences.extra_rows[7].shortcuts[11] = preferences.shortcuts[0];
        assert!(preferences.validate(&[]).is_err());
        for invalid in [
            SlotShortcut {
                key: 82,
                modifiers: 0,
            },
            SlotShortcut {
                key: 67,
                modifiers: 2,
            },
            SlotShortcut {
                key: 30,
                modifiers: 16,
            },
            SlotShortcut {
                key: 0,
                modifiers: 0,
            },
        ] {
            preferences.extra_rows[7].shortcuts[11] = Some(invalid);
            assert!(preferences.validate(&[82]).is_err());
        }
        preferences.extra_rows[7].shortcuts[11] = Some(SlotShortcut {
            key: 82,
            modifiers: 2,
        });
        preferences.validate(&[82])?;
        preferences.extra_rows[0].shortcuts[0] = preferences.extra_rows[7].shortcuts[11];
        assert!(preferences.validate(&[82]).is_err());
        Ok(())
    }

    #[test]
    fn twelve_slot_preferences_migrate_to_fifty_without_losing_bindings()
    -> Result<(), Box<dyn std::error::Error>> {
        let shortcut = serde_json::json!({"key": 30, "modifiers": 0});
        let mut primary = vec![serde_json::Value::Null; 12];
        primary[0] = shortcut.clone();
        let rows = (0..ACTION_BAR_ROWS - 1)
            .map(|_| {
                serde_json::json!({
                    "visible": false,
                    "locked": false,
                    "shortcuts": vec![serde_json::Value::Null; 12]
                })
            })
            .collect::<Vec<_>>();
        let restored: ActionBarPreferences = serde_json::from_value(serde_json::json!({
            "visible": true,
            "locked": false,
            "shortcuts": primary,
            "extra_rows": rows,
            "edge_enabled": [true, true, true]
        }))?;
        assert_eq!(
            restored.shortcuts[0],
            Some(SlotShortcut {
                key: 30,
                modifiers: 0
            })
        );
        assert!(restored.shortcuts[12..].iter().all(Option::is_none));
        assert!(restored.secondary_shortcuts.iter().all(Option::is_none));
        assert!(restored.extra_rows.iter().all(|row| {
            row.shortcuts.iter().all(Option::is_none)
                && row.secondary_shortcuts.iter().all(Option::is_none)
        }));
        Ok(())
    }

    #[test]
    fn rows_lock_independently_and_session_reset_clears_all_assignments() -> io::Result<()> {
        let mut bar = assigned_bar()?;
        let assignment = bar.assignment(0).cloned();
        bar.assign(TOTAL_ACTION_BAR_SLOTS - 1, assignment.clone())?;
        let mut preferences = bar.preferences().clone();
        preferences.visible = false;
        preferences.extra_rows[7].visible = true;
        preferences.extra_rows[7].locked = true;
        bar.reconfigure(preferences, &[])?;
        assert_eq!(bar.preferences().visible_rows(), [0, 0, 1]);
        assert!(bar.activate(0).is_some()); // Hidden means rendering only.
        assert!(bar.activate(TOTAL_ACTION_BAR_SLOTS - 1).is_some());
        assert!(bar.assign(TOTAL_ACTION_BAR_SLOTS - 1, None).is_err());
        assert!(bar.clear_row(ACTION_BAR_ROWS - 1).is_err());
        bar.assign(0, assignment)?;
        bar.clear_row(0)?;
        assert!(bar.assignment(0).is_none());
        assert!(bar.assign(TOTAL_ACTION_BAR_SLOTS, None).is_err());
        bar.reset_session()?;
        assert!((0..TOTAL_ACTION_BAR_SLOTS).all(|slot| bar.assignment(slot).is_none()));
        assert!(bar.preferences().extra_rows[7].locked);
        Ok(())
    }

    #[test]
    fn rebind_quarantines_bound_and_unbound_held_keys_and_preserves_context()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = assigned_bar()?;
        let assignment = bar.assignment(0).cloned();
        bar.assign(TOTAL_ACTION_BAR_SLOTS - 1, assignment)?;
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        let mut preferences = bar.preferences().clone();
        preferences.shortcuts[0] = None;
        preferences.extra_rows[7].shortcuts[ACTION_BAR_SLOTS - 1] = Some(SlotShortcut {
            key: 30,
            modifiers: 0,
        });
        bar.reconfigure(preferences, &[])?;
        assert!(
            bar.route(&[
                event(ButtonState::Pressed, false)?,
                event(ButtonState::Pressed, true)?
            ])
            .is_empty()
        );
        bar.route(&[event(ButtonState::Released, false)?]);
        assert_eq!(bar.route(&[event(ButtonState::Pressed, false)?]).len(), 1);
        bar.route(&[event(ButtonState::Released, false)?]);
        let key = |state| -> Result<_, InputError> {
            Ok(NormalizedInputEvent::Key {
                code: KeyCode::new(4)?,
                state,
                modifiers: Modifiers::NONE,
                repeat: false,
            })
        };
        bar.route(&[key(ButtonState::Pressed)?]); // Unbound key still has physical held state.
        let mut preferences = bar.preferences().clone();
        preferences.extra_rows[7].shortcuts[ACTION_BAR_SLOTS - 1] = Some(SlotShortcut {
            key: 4,
            modifiers: 0,
        });
        bar.set_input_context(true, true)?;
        bar.reconfigure(preferences, &[])?;
        bar.set_input_context(false, false)?;
        assert!(bar.route(&[key(ButtonState::Pressed)?]).is_empty());
        bar.route(&[key(ButtonState::Released)?]);
        bar.set_input_context(true, true)?;
        bar.reconfigure(bar.preferences().clone(), &[])?;
        assert!(bar.route(&[key(ButtonState::Pressed)?]).is_empty());
        bar.route(&[key(ButtonState::Released)?]);
        bar.set_input_context(false, false)?;
        assert_eq!(bar.route(&[key(ButtonState::Pressed)?]).len(), 1);
        let assignment = bar.assignment(TOTAL_ACTION_BAR_SLOTS - 1).cloned();
        bar.reset_session()?;
        bar.assign(TOTAL_ACTION_BAR_SLOTS - 1, assignment)?;
        assert!(bar.route(&[key(ButtonState::Pressed)?]).is_empty());
        bar.route(&[key(ButtonState::Released)?]);
        assert_eq!(bar.route(&[key(ButtonState::Pressed)?]).len(), 1);
        Ok(())
    }
}
