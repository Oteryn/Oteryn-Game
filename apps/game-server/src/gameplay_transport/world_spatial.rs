//! `FIRST-CONTROL-WIRE-V1` typed payload codecs (#642 owner acceptance 5853424280).
//!
//! Moved to `oteryn-protocol-oteryn::world_spatial` (ADR-0011 §2, #162 A6, task
//! `OTV2-20260928-protocol-oteryn-crate-c1a`): a client also needs these codecs, and that crate
//! has no dependency on this one. This module re-exports the whole original `pub(crate)` surface
//! unchanged, so nothing else in this crate needed to change.

// The caller is the connection composition (VIS-3); until then only the tests exercise it.
#![cfg_attr(not(test), allow(dead_code))]

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
#[allow(clippy::expect_used)]
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
}
