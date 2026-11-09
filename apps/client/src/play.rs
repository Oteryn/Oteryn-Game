//! Play view after admission: map, overlays, vitals and movement.
//!
//! The view projects server-owned positions, overlays and live map tiles.

#[path = "tutorial_overlay.rs"]
mod tutorial_overlay;

use crate::input::StepDir;
#[cfg(test)]
use crate::map::World;
use crate::scene::Scene;
use feature_local_reference_preferences::settings::LocalPlayState;
use input::Input;
#[cfg(test)]
use oteryn_renderer::BatchError;
use oteryn_renderer::TileCoord;
#[cfg(test)]
use oteryn_session::WorldObjectOverlayEntry;
use oteryn_session::{SessionEvent, WorldSpatialEntity};
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU32;

#[derive(Debug, Clone)]
pub struct PlayView {
    game_notifications: feature_local_game_notifications::state::State,
    ui_sounds: Vec<UiSound>,
    creature_drag: feature_0082_creature_push::state::CreatureDrag,
    world_context_feedback: Option<String>,
    death: feature_local_death_respawn::protocol::DeathState,
    target_walk_feedback: Option<&'static str>,
    served_channel: Option<oteryn_client_runtime::channel_list::ServedChannel>,
    world_weather: Option<oteryn_session::world_weather::WorldWeather>,
    world_clock: Option<oteryn_session::world_clock::WorldClock>,
    resource_hud: resource_hud::ViewState,
    party_icon_epoch: std::time::Instant,
    party_icons_refresh: Option<std::time::Instant>,
    party_icons: Vec<oteryn_client_runtime::party_skull::PartyIcon>,
    vitals: Option<oteryn_session::ActorVitals>,
    battle_instinct: Option<oteryn_client_ui::battle_instinct_status::BattleInstinctStatus>,
    battle_instinct_refresh: Option<std::time::Instant>,
    sounds: Vec<SoundCue>,
    visuals: Vec<VisualCue>,
    spell_words: std::collections::BTreeMap<u32, String>,
    pub(crate) tutorial_overlay: tutorial_overlay::Overlay,
    own: TileCoord,
    /// The direction of the latest step sent; the player is drawn facing it.
    facing: StepDir,
    floor: i16,
    /// Every decodable overlay entry with its floor; the current floor selects what is drawn.
    markers: BTreeMap<Vec<u8>, (TileCoord, i16)>,
    local: LocalPlayState,
    live_map: bool,
    minimap: feature_local_minimap::state::SharedMinimap,
    /// Added to a session position to reach the map position drawn there.
    anchor: TileCoord,
    /// The session floor the map is anchored to; on any other floor no map is drawn.
    map_floor: Option<i16>,
    scene: Scene,
    /// A step sent to the session task whose outcome has not arrived.
    in_flight: bool,
    /// Arrow, hotkey and click input not yet sent, in the order it was given.
    inputs: VecDeque<Input>,
    cursor_spell: Option<NonZeroU32>,
    /// The other entities the server reports as visible.
    entities: Vec<WorldSpatialEntity>,
    appearances: Vec<oteryn_session::VisibleAppearance>,
    ground_items: Vec<crate::input::Targetable>,
    /// The last combat line the server's answer produced, shown until the next one.
    combat_line: Option<&'static str>,
    action_line: Option<&'static str>,
    /// The last refused logout's line, shown until the next combat line or logout answer.
    logout_line: Option<String>,
    /// Recent server messages in their validated delivery order, owned by this connection.
    server_messages: VecDeque<oteryn_session::AppliedDelta<oteryn_session::ServerMessage>>,
}

mod construction;

mod end;
mod hud_layout;
mod input;
mod mailbox;
mod refusal_details;
mod resource_hud;
pub use hud_layout::{HudAction, HudDock, HudLayout, HudPane, HudPanel};
mod session_loop;
mod session_stepper;
#[cfg(windows)]
mod status_window;
mod stepper;
mod tick;
pub use resource_hud::ResourceHudOptions;
mod window_controls;
pub use window_controls::window_icon;

include!(concat!(env!("OUT_DIR"), "/play_features.rs"));

#[cfg(windows)]
pub use feature_0011_world_interaction::view::{open_context_menu, open_world_context_menu};
pub use feature_local_reference_preferences::settings::{
    ArcLayout, ArcPreferences, BASIC_GROUPS, BasicOptions, ControlPreferences, CustomAction,
    FIXED_PIXEL_FONT_PROPERTY, FramePacer, FramePreferences, GraphicsPreferences, HelpTopic,
    HoverTooltip, MiscellaneousPreferences, MousePreset, MovementKeys, ObjectTarget,
    PerformancePacer, PerformanceSettings, PreferenceBinding, PreferenceInput, PreferenceStore,
    ReferencePreferences, RotationModifier, SHORTCUTS, ScreenshotPreferences, SettingsPage,
    SettingsState, ShortcutEdit, TooltipTarget, WINDOW_ACTIONS, fit_ui_scale, help, help_text,
    preference_bindings, rescale_ui_rect, scale_ui_rect, screenshot,
};

pub use feature_0004_item_view_move::state::GroundItem;
pub use feature_0017_attack::view::GuildFilter;
pub use feature_local_minimap::view::{MinimapBinding, MinimapMark, MinimapMemory};
pub use feature_local_spell_action_bar::view::{
    BarAction, BarDock, BarGesture, BarLayout, ItemUseMode, PassiveAbility, SpellAim,
};

pub use feature_0023_presentation::state::*;

pub use end::{PlayEnd, public_class};
use feature_0013_paced_movement::protocol::step_direction;
pub use feature_0021_logout::protocol::logout_refusal_text;
pub use feature_0031_tutorial_notices::protocol::run_session;
pub use feature_0092_zones_nologout::state::ServerMessageInbox;
pub use mailbox::{Outbox, PlayEvent, PlayLink, Pushed, play_channel};
pub use session_loop::Stepper;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "screenshots_live_tests.rs"]
mod screenshots_live_tests;
