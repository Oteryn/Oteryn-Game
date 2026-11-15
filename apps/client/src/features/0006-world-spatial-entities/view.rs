use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{ActorPosition, WorldSpatialEntity};

impl PlayView {
    pub fn set_own_identity(&mut self, identity: [u8; 16]) {
        self.scene.set_own_identity(identity);
    }
    /// Draws the other visible entities the join snapshot carried.
    pub fn with_entities(mut self, entities: Vec<WorldSpatialEntity>) -> Result<Self, BatchError> {
        self.entities = entities;
        self.rebuild()?;
        Ok(self)
    }

    #[must_use]
    pub const fn own(&self) -> TileCoord {
        self.own
    }

    pub const fn map_center(&self) -> (i32, i32, i16) {
        (self.own.x, self.own.y, self.floor)
    }

    #[must_use]
    pub const fn facing(&self) -> StepDir {
        self.facing
    }

    pub(in crate::play) fn place(&mut self, position: ActorPosition) {
        if self.floor != position.floor {
            self.inputs.retain(|input| {
                !matches!(input, crate::play::Input::TargetWalk(target)
                    if target.floor != position.floor)
            });
        }
        self.own = TileCoord::new(position.x, position.y);
        self.floor = position.floor;
    }
}

impl PlayView {
    pub fn set_presentations(
        &mut self,
        entries: Vec<oteryn_session::VisibleAppearance>,
    ) -> Result<(), BatchError> {
        self.appearances = entries;
        self.rebuild()
    }
}

impl PlayView {
    pub(crate) fn set_appearance_palette(&mut self, palette: &[[u8; 3]]) {
        self.scene.set_appearance_palette(palette);
    }
    pub(in crate::play) fn animate_actors(
        &mut self,
        link: &crate::play::PlayLink,
        now: std::time::Instant,
    ) -> Result<(), BatchError> {
        {
            let mailbox = crate::play::mailbox::lock(&link.mailbox);
            if let Some(catalogue) = &mailbox.properties.appearance.catalogue {
                self.scene.set_appearance_palette(&catalogue.palette);
            }
        }
        let changed = self
            .scene
            .set_presentations_at(&self.appearances, self.floor, now)?;
        self.light_scene()?;
        if changed {
            self.weather_scene();
        }
        Ok(())
    }
}
