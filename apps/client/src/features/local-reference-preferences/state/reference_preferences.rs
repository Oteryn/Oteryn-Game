/// Preferences owned entirely by the native client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreferences {
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
    #[serde(default)]
    pub hud_layout: crate::play::HudLayout,
    #[serde(default)]
    pub resource_hud: crate::play::ResourceHudOptions,
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
