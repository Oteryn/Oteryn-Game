//! Shared lookup of the server-confirmed own actor.
use super::{Session, SessionStream, WorldSpatialEntity};
pub(super) fn get<S: SessionStream + Send>(session: &Session<S>) -> Option<WorldSpatialEntity> {
    let entities = session.world_entities()?;
    entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .copied()
}
