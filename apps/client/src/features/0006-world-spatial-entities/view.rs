use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{ActorPosition, WorldSpatialEntity};

impl PlayView {
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
        self.own = TileCoord::new(position.x, position.y);
        self.floor = position.floor;
    }
}
