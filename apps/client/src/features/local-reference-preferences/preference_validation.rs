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

    pub(in super::super) fn validate(&self) -> Result<(), PreferenceError> {
        if !self.graphics.valid() {
            return Err(PreferenceError);
        }
        if self
            .language_code
            .as_deref()
            .is_some_and(|code| oteryn_client_ui::localization::Language::from_code(code).is_none())
        {
            return Err(PreferenceError);
        }
        if !(1..=1000).contains(&self.console.line_limit) || !self.action_bar.valid() {
            return Err(PreferenceError);
        }
        if self.screenshots.backlog_frames > 10 {
            return Err(PreferenceError);
        }
        if self.active_battle_list >= self.battle_lists.len() || !self.bar_actions.valid() || !self.bar_layout.valid() || !self.action_keys.valid(self) { return Err(PreferenceError); }
        self.validate_bindings()?;
        if self.boss_area_opacity > 100 {
            return Err(PreferenceError);
        }
        if self.audio_device.len() > 256 || self.audio_device.chars().any(char::is_control) {
            return Err(PreferenceError);
        }
        if self.effect_opacity.iter().any(|opacity| *opacity > 100) {
            return Err(PreferenceError);
        }
        if self.ambient_brightness > 100 {
            return Err(PreferenceError);
        }
        if !(1..=500).contains(&self.foreground_fps) || !(1..=500).contains(&self.background_fps) {
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
            || self.audio.battle_volumes.iter().any(|v| *v > 100)
            || self.audio.channel_volumes.iter().any(|v| *v > 100)
            || self.audio.ambient_volume > 100
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
