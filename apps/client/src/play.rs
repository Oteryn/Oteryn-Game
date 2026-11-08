//! Play view after admission: map, overlays, vitals and movement.
//!
//! The view projects server-owned positions, overlays and live map tiles.

#[path = "tutorial_overlay.rs"]
mod tutorial_overlay;

use crate::input::StepDir;
use crate::map::World;
use crate::scene::Scene;
use feature_local_reference_preferences::settings::LocalPlayState;
use input::Input;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{SessionEvent, WorldObjectOverlayEntry, WorldSpatialEntity};
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU32;

#[derive(Debug, Clone)]
pub struct PlayView {
    creature_drag: feature_0082_creature_push::state::CreatureDrag,
    world_context_feedback: Option<String>,
    death: feature_local_death_respawn::protocol::DeathState,
    target_walk_feedback: Option<&'static str>,
    served_channel: Option<oteryn_client_runtime::channel_list::ServedChannel>,
    world_weather: Option<oteryn_session::world_weather::WorldWeather>,
    world_clock: Option<oteryn_session::world_clock::WorldClock>,
    party_icons: Vec<oteryn_client_runtime::party_skull::PartyIcon>,
    vitals: Option<oteryn_session::ActorVitals>,
    battle_instinct: Option<oteryn_client_ui::battle_instinct_status::BattleInstinctStatus>,
    battle_instinct_refresh: Option<std::time::Instant>,
    sounds: Vec<SoundCue>,
    visuals: Vec<VisualCue>,
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

impl PlayView {
    pub fn new(
        own: TileCoord,
        floor: i16,
        overlay: &[WorldObjectOverlayEntry],
    ) -> Result<Self, BatchError> {
        let local = LocalPlayState::new(World::builtin()?);
        let anchor = TileCoord::new(0, 0);
        let mut view = Self {
            creature_drag: Default::default(),
            world_context_feedback: None,
            death: Default::default(),
            target_walk_feedback: None,
            world_clock: None,
            served_channel: None,
            world_weather: None,
            party_icons: Vec::new(),
            vitals: None,
            battle_instinct: None,
            battle_instinct_refresh: None,
            sounds: Vec::new(),
            visuals: Vec::new(),
            tutorial_overlay: Default::default(),
            own,
            facing: StepDir::South,
            floor,
            markers: BTreeMap::new(),
            scene: Scene::centered_on(local.world(), None, own, StepDir::South, &[])?,
            local,
            live_map: false,
            minimap: Default::default(),
            anchor,
            map_floor: None,
            in_flight: false,
            inputs: VecDeque::new(),
            cursor_spell: None,
            entities: Vec::new(),
            appearances: Vec::new(),
            ground_items: Vec::new(),
            combat_line: None,
            action_line: None,
            logout_line: None,
            server_messages: VecDeque::new(),
        };
        for entry in overlay {
            view.set_marker(entry);
        }
        view.rebuild()?;
        Ok(view)
    }
}

mod end;
mod input;
mod mailbox;
mod refusal_details;
mod session_loop;
mod session_stepper;
#[cfg(windows)]
mod status_window;
mod stepper;
mod tick;

include!(concat!(env!("OUT_DIR"), "/play_features.rs"));

#[cfg(windows)]
pub use feature_0011_world_interaction::view::{open_context_menu, open_world_context_menu};
pub use feature_local_reference_preferences::settings::{
    ArcLayout, ArcPreferences, BASIC_GROUPS, BasicOptions, ControlPreferences, FramePacer,
    FramePreferences, HelpTopic, MousePreset, MovementKeys, PreferenceBinding, PreferenceInput,
    PreferenceStore, ReferencePreferences, RotationModifier, SHORTCUTS, SettingsPage,
    SettingsState, WINDOW_ACTIONS, help_text, preference_bindings,
};

pub use feature_0004_item_view_move::state::GroundItem;
pub use feature_local_minimap::view::{MinimapBinding, MinimapMark, MinimapMemory};
pub use feature_local_spell_action_bar::view::{
    BarAction, BarDock, BarGesture, BarLayout, ItemUseMode, PassiveAbility,
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
