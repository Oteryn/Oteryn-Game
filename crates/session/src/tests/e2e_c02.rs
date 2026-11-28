//! Unit coverage for the native session inventory reducer and original-frame retention.
//!
//! The owning client code is `Session::dispatch_delta` in
//! `crates/session/src/domain_dispatch.rs` (the `STATE_DOMAIN_CHARACTER_INVENTORY` arm), which
//! decodes and installs ammunition changes, and `Session::inventory` in
//! `crates/session/src/session_access.rs`, which exposes the resulting client-visible state.

use super::*;

fn entry(handle: u64, definition: u32, count: u32) -> ItemEntry {
    ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::new(handle).unwrap_or(ItemHandle::MIN),
        item_definition_ref: std::num::NonZeroU32::new(definition)
            .unwrap_or(std::num::NonZeroU32::MIN),
        count: std::num::NonZeroU32::new(count).unwrap_or(std::num::NonZeroU32::MIN),
        sub_type: 0,
    }
}

#[test]
fn c02_replays_server_ammunition_payload_and_rejects_invalid_delta() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let initial = CharacterInventory {
                    equipment: vec![item_view::EquippedItem {
                        slot: item_view::EquipmentSlot::Ammo,
                        item: entry(71, 3447, 2),
                    }],
                    ..CharacterInventory::default()
                };
                let inline = CharacterInventory {
                    equipment: vec![item_view::EquippedItem {
                        slot: item_view::EquipmentSlot::Ammo,
                        item: entry(71, 3447, 1),
                    }],
                    ..CharacterInventory::default()
                };
                let payload = item_view::encode_character_inventory_with_equip_drop(&inline, true)
                    .map_err(|error| format!("{error:?}"))?;
                let expected = "3447 1\n";
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let initial_payload =
                    item_view::encode_character_inventory_with_equip_drop(&initial, true)
                        .map_err(|error| format!("{error:?}"))?;
                let expected_wire = encode_state_delta(
                    1,
                    41,
                    item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                    4,
                    5,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &payload,
                )?;
                let wire = expected_wire.clone();
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[4, 6, 12],
                        &[4, 6, 12],
                        &[],
                        item_snapshot(vec![])?,
                        &[(9, 4, 1, initial_payload)],
                    )
                    .await?;
                    write_frame(&mut server, &wire).await?;
                    read_frame(&mut server).await?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(
                            1,
                            42,
                            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                            4,
                            6,
                            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                            &[0x80],
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[4, 6, 12])?).await?;
                session.enable_inventory_replay_capture();
                session.wait_for_events(DEADLINE).await?;
                let frame = session
                    .take_inventory_replay_frame()
                    .ok_or("wire frame absent")?;
                assert_eq!(frame, expected_wire);
                assert!(session.replay_inventory_frame(&frame, 42).is_err());
                let (base, revision, replayed) = session.replay_inventory_frame(&frame, 41)?;
                assert_eq!((base, revision), (4, 5));
                assert_eq!(Some(&replayed), session.inventory());
                let visible = session
                    .inventory()
                    .into_iter()
                    .flat_map(|inventory| &inventory.equipment)
                    .filter(|item| item.slot == item_view::EquipmentSlot::Ammo)
                    .map(|item| format!("{} {}\n", item.item.item_definition_ref, item.item.count))
                    .collect::<String>();
                assert_eq!(visible, expected);
                let unchanged = session.inventory().cloned();
                assert!(session.step(StepDirection::North).await.is_err());
                assert_eq!(session.inventory(), unchanged.as_ref());
                peer.await??;
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "C02 replay thread failed")??
}
