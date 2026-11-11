//! C19a client replay: the first-entry starter backpack arrives in the CharacterInventory join
//! snapshot, consumed by `SnapshotState::decode` (`snapshot_state.rs`) and exposed by
//! `Session::inventory` (`session_access.rs`).
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    item_view::CAPABILITY_ITEM_EQUIP_DROP_V1,
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
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("c19a-expected.txt"))?;
        let revision = expected
            .lines()
            .find_map(|line| line.strip_prefix("revision="))
            .ok_or("c19a expected lacks revision")?
            .parse()?;
        let body = expected
            .lines()
            .filter(|line| !line.starts_with("revision="))
            .map(|line| format!("{line}\n"))
            .collect();
        return Ok((
            std::fs::read(path.join("c19a-snapshot.bin"))?,
            revision,
            body,
        ));
    }
    let backpack = ItemEntry {
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
        let inventory = session.inventory().ok_or("inventory not decoded")?;
        assert!(inventory.main_backpack.is_some());
        assert_eq!(visible(Some(inventory)), expected);

        // A malformed delta is rejected and leaves the starter inventory unchanged.
        let before = session.inventory().cloned();
        let malformed = encode_state_delta(
            1,
            1,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            revision,
            revision + 1,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &[0xff],
        )?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(session.apply_state_delta(1, envelope.payload()).is_err());
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
