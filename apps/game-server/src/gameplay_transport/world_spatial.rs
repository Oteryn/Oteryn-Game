//! `FIRST-CONTROL-WIRE-V1` typed payload codecs (#642 owner acceptance 5853424280).
//!
//! Moved to `oteryn-protocol-oteryn::world_spatial` (ADR-0011 §2, #162 A6, task
//! `OTV2-20260928-protocol-oteryn-crate-c1a`): a client also needs these codecs, and that crate
//! has no dependency on this one. This module re-exports the whole original `pub(crate)` surface
//! unchanged, so nothing else in this crate needed to change.

use std::collections::BTreeMap;

use crate::movement::interest::{
    InterestChange, InterestDiff, InterestEntity, InterestIndex, VisibilityPosition,
    VisibilityQuery, VisibilitySettings, diff_interest,
};
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
pub(crate) use oteryn_protocol_oteryn::world_spatial::*;
pub(crate) use oteryn_protocol_oteryn::world_spatial_entities::*;

/// Server side: the payload type and bytes one session receives for a full snapshot. A session
/// that selected capability 6 gets the entity revision; every other session keeps the v1 type
/// with its own actor only. With capability 4 every object also carries the session's item
/// handle (ITEM-VIEW-1b, attached by `item_view::SessionItemView`).
pub(crate) fn encode_visibility_snapshot(
    selected_capabilities: &[u32],
    snapshot: &WorldSpatialEntitiesSnapshot,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let payload = if selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1) {
            encode_world_spatial_entities_snapshot_with_item_handles(snapshot)?
        } else {
            encode_world_spatial_entities_snapshot(snapshot)?
        };
        Ok((SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
    } else {
        validate_snapshot(snapshot)?;
        Ok((
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&WorldSpatialObservation {
                content_generation: snapshot.content_generation,
                actor_position: snapshot.actor_position,
            }),
        ))
    }
}

/// Server side: the payload type and bytes one session receives for a delta, gated as above.
pub(crate) fn encode_visibility_delta(
    selected_capabilities: &[u32],
    delta: &WorldSpatialEntitiesDelta,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let payload = if selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1) {
            encode_world_spatial_entities_delta_with_item_handles(delta)?
        } else {
            encode_world_spatial_entities_delta(delta)?
        };
        Ok((DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
    } else {
        validate_delta(delta)?;
        Ok((
            DELTA_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&WorldSpatialObservation {
                content_generation: delta.content_generation,
                actor_position: delta.actor_position,
            }),
        ))
    }
}

/// VIS-3 known gap: the runtime carries no facing yet, so every actor faces south.
pub(crate) const PLACEHOLDER_ACTOR_DIRECTION: StepDirection = StepDirection::South;
/// VIS-3 known gap: the runtime carries no appearance yet.
pub(crate) const PLACEHOLDER_APPEARANCE_REF: u32 = 0;
/// VIS-3 known gap: the runtime keeps no maximum health beside the slot, so a live actor shows
/// full health.
pub(crate) const PLACEHOLDER_HEALTH_PERCENT: u8 = 100;

/// VIS-3: the kind of one Channel entity, as the Channel owner reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///
/// Corpses are not shown yet: their item binding (`i00005801`, D3-7) waits on the Content revision.
pub(crate) enum VisibleKind {
    Player,
    Creature,
}

/// VIS-3: one entity of the session's Channel, before the interest query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelEntity {
    pub(crate) kind: VisibleKind,
    pub(crate) identity: [u8; ENTITY_IDENTITY_BYTES],
    /// The actor generation.
    pub(crate) generation: u64,
    pub(crate) position: ActorPosition,
    /// The position revision: a changed revision is an update.
    pub(crate) revision: u64,
}

/// VIS-3: every entity of the session's Channel and the observer's identity among them, read in
/// one Channel-owner work item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelEntities {
    pub(crate) content_generation: [u8; 32],
    pub(crate) observer: [u8; ENTITY_IDENTITY_BYTES],
    pub(crate) entities: Vec<ChannelEntity>,
}

fn wire_entity(entity: &ChannelEntity) -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind: match entity.kind {
            VisibleKind::Player => EntityKind::Player,
            VisibleKind::Creature => EntityKind::Creature,
        },
        entity: EntityRef {
            identity: entity.identity,
            generation: entity.generation,
        },
        position: entity.position,
        detail: EntityDetail::Actor {
            direction: PLACEHOLDER_ACTOR_DIRECTION,
            appearance_ref: PLACEHOLDER_APPEARANCE_REF,
            health_percent: PLACEHOLDER_HEALTH_PERCENT,
        },
    }
}

/// VIS-3: what one visibility refresh sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisibilityUpdate {
    /// Nothing the session sees changed.
    Unchanged,
    /// At most `MOVE-RL-08` changes.
    Delta(WorldSpatialEntitiesDelta),
    /// A larger change: the full current view (the resync disposition, MOVE-RL-11 §4.3).
    Snapshot(WorldSpatialEntitiesSnapshot),
}

/// VIS-3 (MOVE-RL-11 §4): the domain 1 view of one session with capability 6, composed from the
/// VIS-1 interest set of its Channel. The query keeps the own actor and then at most 255 others
/// in canonical order (the degrade disposition); each refresh is diffed against what the
/// session was last sent. It lives with the connection: a resumed connection starts from a new
/// snapshot.
#[derive(Debug, Default)]
pub(crate) struct SessionVisibility {
    shown: Option<VisibilityQuery>,
    entities: BTreeMap<[u8; ENTITY_IDENTITY_BYTES], WorldSpatialEntity>,
}

impl SessionVisibility {
    /// The query of `channel` from the observer, and each selected entity in its wire form, in
    /// the query's order (own actor first). `attach` gives each object its item handle under
    /// capability 4. An observer that is not in `channel`, or not on a valid floor, fails.
    fn select(
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<(VisibilityQuery, Vec<WorldSpatialEntity>), WorldSpatialError> {
        let mut index = InterestIndex::new();
        let mut by_identity = BTreeMap::new();
        for entity in &channel.entities {
            // An entity off the 0..=15 floors cannot be shown; the observer then fails below.
            let Ok(position) = VisibilityPosition::new(
                entity.position.x,
                entity.position.y,
                entity.position.floor,
            ) else {
                continue;
            };
            index.upsert(InterestEntity {
                identity: entity.identity,
                position,
                revision: entity.revision,
            });
            by_identity.insert(entity.identity, entity);
        }
        let query = index
            .query(&channel.observer, VisibilitySettings::REFERENCE)
            .map_err(|_| WorldSpatialError::Malformed)?;
        let mut selected = query
            .entities()
            .iter()
            .map(|entity| {
                by_identity
                    .get(&entity.identity)
                    .map(|entity| wire_entity(entity))
                    .ok_or(WorldSpatialError::Malformed)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if selected
            .first()
            .is_none_or(|own| own.kind != EntityKind::Player)
        {
            return Err(WorldSpatialError::Malformed);
        }
        attach(&mut selected)?;
        Ok((query, selected))
    }

    fn snapshot_of(
        channel: &ChannelEntities,
        selected: &[WorldSpatialEntity],
    ) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
        let own = selected.first().ok_or(WorldSpatialError::Malformed)?;
        Ok(WorldSpatialEntitiesSnapshot {
            content_generation: channel.content_generation,
            actor_position: own.position,
            own_identity: own.entity.identity,
            entities: selected.to_vec(),
        })
    }

    fn show(&mut self, query: VisibilityQuery, selected: Vec<WorldSpatialEntity>) {
        self.shown = Some(query);
        self.entities = selected
            .into_iter()
            .map(|entity| (entity.entity.identity, entity))
            .collect();
    }

    /// The full current view, for the join, resync or resume snapshot.
    pub(crate) fn snapshot(
        &mut self,
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
        let (query, selected) = Self::select(channel, attach)?;
        let snapshot = Self::snapshot_of(channel, &selected)?;
        self.show(query, selected);
        Ok(snapshot)
    }

    /// What changed since the session was last sent its view: a delta of at most 256 entries, or
    /// a snapshot when the change is larger (or nothing was shown yet).
    pub(crate) fn refresh(
        &mut self,
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<VisibilityUpdate, WorldSpatialError> {
        let (query, selected) = Self::select(channel, attach)?;
        let Some(shown) = &self.shown else {
            let snapshot = Self::snapshot_of(channel, &selected)?;
            self.show(query, selected);
            return Ok(VisibilityUpdate::Snapshot(snapshot));
        };
        let changes = match diff_interest(shown, &query) {
            InterestDiff::Resync(_) => {
                let snapshot = Self::snapshot_of(channel, &selected)?;
                self.show(query, selected);
                return Ok(VisibilityUpdate::Snapshot(snapshot));
            }
            InterestDiff::Delta(changes) => changes,
        };
        let current: BTreeMap<_, _> = selected
            .iter()
            .map(|entity| (entity.entity.identity, *entity))
            .collect();
        let own = selected.first().ok_or(WorldSpatialError::Malformed)?;
        let mut delta = WorldSpatialEntitiesDelta {
            content_generation: channel.content_generation,
            actor_position: own.position,
            enter: Vec::new(),
            update: Vec::new(),
            leave: Vec::new(),
        };
        for change in &changes {
            match change {
                InterestChange::Enter(entity) => delta.enter.push(
                    *current
                        .get(&entity.identity)
                        .ok_or(WorldSpatialError::Malformed)?,
                ),
                InterestChange::Update(entity) => delta.update.push(
                    *current
                        .get(&entity.identity)
                        .ok_or(WorldSpatialError::Malformed)?,
                ),
                InterestChange::Leave(identity) => delta.leave.push(
                    self.entities
                        .get(identity)
                        .ok_or(WorldSpatialError::Malformed)?
                        .entity,
                ),
            }
        }
        self.show(query, selected);
        if changes.is_empty() {
            Ok(VisibilityUpdate::Unchanged)
        } else {
            Ok(VisibilityUpdate::Delta(delta))
        }
    }
}

/// Server side (SPEED-1, CONDITIONS-0 §4.3): the command type 1 result one session receives.
/// `TOO_EARLY` reaches only a session that selected capability 13 `PACED_MOVEMENT_V1`; every other
/// session gets `REJECTED` for the same refusal.
pub(crate) fn encode_step_outcome(
    selected_capabilities: &[u32],
    disposition: StepDisposition,
) -> Vec<u8> {
    match disposition {
        StepDisposition::TooEarly
            if !selected_capabilities.contains(&CAPABILITY_PACED_MOVEMENT_V1) =>
        {
            encode_step_result(StepDisposition::Rejected)
        }
        disposition => encode_step_result(disposition),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn entity(n: u8, kind: EntityKind) -> WorldSpatialEntity {
        let mut identity = [0_u8; ENTITY_IDENTITY_BYTES];
        identity[15] = n;
        WorldSpatialEntity {
            kind,
            entity: EntityRef {
                identity,
                generation: u64::from(n),
            },
            position: ActorPosition {
                x: i32::from(n),
                y: 0,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 3,
                health_percent: 50,
            },
        }
    }

    fn snapshot() -> WorldSpatialEntitiesSnapshot {
        let own = entity(0, EntityKind::Player);
        WorldSpatialEntitiesSnapshot {
            content_generation: [7; 32],
            actor_position: own.position,
            own_identity: own.entity.identity,
            entities: vec![own, entity(1, EntityKind::Creature)],
        }
    }

    #[test]
    fn only_a_session_that_selected_capability_6_receives_the_entity_revision() {
        let snapshot = snapshot();
        // Old client against a new server: v1 type and payload, own actor only.
        let (kind, payload) = encode_visibility_snapshot(&[], &snapshot).expect("v1");
        assert_eq!(kind, SNAPSHOT_TYPE_WORLD_SPATIAL_V1);
        assert_eq!(
            decode_world_spatial(&payload),
            Ok(WorldSpatialObservation {
                content_generation: snapshot.content_generation,
                actor_position: snapshot.actor_position,
            })
        );
        // A session that selected another capability still gets v1.
        assert_eq!(
            encode_visibility_snapshot(&[1], &snapshot).expect("v1").0,
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1
        );
        // New client against a new server: type 2 carries every entity.
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&selected, &snapshot).expect("v2");
        assert_eq!(kind, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2);
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(snapshot))
        );
        // New client against an old server: it selected nothing and decodes the v1 type.
        let (kind, payload) = encode_visibility_snapshot(&[], &self::snapshot()).expect("v1");
        assert!(matches!(
            decode_world_spatial_snapshot_view(&[], kind, &payload),
            Ok(WorldSpatialSnapshotView::OwnActor(_))
        ));
    }

    #[test]
    fn delta_follows_the_same_gate_and_refuses_an_over_bound_change() {
        let delta = WorldSpatialEntitiesDelta {
            content_generation: [7; 32],
            actor_position: ActorPosition {
                x: 2,
                y: 0,
                floor: 7,
            },
            enter: vec![entity(2, EntityKind::Creature)],
            update: vec![entity(0, EntityKind::Player)],
            leave: vec![entity(1, EntityKind::Creature).entity],
        };
        let (kind, payload) = encode_visibility_delta(&[], &delta).expect("v1");
        assert_eq!(kind, DELTA_TYPE_WORLD_SPATIAL_V1);
        assert!(decode_world_spatial(&payload).is_ok());
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_delta(&selected, &delta).expect("v2");
        assert_eq!(kind, DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2);
        assert_eq!(
            decode_world_spatial_delta_view(&selected, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta.clone()))
        );
        // The bound applies to v1 sessions too: the change is one the server must resync.
        let mut over = delta;
        over.leave = (0..=255_u8)
            .map(|n| entity(n, EntityKind::Creature).entity)
            .collect();
        assert_eq!(
            encode_visibility_delta(&[], &over),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn too_early_is_sent_only_to_a_session_with_capability_13() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        assert_eq!(
            decode_step_result_paced(
                &encode_step_outcome(&paced, StepDisposition::TooEarly),
                true
            ),
            Ok(StepDisposition::TooEarly)
        );
        assert_eq!(
            decode_step_result(&encode_step_outcome(&[], StepDisposition::TooEarly)),
            Ok(StepDisposition::Rejected)
        );
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            for selected in [&paced[..], &[]] {
                assert_eq!(
                    encode_step_outcome(selected, disposition),
                    encode_step_result(disposition)
                );
            }
        }
    }

    /// ITEM-VIEW-1b: with capability 4 every object carries its item handle on the snapshot and
    /// the delta; without it the handle is never sent.
    #[test]
    fn an_object_carries_its_item_handle_only_with_capability_4() {
        let corpse = |handle: Option<u64>| WorldSpatialEntity {
            detail: EntityDetail::Object {
                item_definition_ref: 4240,
                quantity: 1,
                item_handle: handle.and_then(oteryn_protocol_oteryn::item_view::ItemHandle::new),
            },
            entity: EntityRef {
                identity: [9; ENTITY_IDENTITY_BYTES],
                generation: 0,
            },
            ..entity(9, EntityKind::Corpse)
        };
        let mut handled = snapshot();
        handled.entities.push(corpse(Some(5)));
        let with = [
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ITEM_VIEW_MOVE_V1,
        ];
        let (kind, payload) = encode_visibility_snapshot(&with, &handled).expect("handles");
        assert_eq!(
            decode_world_spatial_snapshot_view(&with, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(handled.clone()))
        );
        // Without capability 4 the handle is not sent.
        let without = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&without, &handled).expect("no handles");
        let mut unhandled = handled.clone();
        unhandled.entities[2] = corpse(None);
        assert_eq!(
            decode_world_spatial_snapshot_view(&without, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(unhandled.clone()))
        );
        // With capability 4 an object without its handle fails closed.
        assert!(encode_visibility_snapshot(&with, &unhandled).is_err());
        let delta = WorldSpatialEntitiesDelta {
            content_generation: [7; 32],
            actor_position: handled.actor_position,
            enter: vec![corpse(Some(5))],
            update: Vec::new(),
            leave: Vec::new(),
        };
        let (kind, payload) = encode_visibility_delta(&with, &delta).expect("handles");
        assert_eq!(
            decode_world_spatial_delta_view(&with, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta))
        );
    }

    // VIS-3: the session's view composed from the interest set.

    fn id(n: u32) -> [u8; ENTITY_IDENTITY_BYTES] {
        let mut identity = [0_u8; ENTITY_IDENTITY_BYTES];
        identity[12..].copy_from_slice(&n.to_be_bytes());
        identity
    }

    fn at(n: u32, kind: VisibleKind, x: i32, y: i32, floor: i16) -> ChannelEntity {
        ChannelEntity {
            kind,
            identity: id(n),
            generation: 1,
            position: ActorPosition { x, y, floor },
            revision: 0,
        }
    }

    /// The observer, `id(0)`, stands at (100, 100, 7).
    fn channel(others: impl IntoIterator<Item = ChannelEntity>) -> ChannelEntities {
        let mut entities = vec![at(0, VisibleKind::Player, 100, 100, 7)];
        entities.extend(others);
        ChannelEntities {
            content_generation: [7; 32],
            observer: id(0),
            entities,
        }
    }

    fn no_handles(_: &mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError> {
        Ok(())
    }

    fn identities(entities: &[WorldSpatialEntity]) -> Vec<[u8; ENTITY_IDENTITY_BYTES]> {
        entities
            .iter()
            .map(|entity| entity.entity.identity)
            .collect()
    }

    #[test]
    fn the_snapshot_is_the_interest_set_own_actor_first_in_canonical_order() {
        let channel = channel([
            at(5, VisibleKind::Creature, 102, 100, 7),
            at(4, VisibleKind::Player, 99, 101, 7),
            at(3, VisibleKind::Player, 101, 100, 7),
            // Outside the 18 x 14 area, and on a floor a ground observer does not see.
            at(6, VisibleKind::Creature, 120, 100, 7),
            at(7, VisibleKind::Creature, 100, 100, 8),
            // Above ground: floor 6 is offset by one tile, so (101, 101, 6) is at ring 0.
            at(8, VisibleKind::Player, 101, 101, 6),
        ]);
        let mut view = SessionVisibility::default();
        let snapshot = view.snapshot(&channel, &mut no_handles).expect("snapshot");
        assert_eq!(
            identities(&snapshot.entities),
            [id(0), id(3), id(4), id(5), id(8)]
        );
        assert_eq!(snapshot.own_identity, id(0));
        assert_eq!(snapshot.actor_position, channel.entities[0].position);
        assert_eq!(snapshot.content_generation, [7; 32]);
        assert_eq!(snapshot.entities[0].kind, EntityKind::Player);
        assert_eq!(snapshot.entities[2].kind, EntityKind::Player);
        assert_eq!(snapshot.entities[3].kind, EntityKind::Creature);
        assert_eq!(
            snapshot.entities[3].detail,
            EntityDetail::Actor {
                direction: PLACEHOLDER_ACTOR_DIRECTION,
                appearance_ref: PLACEHOLDER_APPEARANCE_REF,
                health_percent: PLACEHOLDER_HEALTH_PERCENT,
            }
        );
        // It is a valid type 2 snapshot, and a v1 session still gets its own actor only.
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&selected, &snapshot).expect("v2");
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(snapshot.clone()))
        );
        assert_eq!(
            encode_visibility_snapshot(&[], &snapshot).expect("v1").0,
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1
        );
    }

    #[test]
    fn an_observer_outside_its_channel_fails_closed() {
        let mut lost = channel([at(1, VisibleKind::Creature, 101, 100, 7)]);
        lost.observer = id(9);
        assert!(
            SessionVisibility::default()
                .snapshot(&lost, &mut no_handles)
                .is_err()
        );
        // The observer must be a player.
        let mut creature = channel([]);
        creature.entities[0].kind = VisibleKind::Creature;
        assert!(
            SessionVisibility::default()
                .refresh(&creature, &mut no_handles)
                .is_err()
        );
    }

    /// `count` others in the cell east of the observer, then one entity five tiles west.
    fn crowd(count: u32) -> ChannelEntities {
        channel(
            (1..=count)
                .map(|n| at(n, VisibleKind::Creature, 101, 100, 7))
                .chain([at(9_999, VisibleKind::Creature, 95, 100, 7)]),
        )
    }

    #[test]
    fn the_256_ceiling_keeps_the_nearest_and_the_rest_enter_as_others_leave() {
        // 256 entities, own actor included: all of them.
        let mut view = SessionVisibility::default();
        let all = view.snapshot(&crowd(254), &mut no_handles).expect("256");
        assert_eq!(all.entities.len(), MAX_SNAPSHOT_ENTITIES);
        assert_eq!(
            all.entities.last().map(|e| e.entity.identity),
            Some(id(9_999))
        );
        // 257: the farthest is cut (degrade), never an error.
        let mut view = SessionVisibility::default();
        let cut = view.snapshot(&crowd(255), &mut no_handles).expect("257");
        assert_eq!(cut.entities.len(), MAX_SNAPSHOT_ENTITIES);
        assert!(!identities(&cut.entities).contains(&id(9_999)));
        assert!(encode_visibility_snapshot(&[CAPABILITY_WORLD_SPATIAL_ENTITIES], &cut).is_ok());
        // One near entity leaves: the farthest enters in the same delta.
        let mut fewer = crowd(255);
        fewer.entities.retain(|entity| entity.identity != id(7));
        let VisibilityUpdate::Delta(delta) = view.refresh(&fewer, &mut no_handles).expect("delta")
        else {
            panic!("expected a delta");
        };
        assert_eq!(identities(&delta.enter), [id(9_999)]);
        assert_eq!(
            delta.leave.iter().map(|e| e.identity).collect::<Vec<_>>(),
            [id(7)]
        );
        assert!(delta.update.is_empty());
    }

    /// The observer and `count` creatures from `first`, east of it in the area.
    fn line(first: u32, count: u32) -> ChannelEntities {
        channel((0..count).map(|n| {
            let x = 101 + i32::try_from(n % 9).expect("x");
            at(first + n, VisibleKind::Creature, x, 100, 7)
        }))
    }

    #[test]
    fn a_change_of_up_to_256_entries_is_a_delta_and_a_larger_one_a_snapshot() {
        // 128 leave and 128 enter: 256 entries, one delta.
        let mut view = SessionVisibility::default();
        view.snapshot(&line(1, 128), &mut no_handles)
            .expect("snapshot");
        let VisibilityUpdate::Delta(delta) = view
            .refresh(&line(1_001, 128), &mut no_handles)
            .expect("refresh")
        else {
            panic!("expected a delta");
        };
        assert_eq!((delta.enter.len(), delta.leave.len()), (128, 128));
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_delta(&selected, &delta).expect("bounded");
        assert_eq!(
            decode_world_spatial_delta_view(&selected, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta))
        );
        // 129 leave and 129 enter: 258 entries, a new snapshot of the current view (resync).
        let mut view = SessionVisibility::default();
        view.snapshot(&line(1, 129), &mut no_handles)
            .expect("snapshot");
        let next = line(1_001, 129);
        let VisibilityUpdate::Snapshot(snapshot) =
            view.refresh(&next, &mut no_handles).expect("refresh")
        else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.entities.len(), 130);
        assert_eq!(snapshot.entities[0].entity.identity, id(0));
        // The view now shows the snapshot: nothing changed since.
        assert_eq!(
            view.refresh(&next, &mut no_handles),
            Ok(VisibilityUpdate::Unchanged)
        );
    }

    #[test]
    fn a_moved_or_restamped_entity_is_an_update_and_the_own_step_moves_the_header() {
        let mut view = SessionVisibility::default();
        let first = channel([at(1, VisibleKind::Creature, 101, 100, 7)]);
        view.snapshot(&first, &mut no_handles).expect("snapshot");
        assert_eq!(
            view.refresh(&first, &mut no_handles),
            Ok(VisibilityUpdate::Unchanged)
        );
        let mut stepped = first.clone();
        stepped.entities[0].position.x = 101;
        stepped.entities[0].revision = 1;
        stepped.entities[1].revision = 4;
        let VisibilityUpdate::Delta(delta) =
            view.refresh(&stepped, &mut no_handles).expect("delta")
        else {
            panic!("expected a delta");
        };
        assert_eq!(delta.actor_position, stepped.entities[0].position);
        assert_eq!(identities(&delta.update), [id(0), id(1)]);
        assert!(delta.enter.is_empty() && delta.leave.is_empty());
    }
}
