//! C06 replay owner: `snapshot_state.rs::SnapshotState::decode` consumes inventory/vitals
//! snapshots, while `domain_dispatch.rs::Session::apply_state_delta` consumes their deltas.
//! The visible accessors are `session_access.rs::Session::{inventory,actor_vitals}`.
use crate::snapshot_state::SnapshotState;
use oteryn_protocol_oteryn::{DomainSnapshot, actor_spell, item_view};

type TestError = Box<dyn std::error::Error>;

#[test]
fn c06_real_payloads_replay_into_client_inventory_and_vitals() -> Result<(), TestError> {
    let (inventory_payload, vitals_payload, expected) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c06-inventory.bin"))?,
                std::fs::read(path.join("c06-vitals.bin"))?,
                std::fs::read_to_string(path.join("c06-expected.txt"))?,
            )
        } else {
            let inventory = item_view::CharacterInventory::default();
            let vitals = actor_spell::ActorVitals {
                health: 145,
                max_health: 150,
                mana: 30,
                max_mana: 55,
                soul: 7,
                harmony: 0,
                serene: false,
            };
            (
                item_view::encode_character_inventory(&inventory)
                    .map_err(|error| format!("{error:?}"))?,
                actor_spell::encode_actor_vitals(&vitals).map_err(|error| format!("{error:?}"))?,
                "145 150 30 55 7 0\n".into(),
            )
        };
    let body = snapshot_body(&inventory_payload, &vitals_payload)?;
    let state = SnapshotState::decode(&body, 41, &[4, 12], None, None)?;
    let values = expected
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let [health, max_health, mana, max_mana, soul, entries] = values.as_slice() else {
        return Err("invalid C06 expected values".into());
    };
    let vitals = state.actor_vitals.ok_or("C06 vitals not applied")?;
    let inventory = state
        .inventory
        .as_ref()
        .ok_or("C06 inventory not applied")?;
    assert_eq!(
        (
            vitals.health,
            vitals.max_health,
            vitals.mana,
            vitals.max_mana,
            vitals.soul
        ),
        (*health, *max_health, *mana, *max_mana, *soul)
    );
    assert_eq!(u32::try_from(inventory.entries.len())?, *entries);

    let retained = state.clone();
    let mut invalid = vitals_payload;
    invalid.push(0);
    let invalid_body = snapshot_body(&inventory_payload, &invalid)?;
    assert!(SnapshotState::decode(&invalid_body, 42, &[4, 12], None, Some(&state)).is_err());
    assert_eq!(retained.actor_vitals, state.actor_vitals);
    assert_eq!(retained.inventory, state.inventory);
    Ok(())
}

fn snapshot_body(inventory: &[u8], vitals: &[u8]) -> Result<Vec<u8>, TestError> {
    let spatial = oteryn_protocol_oteryn::world_spatial::encode_world_spatial(
        &oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
            content_generation: [7; 32],
            actor_position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 1,
                y: 2,
                floor: -7,
            },
        },
    );
    let frames = oteryn_protocol_oteryn::encode_single_chunk_snapshot(
        1,
        1,
        41,
        &[
            DomainSnapshot {
                domain_id:
                    oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type:
                    oteryn_protocol_oteryn::world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                revision: 2,
                snapshot_type: item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload: inventory,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 3,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: vitals,
            },
        ],
    )?;
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frames[1])?;
    let (_, body) = oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())?;
    Ok(body.to_vec())
}
