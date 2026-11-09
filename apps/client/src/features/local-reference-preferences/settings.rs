//! Local-only preferences whose keys come from the audited client reference surface.
//!
//! These values neither select a protocol capability nor manufacture server-owned panel data.
#[path = "game_window.rs"]
mod game_window;
pub use game_window::GameWindowPreferences;
use crate::map::World;
use crate::play::PlayView;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
#[path = "arcs.rs"]
mod arcs;
pub use arcs::{ArcLayout, ArcPreferences};
#[path = "basic.rs"]
mod basic;
pub use basic::{BASIC_GROUPS, BasicOptions};
#[path = "bindings.rs"]
mod bindings;
#[path = "control_input.rs"]
mod controls;
#[path = "frames.rs"]
mod frames;
pub use frames::{FramePacer, FramePreferences};
#[path = "help.rs"]
mod help;
#[path = "profile_contexts.rs"]
mod profile_contexts;
#[path = "profiles.rs"]
mod profiles;
#[path = "status_order.rs"]
mod status_order;
#[path = "shortcuts.rs"]
mod shortcuts;
#[path = "store.rs"]
mod store;
#[path = "sync.rs"]
mod sync;
#[path = "tooltips.rs"]
mod tooltips;
#[path = "ui_geometry.rs"]
mod ui_geometry;
pub use ui_geometry::{FIXED_PIXEL_FONT_PROPERTY, fit_ui_scale, rescale_ui_rect, scale_ui_rect};
pub use tooltips::{HoverTooltip, TooltipTarget};
pub use bindings::{PreferenceBinding, PreferenceInput, preference_bindings};
pub use controls::{ControlPreferences, MousePreset, MovementKeys, RotationModifier};
pub use help::{HelpTopic, help_text};
pub use profile_contexts::HotkeyProfiles;
pub use shortcuts::SHORTCUTS;
pub use store::PreferenceStore;
pub use sync::{SettingsPage, SettingsState, WINDOW_ACTIONS};
const PANEL_IDS: &[&str] = &[
    "battle",
    "chat",
    "guild",
    "hireling",
    "imbuement",
    "inventory",
    "market",
    "minimap",
    "npc-shop",
    "party",
    "prey",
    "quest-log",
    "stash",
    "task-board",
    "trade",
    "vip",
    "wheel",
];
#[path = "defaults.rs"]
mod defaults;
use defaults::*;
/// Preferences owned entirely by the native client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreferences {
    #[serde(default)]
    pub game_window: GameWindowPreferences,
    #[serde(default = "enabled")]
    pub tooltips_enabled: bool,
    #[serde(default = "default_tooltip_delay")]
    pub tooltip_delay_ms: u16,
    #[serde(default = "enabled")]
    pub hud_health: bool,
    #[serde(default = "enabled")]
    pub hud_mana: bool,
    #[serde(default = "enabled")]
    pub hud_marks_own: bool,
    #[serde(default = "enabled")]
    pub hud_marks_other: bool,
    #[serde(default)]
    pub hud_npc_icons: bool,
    #[serde(default)]
    pub hud_npc_names: Option<bool>,
    #[serde(default = "default_arc_opacity")]
    pub arc_opacity: u8,
    #[serde(default)]
    pub chat_context_hotkeys: bool,
    #[serde(default = "default_console_font")]
    pub console_font_px: u16,
    #[serde(default)]
    pub compact_status: bool,
    #[serde(default = "enabled")]
    pub status_panel_visible: bool,
    #[serde(default)]
    pub skill_order: Vec<u8>,
    #[serde(default)]
    pub condition_order: Vec<u8>,
    #[serde(default = "default_arc_distance")]
    pub arc_distance: u16,
    #[serde(default = "default_arc_size")]
    pub arc_size: u16,
    #[serde(default = "default_hud_names")]
    pub hud_names: [bool; 3],
    #[serde(default = "default_scale")]
    pub interface_scale: u16,
    #[serde(default)]
    pub cursor_crosshair: bool,
    #[serde(default = "default_rotation_modifier")]
    pub rotation_modifier: String,
    #[serde(default)]
    pub swap_mouse_buttons: bool,
    #[serde(default)]
    pub keyboard_delay_ms: u16,
    /// Saved HUD position and size in client pixels; editable through settings transfer.
    #[serde(default = "default_hud_bounds")]
    pub hud_bounds: [u16; 4],
    #[serde(default = "enabled")]
    pub hud_bars: bool,
    #[serde(default = "enabled")]
    pub show_effects: bool,
    #[serde(default = "enabled")]
    pub show_lights: bool,
    #[serde(default = "enabled")]
    pub show_harmony: bool,
    #[serde(default = "enabled")]
    pub show_serene: bool,
    #[serde(default)]
    pub creature_bars: bool,
    #[serde(default)]
    pub health_circle: bool,
    #[serde(default)]
    pub shortcuts_collapsed: bool,
    #[serde(default = "default_shortcuts")]
    pub shortcut_order: Vec<String>,
    #[serde(default)]
    pub arcs: ArcPreferences,
    #[serde(default)]
    pub frames: FramePreferences,
    #[serde(default)]
    pub archive_bindings: crate::play::feature_0287_actor_spell_catalog::view::ArchiveBindings,
    #[serde(default)]
    pub spellbook_filter: crate::play::feature_0287_actor_spell_catalog::view::SpellbookFilter,
    #[serde(default)]
    pub minimap: crate::play::feature_local_minimap::view::MinimapMemory,
    #[serde(default = "shortcuts::default_shortcuts")]
    pub shortcuts: Vec<String>,
    #[serde(default)]
    pub controls: ControlPreferences,
    #[serde(default)]
    pub skills_layout: crate::play::feature_0274_loyalty_skills::view::SkillsLayout,
    #[serde(default = "crate::play::feature_0017_attack::view::default_lists")]
    pub battle_lists: Vec<crate::play::feature_0017_attack::view::BattleListPreferences>,
    #[serde(default)]
    pub audio: crate::play::AudioSettings,
    #[serde(default)]
    pub keybindings: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub secondary_keybindings: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub hotkey_contexts: HotkeyProfiles,
    #[serde(default)]
    pub item_hotkeys: crate::play::feature_0179_hotkey_equip::view::ItemHotkeys,
    #[serde(default)]
    pub hotkey_profiles:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    pub english: bool,
    /// Requested world zoom percentage, fitted to the current viewport when projected.
    #[serde(default = "default_scale")]
    pub ui_scale: u16,
    #[serde(default = "default_panels")]
    pub visible_panels: Vec<String>,
}
impl Default for ReferencePreferences {
    fn default() -> Self {
        Self {
            tooltips_enabled: true,
            tooltip_delay_ms: 500,
            hud_health: true,
            hud_mana: true,
            hud_marks_own: true,
            hud_marks_other: true,
            hud_npc_icons: false,
            hud_npc_names: None,
            arc_opacity: 100,
            chat_context_hotkeys: false,
            console_font_px: 14,
            compact_status: false,
            status_panel_visible: true,
            skill_order: Vec::new(),
            condition_order: Vec::new(),
            arc_distance: 8,
            arc_size: 3,
            hud_names: default_hud_names(),
            interface_scale: 100,
            cursor_crosshair: false,
            rotation_modifier: default_rotation_modifier(),
            swap_mouse_buttons: false,
            keyboard_delay_ms: 0,
            hud_bounds: default_hud_bounds(),
            hud_bars: true,
            game_window: Default::default(),
            show_effects: true,
            show_lights: true,
            show_harmony: true,
            show_serene: true,
            creature_bars: false,
            health_circle: false,
            shortcuts_collapsed: false,
            shortcut_order: default_shortcuts(),
            spellbook_filter: Default::default(),
            archive_bindings: Default::default(),
            frames: Default::default(),
            arcs: Default::default(),
            minimap: Default::default(),
            shortcuts: shortcuts::default_shortcuts(),
            controls: ControlPreferences::default(),
            skills_layout: crate::play::feature_0274_loyalty_skills::view::SkillsLayout::default(),
            battle_lists: crate::play::feature_0017_attack::view::default_lists(),
            audio: crate::play::AudioSettings::default(),
            keybindings: std::collections::BTreeMap::new(),
            secondary_keybindings: std::collections::BTreeMap::new(),
            hotkey_contexts: HotkeyProfiles::default(),
            item_hotkeys: Default::default(),
            hotkey_profiles: std::collections::BTreeMap::new(),
            english: false,
            ui_scale: default_scale(),
            visible_panels: default_panels(),
        }
    }
}
impl ReferencePreferences {
    pub const MOUSE_PRESETS: [&'static str; 2] = ["Right-handed", "Left-handed"];

    pub fn set_mouse_preset(&mut self, index: usize) -> Result<(), PreferenceError> {
        if index >= Self::MOUSE_PRESETS.len() { return Err(PreferenceError); }
        self.swap_mouse_buttons = index == 1;
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), PreferenceError> {
        self.validate_bindings()?;
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
        if self.tooltip_delay_ms > 3000 || self.arc_opacity > 100
            || !(10..=32).contains(&self.console_font_px)
            || !crate::play::feature_0017_attack::view::validate_lists(&self.battle_lists)
            || self.arc_distance > 80
            || !(1..=8).contains(&self.arc_size)
            || !(80..=200).contains(&self.interface_scale)
            || !self.spellbook_filter.valid()
            || !self.archive_bindings.valid()
            || !self.frames.valid()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceError;

impl Display for PreferenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("Invalid preferences or conflicting key binding")
    }
}

impl std::error::Error for PreferenceError {}

#[derive(Debug, Clone)]
pub(crate) struct LocalPlayState {
    character: Option<[u8; 16]>,
    pub(in crate::play) minimap_binding:
        Option<crate::play::feature_local_minimap::view::MinimapBinding>,
    pub(in crate::play) minimap_edit: u64,
    pub(in crate::play) minimap_origin: Option<oteryn_session::ActorPosition>,
    world: Arc<World>,
    pub(in crate::play) preferences: ReferencePreferences,
    viewport: [u32; 2],
}

impl LocalPlayState {
    pub(crate) fn new(world: World) -> Self {
        Self {
            character: None,
            minimap_binding: None,
            minimap_edit: 0,
            minimap_origin: None,
            world: Arc::new(world),
            preferences: ReferencePreferences::default(),
            viewport: [960, 540],
        }
    }

    pub(crate) fn world(&self) -> Arc<World> {
        Arc::clone(&self.world)
    }

    pub(crate) fn set_world(&mut self, world: Arc<World>) {
        self.world = world;
    }
}

impl PlayView {
    pub fn preference_character(&self) -> Option<[u8; 16]> {
        self.local.character
    }

    /// Installed only after native admission has accepted the selected character.
    pub(crate) fn set_preference_character(&mut self, character: [u8; 16]) {
        self.local.character = Some(character);
    }

    pub(in crate::play) fn apply_preference_scale(
        &mut self,
    ) -> Result<(), oteryn_renderer::BatchError> {
        let pixels = crate::scene::SCENE_TILE_PX * u32::from(self.preferences().ui_scale) / 100;
        let fit = (self.local.viewport[0] / crate::scene::SCENE_COLUMNS)
            .min(self.local.viewport[1] / crate::scene::SCENE_ROWS)
            .max(1);
        self.scene.set_label_scale(self.preferences().interface_scale);
        self.scene.set_tile_px(pixels.min(fit))
    }

    pub fn set_world_viewport(&mut self, viewport: [u32; 2]) -> Result<(), PreferenceError> {
        if viewport.contains(&0) || self.local.viewport == viewport {
            return Ok(());
        }
        let mut candidate = self.clone();
        candidate.local.viewport = viewport;
        candidate
            .apply_preference_scale()
            .map_err(|_| PreferenceError)?;
        *self = candidate;
        Ok(())
    }

    #[must_use]
    pub const fn preferences(&self) -> &ReferencePreferences {
        &self.local.preferences
    }

    pub fn set_preferences(
        &mut self,
        preferences: ReferencePreferences,
    ) -> Result<(), PreferenceError> {
        preferences.validate()?;
        let mut candidate = self.clone();
        candidate.local.preferences = preferences;
        candidate.rebuild().map_err(|_| PreferenceError)?;
        *self = candidate;
        Ok(())
    }
}
