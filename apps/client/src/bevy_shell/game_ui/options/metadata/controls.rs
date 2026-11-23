use super::*;
use crate::play::{MousePreset, MovementKeys, RotationModifier};

pub(super) const ROWS: [Row; 8] = [
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
    check!(
        "controls.default_delay",
        "Use default keyboard delay",
        controls.default_delay
    ),
    check!(
        "controls.move_whole_stack",
        "Move stacks directly",
        controls.move_whole_stack
    ),
    Row {
        id: "controls.scheme",
        label: "Mouse control scheme",
        editor: Editor::Choice {
            values: &["Regular controls", "Classic controls", "Left smart-click"],
            read: |p| p.controls.scheme as usize,
            write: |p, i| {
                p.controls.scheme = match i {
                    1 => crate::play::MouseScheme::Classic,
                    2 => crate::play::MouseScheme::LeftSmart,
                    _ => crate::play::MouseScheme::Regular,
                }
            },
        },
    },
    Row {
        id: "controls.turn_towards_move",
        label: "Always face the direction of movement",
        editor: Editor::Check {
            read: |p| !p.controls.preserve_facing,
            write: |p, v| p.controls.preserve_facing = !v,
        },
    },
];
