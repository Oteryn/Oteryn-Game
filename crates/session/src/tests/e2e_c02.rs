//! C02 replay coverage for the native session inventory reducer.
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
                let (payload, expected) =
                    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                        let root = std::path::Path::new(&root);
                        (
                            std::fs::read(root.join("c02-inventory.bin"))?,
                            std::fs::read_to_string(root.join("c02-expected.txt"))?,
                        )
                    } else {
                        (
                            item_view::encode_character_inventory_with_equip_drop(&inline, true)
                                .map_err(|error| format!("{error:?}"))?,
                            "3447 1\n".into(),
                        )
                    };
                let selected = capture_inventory_profile::selected("c02", &[4, 6, 12])?;
                // Only the after view is captured. The synthetic join cannot invent unselected equipment.
                let initial = if selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1) {
                    initial
                } else {
                    CharacterInventory::default()
                };
                let expected_inventory = capture_inventory_profile::decode(&payload, &selected)?;
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let initial_payload = item_view::encode_character_inventory_with_appearances(
                    &initial,
                    selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
                    selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
                )
                .map_err(|error| format!("{error:?}"))?;
                let selected_for_peer = selected.clone();
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &selected_for_peer,
                        &selected_for_peer,
                        &[],
                        item_snapshot(vec![])?,
                        &[(9, 4, 1, initial_payload)],
                    )
                    .await?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(
                            1,
                            41,
                            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                            4,
                            5,
                            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                            &payload,
                        )?,
                    )
                    .await?;
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
                let admission = Admission {
                    supported_capabilities: &selected,
                    ..admission_with(&[4, 6, 12])?
                };
                let mut session = Session::admit(client, admission).await?;
                assert_eq!(session.inventory(), Some(&initial));
                session.wait_for_events(DEADLINE).await?;
                assert_eq!(session.inventory(), Some(&expected_inventory));
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
