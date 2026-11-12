use super::{LocalPlayState, PlayView, Scene, StepDir, TileCoord};
use crate::map::World;
use oteryn_renderer::BatchError;
use oteryn_session::WorldObjectOverlayEntry;
use std::collections::{BTreeMap, VecDeque};

impl PlayView {
    /// Keep view-owned notifications and editors inside the selected shared frontend.
    pub fn use_shared_frontend(&mut self) {
        self.shared_frontend = true;
        self.game_notifications.use_shared_frontend();
    }

    pub fn new(
        own: TileCoord,
        floor: i16,
        overlay: &[WorldObjectOverlayEntry],
    ) -> Result<Self, BatchError> {
        let local = LocalPlayState::new(World::builtin()?);
        let anchor = TileCoord::new(0, 0);
        let mut view = Self {
            shared_frontend: false,
            game_notifications: Default::default(),
            touch: Default::default(),
            creature_drag: Default::default(),
            world_context_feedback: None,
            death: Default::default(),
            target_walk_feedback: None,
            world_clock: None,
            served_channel: None,
            world_weather: None,
            resource_hud: Default::default(),
            party_icon_epoch: std::time::Instant::now(),
            party_icons_refresh: None,
            party_icons: Vec::new(),
            vitals: None,
            battle_instinct: None,
            battle_instinct_refresh: None,
            sounds: Vec::new(),
            ui_sounds: Vec::new(),
            visuals: Vec::new(),
            hit_flash: None,
            spell_words: std::collections::BTreeMap::new(),
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
