//! I11 client replay: food crafting changes domain 9, consumed by
//! `SnapshotState::decode` (`snapshot_state.rs`) and `Session::apply_state_delta`
//! (`domain_dispatch.rs`), then exposed by `Session::inventory` (`session_access.rs`).
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
];

#[test]
fn e2e_i11_inventory_replay() -> Result<(), BoxError> {
    crate::tests::block_on(async {
        let (snapshot, delta, metadata, expected) = fixture()?;
        let numbers = metadata
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [snapshot_revision, base_revision, new_revision, sequence] = numbers.as_slice() else {
            return Err("invalid I11 capture metadata".into());
        };
        let (snapshot_revision, base_revision, new_revision, sequence) =
            (*snapshot_revision, *base_revision, *new_revision, *sequence);
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let snapshot_for_peer = snapshot.clone();
        let delta_for_peer = delta.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                entity_snapshot()?,
                &[(
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    snapshot_revision,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    snapshot_for_peer,
                )],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    sequence,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    base_revision,
                    new_revision,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &delta_for_peer,
                )?,
            )
            .await?;
            std::future::pending::<()>().await;
            Ok::<(), BoxError>(())
        });
        let mut session = match Session::admit(client, admission_with(SELECTED)?).await {
            Ok(session) => session,
            Err(error) => {
                peer.abort();
                let peer_error = peer.await;
                return Err(format!("I11 admission: {error:?}; peer={peer_error:?}").into());
            }
        };
        session
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| format!("I11 delta: {error:?}"))?;
        assert_eq!(visible(session.inventory()), expected);

        let before = session.inventory().cloned();
        let malformed = encode_state_delta(
            1,
            sequence + 1,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            new_revision,
            new_revision + 1,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &[0xff],
        )?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(
            session
                .apply_state_delta(sequence + 1, envelope.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), before.as_ref());
        peer.abort();
        assert!(peer.await.is_err());
        Ok(())
    })?
}

fn entity_snapshot() -> Result<(u32, Vec<u8>), BoxError> {
    let position = ActorPosition {
        x: 0,
        y: 0,
        floor: 0,
    };
    let own_identity = [0xa0; 16];
    let own = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: own_identity,
            generation: 1,
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let payload = world_spatial_entities::encode_world_spatial_entities_snapshot_with_item_handles(
        &world_spatial_entities::WorldSpatialEntitiesSnapshot {
            content_generation: [0x11; 32],
            actor_position: position,
            own_identity,
            entities: vec![own],
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        payload,
    ))
}

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String, String), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok((
            std::fs::read(path.join("i11-snapshot.bin"))?,
            std::fs::read(path.join("i11-inventory.bin"))?,
            std::fs::read_to_string(path.join("i11-revisions.txt"))?,
            std::fs::read_to_string(path.join("i11-expected.txt"))?,
        ));
    }
    let item = |handle, definition| ItemEntry {
        handle: ItemHandle::new(handle).unwrap_or(ItemHandle::MIN),
        item_definition_ref: std::num::NonZeroU32::new(definition)
            .unwrap_or(std::num::NonZeroU32::MIN),
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let dough = item(1, 3604);
    let bread = item(2, 3600);
    let backpack = item(3, 2854);
    let before = CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![dough],
        ..CharacterInventory::default()
    };
    let after = CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![bread],
        ..CharacterInventory::default()
    };
    Ok((
        item_view::encode_character_inventory(&before).map_err(|error| format!("{error:?}"))?,
        item_view::encode_character_inventory(&after).map_err(|error| format!("{error:?}"))?,
        "1 1 2 41".into(),
        visible(Some(&after)),
    ))
}

fn visible(inventory: Option<&CharacterInventory>) -> String {
    inventory.map_or_else(String::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
            .collect()
    })
}
