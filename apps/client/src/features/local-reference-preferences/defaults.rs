use super::*;

pub(super) fn enabled() -> bool {
    true
}

pub(super) fn default_tooltip_delay() -> u16 {
    500
}

pub(super) fn default_arc_opacity() -> u8 {
    100
}

pub(super) fn default_console_font() -> u16 {
    14
}

pub(super) fn default_arc_distance() -> u16 {
    8
}
pub(super) fn default_arc_size() -> u16 {
    3
}

pub(super) fn default_hud_names() -> [bool; 3] {
    [true; 3]
}

pub(super) fn default_shortcuts() -> Vec<String> {
    WINDOW_ACTIONS
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect()
}

pub(super) fn default_hud_bounds() -> [u16; 4] {
    [8, 8, 900, 40]
}

pub(super) fn default_scale() -> u16 {
    100
}

pub(super) fn default_rotation_modifier() -> String {
    "CTRL".into()
}

pub(super) fn default_panels() -> Vec<String> {
    ["battle", "chat", "inventory", "minimap"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub(super) fn default_boss_opacity() -> u8 {
    100
}
pub(super) fn default_opacity() -> [u8; 3] {
    [100; 3]
}
pub(super) fn default_foreground_fps() -> u16 {
    60
}
pub(super) fn default_background_fps() -> u16 {
    15
}
