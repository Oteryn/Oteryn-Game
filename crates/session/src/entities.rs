//! Session entities.
use super::*;

/// The visible entities of a session that selected capability 6, keyed by `EntityRef` (identity
/// and generation). Identities are unique, at most [`MAX_SNAPSHOT_ENTITIES`] are stored, and the
/// own actor is always a stored `Player` at the own-actor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntities {
    pub(super) own_identity: [u8; world_spatial_entities::ENTITY_IDENTITY_BYTES],
    pub(super) entities: BTreeMap<EntityRef, WorldSpatialEntity>,
    // Same-tile corpse slots retain the publisher's bottom-first order.
    ordered_others: Vec<EntityRef>,
    pub(super) lights: world_spatial_entities::EntityLights,
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

    /// Every entity but the own actor, with same-tile corpses in wire stack order.
    pub fn others(&self) -> impl Iterator<Item = &WorldSpatialEntity> {
        self.ordered_others
            .iter()
            .filter_map(|reference| self.get(reference))
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
        lights: world_spatial_entities::EntityLights,
    ) -> Self {
        let mut view = Self {
            own_identity: snapshot.own_identity,
            lights,
            entities: snapshot
                .entities
                .iter()
                .map(|entity| (entity.entity, *entity))
                .collect(),
            ordered_others: Vec::new(),
        };
        view.order_others(snapshot.entities.iter());
        view
    }

    fn order_others<'a>(&mut self, arrival: impl Iterator<Item = &'a WorldSpatialEntity>) {
        let tile = |entity: &WorldSpatialEntity| {
            let position = entity.position;
            (position.floor, position.x, position.y)
        };
        let mut stacks = BTreeMap::<_, std::collections::VecDeque<_>>::new();
        for corpse in arrival.filter(|entity| entity.kind == EntityKind::Corpse) {
            stacks
                .entry(tile(corpse))
                .or_default()
                .push_back(corpse.entity);
        }
        self.ordered_others = self
            .entities
            .values()
            .filter(|entity| entity.entity.identity != self.own_identity)
            .map(|entity| {
                if entity.kind == EntityKind::Corpse {
                    stacks
                        .get_mut(&tile(entity))
                        .and_then(|stack| stack.pop_front())
                        .unwrap_or(entity.entity)
                } else {
                    entity.entity
                }
            })
            .collect();
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
    pub(super) fn after(
        &self,
        delta: &WorldSpatialEntitiesDelta,
        lights: &world_spatial_entities::EntityLights,
    ) -> Result<Self, SessionError> {
        let inconsistent = |reason| SessionError::EntityStoreInconsistent { reason };
        let mut next = self.clone();
        for reference in &delta.leave {
            next.lights.remove(&reference.identity);
            if next.entities.remove(reference).is_none() {
                return Err(inconsistent("leave of an entity that is not stored"));
            }
        }
        for entity in &delta.update {
            next.lights.remove(&entity.entity.identity);
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
        world_spatial_entities::unique_item_handles(next.entities.values())
            .map_err(|_| inconsistent("duplicate item handle in entity store"))?;
        for entity in delta.enter.iter().chain(&delta.update) {
            entity.validate_forecast_owner(&next.own_identity)?;
        }
        next.lights.extend(lights);
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
        let moved_or_new_corpse = |entity: &WorldSpatialEntity| {
            self.get(&entity.entity).is_none_or(|previous| {
                previous.position != entity.position || previous.kind != entity.kind
            })
        };
        let stayed = self
            .others()
            .filter_map(|previous| next.get(&previous.entity))
            .filter(|entity| !moved_or_new_corpse(entity));
        let arrived = delta
            .update
            .iter()
            .chain(&delta.enter)
            .filter(|entity| moved_or_new_corpse(entity));
        let order = stayed.chain(arrived).copied().collect::<Vec<_>>();
        next.order_others(order.iter());
        Ok(next)
    }
}
