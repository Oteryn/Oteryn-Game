//! E02 client replay: NPC gold-to-platinum exchange changes the backpack coins. The owning client state is the CharacterInventory domain, decoded
//! by `SnapshotState::decode` and applied by `Session::apply_state_delta`
//! (`crates/session/src/domain_dispatch.rs`), read through `Session::inventory`.
//!
//! The before inventory and the applied after inventory are session-owned views of the same
//! character, re-encoded by the row with the production encoder (the C09/I11 practice).
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    item_view::CAPABILITY_ITEM_EQUIP_DROP_V1,
];

struct Capture {
    before: Vec<u8>,
    after: Vec<u8>,
    /// (definition reference, count) of every carried entry, before then after.
    before_units: Vec<(u32, u32)>,
    after_units: Vec<(u32, u32)>,
}

fn units(inventory: Option<&CharacterInventory>) -> Vec<(u32, u32)> {
    inventory.map_or_else(Vec::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| (item.item_definition_ref.get(), item.count.get()))
            .collect()
    })
}

fn parse(line: &str) -> Result<Vec<(u32, u32)>, BoxError> {
    line.split(';')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (reference, count) = pair.split_once(':').ok_or("E02 pair lacks ':'")?;
            Ok((reference.parse()?, count.parse()?))
        })
        .collect()
}

fn entry(handle: u64, reference: u32, count: u32) -> Result<ItemEntry, BoxError> {
    Ok(ItemEntry {
        handle: ItemHandle::new(handle).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(reference).ok_or("reference")?,
        count: std::num::NonZeroU32::new(count).ok_or("count")?,
        sub_type: 0,
    })
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("e02-expected.txt"))?;
        let value = |key: &str| -> Result<&str, BoxError> {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .ok_or_else(|| format!("e02 expected lacks {key}").into())
        };
        return Ok(Capture {
            before: std::fs::read(path.join("e02-before.bin"))?,
            after: std::fs::read(path.join("e02-after.bin"))?,
            before_units: parse(value("before=")?)?,
            after_units: parse(value("after=")?)?,
        });
    }
    let encode = |entries: Vec<ItemEntry>| -> Result<Vec<u8>, BoxError> {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(2, 2854, 1)?),
            entries,
            ..CharacterInventory::default()
        };
        item_view::encode_character_inventory(&inventory).map_err(|e| format!("{e:?}").into())
    };
    Ok(Capture {
        before: encode(vec![entry(3, 31, 100)?])?,
        after: encode(vec![entry(4, 35, 1)?])?,
        before_units: vec![(31, 100)],
        after_units: vec![(35, 1)],
    })
}

#[test]
fn e02_coin_inventory_replay() -> Result<(), BoxError> {
    crate::tests::block_on(async {
        let capture = fixture()?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let before = capture.before.clone();
        let after = capture.after.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                entity_snapshot()?,
                &[(
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    1,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    before,
                )],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    1,
                    2,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &after,
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
                return Err(format!("E02 admission: {error:?}; peer={peer_error:?}").into());
            }
        };
        assert_eq!(units(session.inventory()), capture.before_units);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while units(session.inventory()) != capture.after_units {
            if tokio::time::Instant::now() >= deadline {
                return Err("E02 coin delta never reached the inventory".into());
            }
            session.service_liveness(Duration::from_millis(50)).await?;
        }
        assert_eq!(units(session.inventory()), capture.after_units);
        assert_ne!(capture.before_units, capture.after_units);

        // A malformed inventory delta is rejected and leaves the converted coins unchanged.
        let kept = session.inventory().cloned();
        let malformed = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            2,
            3,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &[0xff],
        )?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(session.apply_state_delta(42, envelope.payload()).is_err());
        assert_eq!(session.inventory(), kept.as_ref());
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
