use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{ActorPosition, WorldSpatialEntity};

impl PlayView {
    pub fn set_own_identity(&mut self, identity: [u8; 16]) {
        self.scene.set_own_identity(identity);
    }
    pub fn from_join(snapshot: &oteryn_session::JoinSnapshot) -> Result<Self, BatchError> {
        let position = snapshot.world_spatial.actor_position;
        Self::new(
            TileCoord::new(position.x, position.y),
            position.floor,
            &snapshot.world_object_overlay,
        )
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
