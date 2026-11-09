//! Local-only preferences whose keys come from the audited client reference surface.
//!
//! These values neither select a protocol capability nor manufacture server-owned panel data.
#[path = "capture_options.rs"]
mod capture_options;
pub use capture_options::ScreenshotPreferences;
#[path = "game_window.rs"]
mod game_window;
pub use game_window::GameWindowPreferences;
#[path = "action_bar.rs"]
mod action_bar;
pub use action_bar::ActionBarPreferences;
#[path="miscellaneous.rs"]
mod miscellaneous;
pub use miscellaneous::MiscellaneousPreferences;
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
#[path = "bar_hotkeys.rs"]
mod bar_hotkeys;
#[path = "bindings.rs"]
mod bindings;
#[path = "control_input.rs"]
mod controls;
#[path = "frames.rs"]
mod frames;
pub use frames::{FramePacer, FramePreferences};
#[path = "graphics.rs"]
mod graphics;
#[path = "gameplay.rs"]
mod gameplay;

#[path = "help.rs"]
pub mod help;
#[path = "localization.rs"]
mod localization;
#[path = "minimap_transfer.rs"]
mod minimap_transfer;
#[path = "presentation.rs"]
mod presentation;
#[path = "profile_contexts.rs"]
mod profile_contexts;
#[path = "custom_actions.rs"]
mod custom_actions;
#[path = "custom_dispatch.rs"]
mod custom_dispatch;
pub use custom_actions::{CustomAction, ObjectTarget};
#[path = "profiles.rs"]
mod profiles;
#[path = "shortcuts.rs"]
mod shortcuts;
#[path = "screenshot.rs"]
pub mod screenshot;

#[path = "status_order.rs"]
mod status_order;
#[path = "store.rs"]
mod store;
#[path = "sync.rs"]
mod sync;
pub use graphics::GraphicsPreferences;
pub use shortcuts::ShortcutEdit;
#[path = "tooltips.rs"]
mod tooltips;
#[path = "ui_geometry.rs"]
mod ui_geometry;
pub use ui_geometry::{FIXED_PIXEL_FONT_PROPERTY, fit_ui_scale, rescale_ui_rect, scale_ui_rect};
pub use tooltips::{HoverTooltip, TooltipTarget};
pub(crate) use bindings::parse_action_chord;
pub use bindings::{PreferenceBinding, PreferenceInput, preference_bindings};
pub use controls::{ControlPreferences, MousePreset, MovementKeys, RotationModifier};
pub use help::{HelpTopic, help_text};
pub use profile_contexts::HotkeyProfiles;
pub use shortcuts::SHORTCUTS;
pub use store::PreferenceStore;
pub use sync::{SettingsPage, SettingsState, WINDOW_ACTIONS};
#[path = "defaults.rs"]
mod defaults;
use defaults::*;

/// Preferences owned entirely by the native client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreferences {
    #[serde(skip)]
    pub miscellaneous: MiscellaneousPreferences,
    #[serde(default)]
    pub hud_layout: crate::play::HudLayout,
    #[serde(default)]
    pub resource_hud: crate::play::ResourceHudOptions,
    #[serde(default = "default_boss_opacity")]
    pub boss_area_opacity: u8,
    #[serde(default)]
    pub action_bar: ActionBarPreferences,
    #[serde(default)]
    pub graphics: GraphicsPreferences,
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
    #[serde(default)]
    pub manual_hotkey_profile: bool,
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
    #[serde(default)]
    pub audio_device: String,
    #[serde(default = "default_opacity")]
    pub effect_opacity: [u8; 3],
    /// Disable TCP small-packet buffering on the next gameplay connection.
    #[serde(default)]
    pub optimise_connection: bool,
    #[serde(default)]
    pub ambient_brightness: u8,
    #[serde(default = "default_foreground_fps")]
    pub foreground_fps: u16,
    #[serde(default = "default_background_fps")]
    pub background_fps: u16,
    #[serde(default)]
    pub language_code: Option<String>,
    #[serde(default)]
    pub console: presentation::ConsolePreferences,
    #[serde(default)]
    pub gameplay: gameplay::GameplayPreferences,
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
    pub screenshots: ScreenshotPreferences,
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
    #[serde(default)]
    pub active_battle_list: usize,
    #[serde(default)]
    pub action_keys: crate::play::feature_local_spell_action_bar::view::ActionKeys,
    #[serde(default)]
    pub bar_layout: crate::play::feature_local_spell_action_bar::view::BarLayout,
    #[serde(default)]
    pub bar_actions: crate::play::feature_local_spell_action_bar::view::BarActions,
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
    pub custom_hotkeys: std::collections::BTreeMap<String, CustomAction>,
    #[serde(default)]
    pub bar_hotkey_profiles: std::collections::BTreeMap<
        String,
        crate::play::feature_local_spell_action_bar::view::ActionKeys,
    >,
    #[serde(default)]
    pub bar_action_profiles: std::collections::BTreeMap<
        String,
        crate::play::feature_local_spell_action_bar::view::BarActions,
    >,
    #[serde(default)]
    pub custom_hotkey_profiles:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, CustomAction>>,
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
            miscellaneous: Default::default(),
            boss_area_opacity: default_boss_opacity(),
            graphics: GraphicsPreferences::default(),
            hud_layout: Default::default(),
            resource_hud: Default::default(),
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
            manual_hotkey_profile: false,
            console_font_px: 14,
            compact_status: false,
            status_panel_visible: true,
            skill_order: Vec::new(),
            condition_order: Vec::new(),
            audio_device: String::new(),
            effect_opacity: default_opacity(),
            optimise_connection: false,
            ambient_brightness: 0,
            foreground_fps: default_foreground_fps(),
            background_fps: default_background_fps(),
            language_code: None,
            console: Default::default(),
            action_bar: Default::default(),
            gameplay: gameplay::GameplayPreferences::default(),
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
            screenshots: Default::default(),
            arcs: Default::default(),
            minimap: Default::default(),
            shortcuts: shortcuts::default_shortcuts(),
            controls: ControlPreferences::default(),
            skills_layout: crate::play::feature_0274_loyalty_skills::view::SkillsLayout::default(),
            active_battle_list: Default::default(),
            action_keys: Default::default(),
            bar_layout: Default::default(),
            bar_actions: Default::default(),
            battle_lists: crate::play::feature_0017_attack::view::default_lists(),
            audio: crate::play::AudioSettings::default(),
            keybindings: std::collections::BTreeMap::new(),
            secondary_keybindings: std::collections::BTreeMap::new(),
            hotkey_contexts: HotkeyProfiles::default(),
            item_hotkeys: Default::default(),
            custom_hotkeys: std::collections::BTreeMap::new(),
            bar_hotkey_profiles: std::collections::BTreeMap::new(),
            bar_action_profiles: std::collections::BTreeMap::new(),
            custom_hotkey_profiles: std::collections::BTreeMap::new(),
            hotkey_profiles: std::collections::BTreeMap::new(),
            english: false,
            ui_scale: default_scale(),
            visible_panels: default_panels(),
        }
    }
}
#[path = "preference_validation.rs"]
mod preference_validation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceError;

impl Display for PreferenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(tr!("Invalid preferences or conflicting key binding"))
    }
}

impl std::error::Error for PreferenceError {}

#[path = "projection.rs"]
mod projection;
pub(crate) use projection::LocalPlayState;
