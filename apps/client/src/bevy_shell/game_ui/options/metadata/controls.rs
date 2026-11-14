use super::*;
use crate::play::{MousePreset, MovementKeys, RotationModifier};

pub(super) const ROWS: [Row; 4] = [
    Row {
        id: "controls.mouse",
        label: "Mouse Preset",
        editor: Editor::Choice {
            values: &["Right-handed", "Left-handed"],
            read: |p| {
                usize::from((p.controls.mouse == MousePreset::LeftHanded) ^ p.swap_mouse_buttons)
            },
            write: |p, v| {
                p.controls.mouse = if v == 1 {
                    MousePreset::LeftHanded
                } else {
                    MousePreset::Standard
                };
                p.swap_mouse_buttons = false;
            },
        },
    },
    Row {
        id: "controls.movement",
        label: "Movement Keys",
        editor: Editor::Choice {
            values: &["Arrow keys", "Arrow keys and WASD"],
            read: |p| usize::from(p.controls.movement == MovementKeys::ArrowsAndWasd),
            write: |p, v| {
                p.controls.movement = if v == 1 {
                    MovementKeys::ArrowsAndWasd
                } else {
                    MovementKeys::Arrows
                }
            },
        },
    },
    Row {
        id: "controls.rotation",
        label: "Rotation Modifier",
        editor: Editor::Choice {
            values: &["Ctrl", "Shift", "Alt"],
            read: |p| match p.controls.rotation {
                RotationModifier::Control => 0,
                RotationModifier::Shift => 1,
                RotationModifier::Alt => 2,
            },
            write: |p, v| {
                p.controls.rotation = match v {
                    1 => RotationModifier::Shift,
                    2 => RotationModifier::Alt,
                    _ => RotationModifier::Control,
                };
                p.rotation_modifier = match v {
                    1 => "SHIFT",
                    2 => "ALT",
                    _ => "CTRL",
                }
                .into();
            },
        },
    },
    Row {
        id: "controls.keyboard_delay",
        label: "Keyboard Delay (ms)",
        editor: Editor::Number {
            min: 0,
            max: 1000,
            read: |p| p.controls.keyboard_delay_ms.max(p.keyboard_delay_ms),
            write: |p, v| {
                p.controls.keyboard_delay_ms = v;
                p.keyboard_delay_ms = v;
            },
        },
    },
];
