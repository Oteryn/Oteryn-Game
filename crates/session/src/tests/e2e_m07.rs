//! Synthetic reducer regression tests for the M07 spatial delta.
//! The live row replays its own wire frames inside the qualification harness.
//!
//! The production consumer is `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; its capability-6 branch calls
//! `decode_world_spatial_entities_delta_with_icons`, updates
//! `Session::world_spatial.actor_position`, and exposes it through
//! `Session::world_spatial` in `crates/session/src/session_access.rs`.
use super::*;

#[test]
fn m07_rope_ascent_spatial_reducer_regression() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(replay)?
        .join()
        .map_err(|_| "M07 replay thread panicked")?
}

fn replay() -> Result<(), BoxError> {
    block_on(async {
        use oteryn_protocol_oteryn::world_spatial_entities::*;
        let after = ActorPosition {
            x: 11,
            y: 19,
            floor: 7,
        };
        let delta = WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: after,
            enter: vec![],
            update: vec![own(after)],
            leave: vec![],
        };
        let payload = encode_world_spatial_entities_delta_with_item_handles(&delta)
            .map_err(|error| format!("{error:?}"))?;
        let metadata = "11 19 8\n11 19 7\n42 5 6 41\n";
        let values = metadata
            .split_whitespace()
            .map(str::parse::<i64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [
            before_x,
            before_y,
            before_floor,
            after_x,
            after_y,
            after_floor,
            sequence,
            base,
            new,
            _command_sequence,
        ] = values.as_slice()
        else {
            return Err("invalid M07 expected metadata".into());
        };
        let decoded = decode_world_spatial_entities_delta_with_item_handles(&payload)
            .map_err(|error| format!("{error:?}"))?;
        let before = ActorPosition {
            x: i32::try_from(*before_x)?,
            y: i32::try_from(*before_y)?,
            floor: i16::try_from(*before_floor)?,
        };
        let own = own(before);
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: decoded.content_generation,
            actor_position: before,
            own_identity: OWN,
            entities: vec![own],
        };
        let snapshot_payload = encode_world_spatial_entities_snapshot_with_item_handles(&snapshot)
            .map_err(|error| format!("{error:?}"))?;
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, snapshot_payload),
            &[4, 6],
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert_eq!(session.world_spatial().actor_position, before);
        let frame = encode_state_delta(
            1,
            u64::try_from(*sequence)?,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            u64::try_from(*base)?,
            u64::try_from(*new)?,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &payload,
        )?;
        let use_result = world_object::encode_use_result(UseDisposition::Committed)
            .map_err(|error| format!("{error:?}"))?;
        let receipt = encode_command_result(1, 41, 7, CommandStatus::Accepted, &use_result)?;
        let checkpoint = session.begin_spatial_capture()?;
        session.record_spatial_capture(&receipt, &decode_wire_envelope(&receipt)?);
        session.record_spatial_capture(&frame, &decode_wire_envelope(&frame)?);
        let captured = session
            .finish_spatial_capture()
            .ok_or("M07 capture missing")?;
        checkpoint
            .verify_move(&captured, decoded.actor_position, 7, 41, 42)
            .map_err(|error| error.to_string())?;
        let checkpoint = session.begin_spatial_capture()?;
        assert!(
            checkpoint
                .verify_move(&captured, decoded.actor_position, 8, 41, 42)
                .is_err()
        );
        let checkpoint = session.begin_spatial_capture()?;
        assert!(
            checkpoint
                .verify_move(
                    &captured[..captured.len() - 1],
                    decoded.actor_position,
                    7,
                    41,
                    42
                )
                .is_err()
        );
        session.apply_state_delta(
            u64::try_from(*sequence)?,
            decode_wire_envelope(&frame)?.payload(),
        )?;
        assert_eq!(
            session.world_spatial().actor_position,
            ActorPosition {
                x: i32::try_from(*after_x)?,
                y: i32::try_from(*after_y)?,
                floor: i16::try_from(*after_floor)?
            }
        );
        let unchanged = session.world_spatial().actor_position;
        let malformed = encode_state_delta(
            1,
            u64::try_from(*sequence + 1)?,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            u64::try_from(*new)?,
            u64::try_from(*new + 1)?,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(
                    u64::try_from(*sequence + 1)?,
                    decode_wire_envelope(&malformed)?.payload()
                )
                .is_err()
        );
        assert_eq!(session.world_spatial().actor_position, unchanged);
        drop(session);
        peer.await??;
        Ok(())
    })?
}
