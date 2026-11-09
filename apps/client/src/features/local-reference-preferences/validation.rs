//! Shared validation, including both generations of graphics preferences.
use super::*;
impl ReferencePreferences {
    pub const MOUSE_PRESETS: [&'static str; 2] = ["Right-handed", "Left-handed"];
    pub fn set_mouse_preset(&mut self, index: usize) -> Result<(), PreferenceError> {
        if index >= Self::MOUSE_PRESETS.len() {
            return Err(PreferenceError);
        }
        self.swap_mouse_buttons = index == 1;
        Ok(())
    }
    pub(super) fn validate(&self) -> Result<(), PreferenceError> {
        self.validate_bindings()?;
        if !self.performance.valid() {
            return Err(PreferenceError);
        }
        self.validate_profiles()?;
        self.validate_status_order()?;
        if self.shortcut_order.len() > WINDOW_ACTIONS.len()
            || self.shortcut_order.iter().enumerate().any(|(i, id)| {
                !WINDOW_ACTIONS.iter().any(|(key, _)| key == id)
                    || self.shortcut_order[..i].contains(id)
            })
        {
            return Err(PreferenceError);
        }
        if self.active_battle_list >= self.battle_lists.len()
            || !self.bar_actions.valid()
            || !self.bar_layout.valid()
            || !self.action_keys.valid(self)
        {
            return Err(PreferenceError);
        }
        if self.tooltip_delay_ms > 3000
            || self.arc_opacity > 100
            || !(10..=32).contains(&self.console_font_px)
            || !crate::play::feature_0017_attack::view::validate_lists(&self.battle_lists)
            || self.arc_distance > 80
            || !(1..=8).contains(&self.arc_size)
            || !(80..=200).contains(&self.interface_scale)
            || !self.spellbook_filter.valid()
            || !self.archive_bindings.valid()
            || !self.frames.valid()
            || !self.screenshots.valid()
            || !self.arcs.valid()
            || !self.minimap.valid()
            || !shortcuts::valid(&self.shortcuts)
            || !self.controls.valid()
            || !self.skills_layout.valid()
            || !matches!(self.rotation_modifier.as_str(), "CTRL" | "ALT" | "SHIFT")
            || self.keyboard_delay_ms > 1000
            || self.hud_bounds[2] < 120
            || self.hud_bounds[3] < 24
            || self.hud_bounds.iter().any(|value| *value > 4096)
            || self.audio.master_volume > 100
            || !(80..=180).contains(&self.ui_scale)
            || self.visible_panels.len() > PANEL_IDS.len()
            || self.visible_panels.iter().enumerate().any(|(index, id)| {
                PANEL_IDS.binary_search(&id.as_str()).is_err()
                    || self.visible_panels[..index].contains(id)
            })
        {
            return Err(PreferenceError);
        }
        Ok(())
    }
}
