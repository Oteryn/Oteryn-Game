//! C19a client replay: the first-entry starter backpack arrives in the CharacterInventory join
//! snapshot, consumed by `SnapshotState::decode` (`snapshot_state.rs`) and exposed by
//! `Session::inventory` (`session_access.rs`).
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::item_view;

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
];

/// Starter backpack row as the client shows it, plus every carried entry line.
fn visible(inventory: Option<&CharacterInventory>) -> String {
    inventory.map_or_else(String::new, |inventory| {
        let mut text = inventory.main_backpack.map_or_else(String::new, |item| {
            format!(
                "backpack={} {} {}\n",
                item.handle, item.item_definition_ref, item.count
            )
        });
        for item in &inventory.entries {
            text.push_str(&format!(
                "entry={} {} {}\n",
                item.handle, item.item_definition_ref, item.count
            ));
        }
        text
    })
}

fn fixture() -> Result<(Vec<u8>, u64, String), BoxError> {
    let backpack = ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::new(3).unwrap_or(ItemHandle::MIN),
        item_definition_ref: std::num::NonZeroU32::new(2854).unwrap_or(std::num::NonZeroU32::MIN),
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let inventory = CharacterInventory {
        main_backpack: Some(backpack),
        ..CharacterInventory::default()
    };
    Ok((
        item_view::encode_character_inventory(&inventory).map_err(|error| format!("{error:?}"))?,
        1,
        visible(Some(&inventory)),
    ))
}

#[test]
fn e2e_c19a_starter_backpack_replay() -> Result<(), BoxError> {
    crate::tests::block_on(async {
        let (snapshot, revision, expected) = fixture()?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let snapshot_for_peer = snapshot.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                entity_snapshot()?,
                &[(
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    revision,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    snapshot_for_peer,
                )],
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
                return Err(format!("C19a admission: {error:?}; peer={peer_error:?}").into());
            }
        };
        let captured = session
            .gated_snapshot(STATE_DOMAIN_CHARACTER_INVENTORY)
            .ok_or("inventory join snapshot absent")?;
        assert_eq!(captured.revision, revision);
        assert_eq!(captured.payload, snapshot);
        let inventory = session.inventory().ok_or("inventory not decoded")?;
        assert!(inventory.main_backpack.is_some());
        assert_eq!(visible(Some(inventory)), expected);

        let sequence = session.last_server_sequence();
        assert_eq!(
            session.verify_inventory_replay(&snapshot, revision, sequence + 1),
            Err("inventory replay capture does not match the current session")
        );
        session.verify_inventory_replay(&snapshot, revision, sequence)?;
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
        motion: None,
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
