use super::*;

#[test]
fn m40_diagonal_direction_codes_round_trip_without_changing_cardinals() -> Result<(), String> {
    for value in 1..=8 {
        let payload = [8, value];
        let direction = decode_step_intent_profile(&payload, true)
            .map_err(|error| format!("native direction {value}: {error:?}"))?;
        assert_eq!(encode_step_intent(direction), payload);
        if value > 4 {
            assert_eq!(
                decode_step_intent(&payload),
                Err(WorldSpatialError::Malformed)
            );
        } else {
            assert_eq!(decode_step_intent(&payload), Ok(direction));
        }
    }
    for value in [0, 9, 127, 255] {
        let mut payload = vec![8];
        push_varint(&mut payload, value);
        assert_eq!(
            decode_step_intent(&payload),
            Err(WorldSpatialError::Malformed)
        );
    }
    assert_eq!(
        decode_step_intent(&[8, 5, 8, 6]),
        Err(WorldSpatialError::Malformed)
    );
    Ok(())
}

#[test]
fn m40_larger_pacing_receipts_require_both_capabilities() -> Result<(), String> {
    let result = StepResult {
        disposition: StepDisposition::Moved,
        step_wait_ms: MAX_DIAGONAL_STEP_WAIT_MS,
    };
    assert!(encode_step_result_paced(result, true).is_err());
    assert!(encode_step_result_with_diagonal(result, false, true).is_err());
    let bytes =
        encode_step_result_with_diagonal(result, true, true).map_err(|e| format!("{e:?}"))?;
    assert!(decode_step_result_receipt(&bytes, true).is_err());
    assert_eq!(
        decode_step_result_receipt_with_diagonal(&bytes, true, true),
        Ok(result)
    );
    assert!(
        encode_step_result_with_diagonal(
            StepResult {
                step_wait_ms: MAX_DIAGONAL_STEP_WAIT_MS + 50,
                ..result
            },
            true,
            true
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn m40_step_extension_does_not_add_diagonal_turns() {
    use crate::target_move::{
        TargetMoveIntent, decode_target_move_intent, encode_target_move_intent,
    };
    for direction in [
        StepDirection::NorthEast,
        StepDirection::SouthEast,
        StepDirection::SouthWest,
        StepDirection::NorthWest,
    ] {
        assert!(encode_target_move_intent(TargetMoveIntent::Turn(direction)).is_err());
        assert!(decode_target_move_intent(&[26, 2, 8, direction as u8]).is_err());
    }
}

#[test]
fn m40_actor_facing_remains_cardinal() -> Result<(), String> {
    use crate::world_spatial_entities::{
        EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesSnapshot, WorldSpatialEntity,
        encode_world_spatial_entities_snapshot,
    };
    let mut snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        own_identity: [1; 16],
        entities: vec![WorldSpatialEntity {
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::East,
                appearance_ref: 1,
                health_percent: 100,
            },
        }],
    };
    encode_world_spatial_entities_snapshot(&snapshot).map_err(|e| format!("valid actor: {e:?}"))?;
    snapshot.entities[0].detail = EntityDetail::Actor {
        direction: StepDirection::NorthEast,
        appearance_ref: 1,
        health_percent: 100,
    };
    assert!(encode_world_spatial_entities_snapshot(&snapshot).is_err());
    Ok(())
}
