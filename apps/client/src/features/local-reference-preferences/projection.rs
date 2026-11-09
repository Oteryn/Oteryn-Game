//! Local view projection and character-bound preference state.
use super::*;
#[derive(Debug, Clone)]
pub(crate) struct LocalPlayState {
    character: Option<[u8; 16]>,
    pub(in crate::play) minimap_binding:
        Option<crate::play::feature_local_minimap::view::MinimapBinding>,
    pub(in crate::play) minimap_edit: u64,
    pub(in crate::play) minimap_origin: Option<oteryn_session::ActorPosition>,
    world: Arc<World>,
    pub(in crate::play) preferences: ReferencePreferences,
    pub(super) viewport: [u32; 2],
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
        self.scene.set_label_scale(self.preferences().interface_scale);
        self.scene.set_tile_px(self.preferences().graphics.tile_pixels(self.preferences().ui_scale,self.local.viewport))
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
