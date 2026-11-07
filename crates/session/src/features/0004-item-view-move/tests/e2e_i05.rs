//! I05 replay for the real native session projection.
//!
//! Payload ownership is `crate::domain_dispatch::Session::apply_delta`, in its
//! `STATE_DOMAIN_CHARACTER_INVENTORY` arm; the visible accessor is
//! `crate::session_access::Session::inventory`. The UI/runtime feature layers
//! only project that already-applied session inventory.
use super::*;
use oteryn_protocol_oteryn::item_view;

#[test]
fn i05_equipment_transform_replays_and_malformed_delta_preserves_state() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(replay()))?
        .join()
        .map_err(|_| "i05 replay thread failed")??
}

async fn replay() -> Result<(), BoxError> {
    let (payload, expected) = fixture()?;
    let numbers = expected
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [
        definition,
        count,
        base_revision,
        new_revision,
        server_sequence,
    ] = numbers.as_slice()
    else {
        return Err("invalid i05 expected metadata".into());
    };
    let (definition, count, base_revision, new_revision, server_sequence) = (
        *definition,
        *count,
        *base_revision,
        *new_revision,
        *server_sequence,
    );
    let initial = CharacterInventory::default();
    let initial_payload = item_view::encode_character_inventory_with_equip_drop(&initial, true)
        .map_err(|error| format!("{error:?}"))?;
    let selected = &[4, 6, 12, 14];
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let captured = payload.clone();
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            selected,
            selected,
            &[],
            item_snapshot(vec![])?,
            &[(9, base_revision, 1, initial_payload), (14, 1, 1, vec![])],
        )
        .await?;
        write_frame(
            &mut server,
            &encode_state_delta(
                1,
                server_sequence,
                9,
                base_revision,
                new_revision,
                1,
                &captured,
            )?,
        )
        .await?;
        let mut rest = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut server, &mut rest).await?;
        Ok::<(), BoxError>(())
    });
    let mut admission = item_admission()?;
    admission.supported_capabilities = selected;
    let mut session = Session::admit(client, admission).await?;
    session.service_liveness(Duration::from_millis(20)).await?;
    let ring = session
        .inventory()
        .and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|row| row.slot == item_view::EquipmentSlot::Ring)
        })
        .ok_or("i05 replay ring slot empty")?;
    assert_eq!(u64::from(ring.item.item_definition_ref.get()), definition);
    assert_eq!(u64::from(ring.item.count.get()), count);
    let before = session.inventory().cloned();
    let mut malformed = payload;
    malformed.pop();
    assert!(item_view::decode_character_inventory_with_equip_drop(&malformed, true).is_err());
    assert_eq!(session.inventory().cloned(), before);
    drop(session);
    peer.await??;
    Ok(())
}

fn fixture() -> Result<(Vec<u8>, String), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok((
            std::fs::read(path.join("i05-inventory.bin"))?,
            std::fs::read_to_string(path.join("i05-expected.txt"))?,
        ));
    }
    let inventory = CharacterInventory {
        equipment: vec![item_view::EquippedItem {
            slot: item_view::EquipmentSlot::Ring,
            item: entry(77, 3088, 1),
        }],
        ..Default::default()
    };
    Ok((
        item_view::encode_character_inventory_with_equip_drop(&inventory, true)
            .map_err(|error| format!("{error:?}"))?,
        "3088 1 1 2 41\n".into(),
    ))
}
