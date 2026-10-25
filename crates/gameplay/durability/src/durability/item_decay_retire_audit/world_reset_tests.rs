#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;

fn reset_message(entry: bool) -> OneItemDecayRetireV1 {
    let mut message = if entry {
        super::super::tests::world_reset_entry_fixture()
    } else {
        super::super::tests::world_reset_root_fixture()
    };
    let before = message.before.as_ref().unwrap();
    let target_digest = vec![7; 32];
    message.world_reset = Some(OneItemWorldResetV1 {
        world_id: before.world_id.clone(),
        reset_epoch: 0,
        item_instance_id: before.item_instance_id.clone(),
        retirement_binding: Sha256::new()
            .chain_update(b"oteryn:world-reset-retirement:v1")
            .chain_update(&target_digest)
            .finalize()
            .to_vec(),
        target_digest,
    });
    message.cause = None;
    message.ground_container_entry =
        message
            .corpse_entry
            .take()
            .map(|entry| OneItemGroundContainerEntryV1 {
                parent_item_instance_id: entry.corpse_item_instance_id,
                placement_ordinal: entry.placement_ordinal,
                parent_ground: entry.corpse_ground,
            });
    message
}

#[test]
fn world_reset_round_trips_both_locations_without_a_decay_deadline() {
    for (entry, shape) in [
        (false, DecayRetireShape::Ground),
        (true, DecayRetireShape::GroundContainerEntry),
    ] {
        let message = reset_message(entry);
        assert_eq!(check_decay_retire(&message), Ok(shape));
        let wire = encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id: super::super::tests::world_reset_uuid_fixture(30),
                transaction_id: super::super::tests::world_reset_uuid_fixture(40),
                occurred_at_unix_ms: 1,
                server_build_id: "oteryn-game-server/test",
            },
            message.clone(),
            mint::Type2EventTuple::V1,
        )
        .unwrap();
        assert_eq!(decode_decay_retire_envelope(&wire).unwrap().1, message);
    }
}

#[test]
fn world_reset_rejects_mixed_causes_locations_and_substituted_binding() {
    let mutations: [fn(&mut OneItemDecayRetireV1); 11] = [
        |message| {
            message.before.as_mut().unwrap().key_number = 65_536;
            message.after.as_mut().unwrap().key_number = 65_536;
        },
        |message| {
            message.before.as_mut().unwrap().key_number = 1;
            message.after.as_mut().unwrap().key_number = 1;
        },
        |message| message.cause = super::super::tests::world_reset_entry_fixture().cause,
        |message| {
            message.corpse_entry = super::super::tests::world_reset_entry_fixture().corpse_entry
        },
        |message| message.world_reset.as_mut().unwrap().reset_epoch = u64::MAX,
        |message| message.world_reset.as_mut().unwrap().item_instance_id[0] ^= 1,
        |message| message.world_reset.as_mut().unwrap().world_id[0] ^= 1,
        |message| message.world_reset.as_mut().unwrap().target_digest[0] ^= 1,
        |message| message.world_reset.as_mut().unwrap().retirement_binding[0] ^= 1,
        |message| {
            message
                .ground_container_entry
                .as_mut()
                .unwrap()
                .placement_ordinal = 17
        },
        |message| {
            message
                .ground_container_entry
                .as_mut()
                .unwrap()
                .parent_ground = None
        },
    ];
    for mutate in mutations {
        let mut message = reset_message(true);
        mutate(&mut message);
        assert_eq!(check_decay_retire(&message), Err(AuditError::InvalidInput));
    }
}

#[test]
fn world_reset_largest_admitted_audit_fits_the_registered_envelope() {
    for (entry, key) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut message = reset_message(entry);
        for state in [&mut message.before, &mut message.after] {
            let definition = state.as_mut().unwrap().definition.as_mut().unwrap();
            definition.family = "f".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
            definition.production_key = "k".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
            definition.revision_ref = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        }
        message.before.as_mut().unwrap().quantity = if key { 1 } else { u32::MAX };
        if key {
            message.before.as_mut().unwrap().key_number = mint::WORLDINT0_KEY_NUMBER_MAX;
            message.after.as_mut().unwrap().key_number = mint::WORLDINT0_KEY_NUMBER_MAX;
        }
        message.world_reset.as_mut().unwrap().reset_epoch = u64::MAX - 1;
        message.runtime_scope_ownership_generation = u64::MAX;
        let ground = if entry {
            let entry = message.ground_container_entry.as_mut().unwrap();
            entry.placement_ordinal = 16;
            entry.parent_ground.as_mut().unwrap()
        } else {
            message.ground.as_mut().unwrap()
        };
        ground.spatial_position = vec![1; mint::RL07_TECHNICAL_FIELD_BYTES_MAX];
        ground.corpse_ref = vec![2; mint::RL07_TECHNICAL_FIELD_BYTES_MAX];
        ground.native_room_placement_context = vec![3; mint::RL07_TECHNICAL_FIELD_BYTES_MAX];
        ground.map_revision = "m".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        ground.content_revision = "c".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        ground.runtime_scope_ownership_generation = u64::MAX;
        let build = "b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
        let wire = encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id: super::super::tests::world_reset_uuid_fixture(30),
                transaction_id: super::super::tests::world_reset_uuid_fixture(40),
                occurred_at_unix_ms: i64::MAX,
                server_build_id: &build,
            },
            message,
            mint::Type2EventTuple::V2,
        )
        .unwrap();
        let (envelope, _) = decode_decay_retire_envelope(&wire).unwrap();
        assert!(envelope.payload.len() <= mint::RL07_PAYLOAD_BYTES_MAX);
        assert!(wire.len() <= mint::RL07_ENVELOPE_BYTES_MAX);
        eprintln!(
            "WORLD-RESET-AUDIT: entry={entry} key={key} payload={} envelope={}",
            envelope.payload.len(),
            wire.len()
        );
    }
}
