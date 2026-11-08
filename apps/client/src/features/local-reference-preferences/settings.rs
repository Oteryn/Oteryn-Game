//! Local-only preferences whose keys come from the audited client reference surface.
//!
//! These values neither select a protocol capability nor manufacture server-owned panel data.

use crate::map::World;
use crate::play::PlayView;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

#[path = "bindings.rs"]
mod bindings;
#[path = "profiles.rs"]
mod profiles;
#[path = "store.rs"]
mod store;
#[path = "sync.rs"]
mod sync;
pub use bindings::{PreferenceBinding, PreferenceInput, preference_bindings};
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

fn enabled() -> bool {
    true
}

fn default_arc_distance() -> u16 {
    8
}
fn default_arc_size() -> u16 {
    3
}

fn default_hud_names() -> [bool; 3] {
    [true; 3]
}

fn default_shortcuts() -> Vec<String> {
    WINDOW_ACTIONS
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect()
}

fn default_hud_bounds() -> [u16; 4] {
    [8, 8, 900, 40]
}

fn default_scale() -> u16 {
    100
}

fn default_rotation_modifier() -> String {
    "CTRL".into()
}

fn default_panels() -> Vec<String> {
    ["battle", "chat", "inventory", "minimap"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// Preferences owned entirely by the native client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreferences {
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
    pub audio: crate::play::AudioSettings,
    #[serde(default)]
    pub keybindings: std::collections::BTreeMap<String, String>,
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
            show_effects: true,
            show_lights: true,
            show_harmony: true,
            show_serene: true,
            creature_bars: false,
            health_circle: false,
            shortcuts_collapsed: false,
            shortcut_order: default_shortcuts(),
            audio: crate::play::AudioSettings::default(),
            keybindings: std::collections::BTreeMap::new(),
            hotkey_profiles: std::collections::BTreeMap::new(),
            english: false,
            ui_scale: default_scale(),
            visible_panels: default_panels(),
        }
    }
}

impl ReferencePreferences {
    pub(super) fn validate(&self) -> Result<(), PreferenceError> {
        self.validate_bindings()?;
        self.validate_profiles()?;
        if self.shortcut_order.len() > WINDOW_ACTIONS.len()
            || self.shortcut_order.iter().enumerate().any(|(i, id)| {
                !WINDOW_ACTIONS.iter().any(|(key, _)| key == id)
                    || self.shortcut_order[..i].contains(id)
            })
        {
            return Err(PreferenceError);
        }
        if self.arc_distance > 80
            || !(1..=8).contains(&self.arc_size)
            || !(80..=200).contains(&self.interface_scale)
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

    #[must_use]
    pub fn panel_visible(&self, id: &str) -> bool {
        self.visible_panels.iter().any(|visible| visible == id)
    }

    pub fn scaled_hud_bounds(&self) -> [u16; 4] {
        self.hud_bounds
            .map(|value| (u32::from(value) * u32::from(self.interface_scale) / 100) as u16)
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
    world: Arc<World>,
    preferences: ReferencePreferences,
    viewport: [u32; 2],
}

impl LocalPlayState {
    pub(crate) fn new(world: World) -> Self {
        Self {
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
    pub(in crate::play) fn apply_preference_scale(
        &mut self,
    ) -> Result<(), oteryn_renderer::BatchError> {
        let pixels = crate::scene::SCENE_TILE_PX * u32::from(self.preferences().ui_scale) / 100;
        let fit = (self.local.viewport[0] / crate::scene::SCENE_COLUMNS)
            .min(self.local.viewport[1] / crate::scene::SCENE_ROWS)
            .max(1);
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
