//! Mouse and movement preferences translate into existing authoritative gameplay inputs.
use oteryn_input_actions::{
    ButtonState, KeyCode, Modifier, Modifiers, MouseButton, NormalizedInputEvent,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MousePreset {
    #[default]
    Standard,
    LeftHanded,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementKeys {
    #[default]
    Arrows,
    ArrowsAndWasd,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationModifier {
    #[default]
    Control,
    Shift,
    Alt,
}
impl RotationModifier {
    fn modifier(self) -> Modifier {
        match self {
            Self::Control => Modifier::Control,
            Self::Shift => Modifier::Shift,
            Self::Alt => Modifier::Alt,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControlPreferences {
    pub mouse: MousePreset,
    pub movement: MovementKeys,
    pub rotation: RotationModifier,
    /// Minimum interval between operating-system repeat presses. Initial presses stay immediate.
    pub keyboard_delay_ms: u16,
}
impl ControlPreferences {
    pub fn drag_button(&self, world_surface: bool) -> MouseButton {
        if world_surface && self.mouse == MousePreset::LeftHanded {
            MouseButton::SECONDARY
        } else {
            MouseButton::PRIMARY
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.keyboard_delay_ms <= 1000
    }
}
#[derive(Default)]
pub(super) struct ControlInput {
    held: BTreeMap<KeyCode, (KeyCode, Modifiers, Duration)>,
    mouse: BTreeMap<MouseButton, MouseButton>,
}
impl ControlInput {
    pub(super) fn route(
        &mut self,
        prefs: &ControlPreferences,
        events: &[NormalizedInputEvent],
        chat: bool,
        now: Duration,
    ) -> Vec<NormalizedInputEvent> {
        let mut routed = Vec::with_capacity(events.len());
        for event in events {
            match event {
                NormalizedInputEvent::Key {
                    code,
                    modifiers,
                    state,
                    repeat,
                } => {
                    let (mapped, mods, last) = self.held.get(code).copied().unwrap_or_else(|| {
                        let rotation = Modifiers::one(prefs.rotation.modifier());
                        let movement_modifiers =
                            *modifiers == Modifiers::NONE || *modifiers == rotation;
                        let mapped = if !chat
                            && prefs.movement == MovementKeys::ArrowsAndWasd
                            && movement_modifiers
                        {
                            match code.get() {
                                26 => KeyCode::ARROW_UP,
                                4 => KeyCode::ARROW_LEFT,
                                22 => KeyCode::ARROW_DOWN,
                                7 => KeyCode::ARROW_RIGHT,
                                _ => *code,
                            }
                        } else {
                            *code
                        };
                        let arrow = [
                            KeyCode::ARROW_UP,
                            KeyCode::ARROW_RIGHT,
                            KeyCode::ARROW_DOWN,
                            KeyCode::ARROW_LEFT,
                        ]
                        .contains(&mapped);
                        let mods = if arrow && *modifiers == rotation {
                            Modifiers::one(Modifier::Control)
                        } else if arrow
                            && prefs.rotation != RotationModifier::Control
                            && *modifiers == Modifiers::one(Modifier::Control)
                        {
                            Modifiers::NONE
                        } else {
                            *modifiers
                        };
                        (mapped, mods, now)
                    });
                    if *state == ButtonState::Released {
                        self.held.remove(code);
                    } else {
                        if *repeat
                            && now.saturating_sub(last)
                                < Duration::from_millis(u64::from(prefs.keyboard_delay_ms))
                        {
                            continue;
                        }
                        self.held.insert(*code, (mapped, mods, now));
                    }
                    routed.push(NormalizedInputEvent::Key {
                        code: mapped,
                        modifiers: mods,
                        state: *state,
                        repeat: *repeat,
                    });
                }
                NormalizedInputEvent::MouseButton { button, state, .. } => {
                    let mapped = self.mouse.get(button).copied().unwrap_or_else(|| {
                        if prefs.mouse == MousePreset::LeftHanded {
                            swap_button(*button)
                        } else {
                            *button
                        }
                    });
                    if *state == ButtonState::Released {
                        self.mouse.remove(button);
                    } else {
                        self.mouse.insert(*button, mapped);
                    }
                    let mut event = event.clone();
                    if let NormalizedInputEvent::MouseButton { button, .. } = &mut event {
                        *button = mapped;
                    }
                    routed.push(event);
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => {
                    self.held.clear();
                    self.mouse.clear();
                    routed.push(event.clone());
                }
                _ => routed.push(event.clone()),
            }
        }
        routed
    }
}

impl super::ReferencePreferences {
    pub(super) fn reconcile_controls(&mut self, previous: Option<&Self>) {
        let legacy_mouse_changed =
            previous.map_or(self.controls.mouse == MousePreset::Standard, |old| {
                self.controls.mouse == old.controls.mouse
                    && self.swap_mouse_buttons
                    && self.swap_mouse_buttons != old.swap_mouse_buttons
            });
        if legacy_mouse_changed {
            self.controls.mouse = if self.swap_mouse_buttons {
                MousePreset::LeftHanded
            } else {
                MousePreset::Standard
            };
        }
        self.swap_mouse_buttons = false;
        // Default legacy aliases in an older editor snapshot cannot erase an explicit control choice.
        let legacy_rotation_changed =
            previous.map_or(self.controls.rotation == RotationModifier::Control, |old| {
                self.controls.rotation == old.controls.rotation
                    && self.rotation_modifier != "CTRL"
                    && self.rotation_modifier != old.rotation_modifier
            });
        if legacy_rotation_changed {
            self.controls.rotation = match self.rotation_modifier.as_str() {
                "ALT" => RotationModifier::Alt,
                "SHIFT" => RotationModifier::Shift,
                _ => RotationModifier::Control,
            };
        }
        self.rotation_modifier = match self.controls.rotation {
            RotationModifier::Alt => "ALT",
            RotationModifier::Shift => "SHIFT",
            RotationModifier::Control => "CTRL",
        }
        .into();
    }
}

pub(super) fn swap_button(button: MouseButton) -> MouseButton {
    match button {
        MouseButton::PRIMARY => MouseButton::SECONDARY,
        MouseButton::SECONDARY => MouseButton::PRIMARY,
        other => other,
    }
}
