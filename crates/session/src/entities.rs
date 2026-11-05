//! Session entities.
use super::*;

/// The visible entities of a session that selected capability 6, keyed by `EntityRef` (identity
/// and generation). Identities are unique, at most [`MAX_SNAPSHOT_ENTITIES`] are stored, and the
/// own actor is always a stored `Player` at the own-actor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntities {
    pub(super) own_identity: [u8; world_spatial_entities::ENTITY_IDENTITY_BYTES],
    pub(super) entities: BTreeMap<EntityRef, WorldSpatialEntity>,
}

impl WorldEntities {
    /// The own actor's identity.
    #[must_use]
    pub const fn own_identity(&self) -> &[u8; world_spatial_entities::ENTITY_IDENTITY_BYTES] {
        &self.own_identity
    }

    #[must_use]
    pub fn get(&self, entity: &EntityRef) -> Option<&WorldSpatialEntity> {
        self.entities.get(entity)
    }

    /// Every stored entity, ordered by `EntityRef`.
    pub fn iter(&self) -> impl Iterator<Item = &WorldSpatialEntity> {
        self.entities.values()
    }

    /// Every stored entity but the own actor, ordered by `EntityRef`.
    pub fn others(&self) -> impl Iterator<Item = &WorldSpatialEntity> {
        self.entities
            .values()
            .filter(|entity| entity.entity.identity != self.own_identity)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub(super) fn from_snapshot(
        snapshot: &world_spatial_entities::WorldSpatialEntitiesSnapshot,
    ) -> Self {
        Self {
            own_identity: snapshot.own_identity,
            entities: snapshot
                .entities
                .iter()
                .map(|entity| (entity.entity, *entity))
                .collect(),
        }
    }

    pub(super) fn has_identity(
        &self,
        identity: &[u8; world_spatial_entities::ENTITY_IDENTITY_BYTES],
    ) -> bool {
        self.entities
            .keys()
            .any(|stored| stored.identity == *identity)
    }

    /// The store after `delta`, or the reason the delta contradicts it. Nothing changes here.
    pub(super) fn after(&self, delta: &WorldSpatialEntitiesDelta) -> Result<Self, SessionError> {
        let inconsistent = |reason| SessionError::EntityStoreInconsistent { reason };
        let mut next = self.clone();
        for reference in &delta.leave {
            if next.entities.remove(reference).is_none() {
                return Err(inconsistent("leave of an entity that is not stored"));
            }
        }
        for entity in &delta.update {
            match next.entities.get_mut(&entity.entity) {
                Some(stored) => *stored = *entity,
                None => return Err(inconsistent("update of an entity that is not stored")),
            }
        }
        for entity in &delta.enter {
            if next.has_identity(&entity.entity.identity) {
                return Err(inconsistent("enter of an identity that is already stored"));
            }
            next.entities.insert(entity.entity, *entity);
        }
        if next.entities.len() > MAX_SNAPSHOT_ENTITIES {
            return Err(inconsistent("more entities than the snapshot bound"));
        }
        let own_at_header = next.entities.values().any(|entity| {
            entity.kind == EntityKind::Player
                && entity.entity.identity == next.own_identity
                && entity.position == delta.actor_position
        });
        if !own_at_header {
            return Err(inconsistent(
                "own actor is gone or not at the header position",
            ));
        }
        Ok(next)
    }
}
