use super::*;

#[test]
fn gap_0607_quick_loot_destinations_and_distribution() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().map_err(|e| e.to_string())?;
            runtime
                .block_on(async {
                    use oteryn_session::{Admission, CharacterId, Session};
                    use std::time::Duration;

                    let (client, server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(quick_loot_destinations_peer(server));
                    let session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&quick_loot_id(4))?,
                            admission_material: b"quick-loot-destinations",
                            client_build_id: "quick-loot-native-test",
                            supported_capabilities: QUICK_LOOT_CAPS,
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let (play_link, commands, outbox) = play_channel();
                    let task = tokio::spawn(crate::play::run_session(session, commands, outbox));
                    let mut app = app_with_link(play_link)?;
                    app.add_plugins(bevy::ui_widgets::ButtonPlugin);
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while link(&app)?.quick_loot_panel().filter_mode != Some(Mode::Accept) {
                            tokio::task::yield_now().await;
                        }
                        Ok::<(), Box<dyn std::error::Error>>(())
                    })
                    .await??;
                    open(&mut app)?;
                    wait_for_text(&mut app, "Confirmed: only listed items").await?;
                    click_caption(&mut app, "Refresh destinations")?;
                    wait_for_text(&mut app, "Obtain: Main backpack").await?;
                    click_caption(&mut app, "Distribute backpack contents")?;
                    wait_for_text(&mut app, "Distributed 1 of 2 items").await?;
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while !link(&app)?.inventory_panel().inventory.is_some_and(|inventory| {
                            matches!(inventory.entries.as_slice(), [item] if item.handle.get() == 7)
                        }) {
                            tokio::task::yield_now().await;
                        }
                        Ok::<(), Box<dyn std::error::Error>>(())
                    })
                    .await??;
                    drop(app);
                    tokio::time::timeout(Duration::from_secs(5), task).await??;
                    tokio::time::timeout(Duration::from_secs(5), peer)
                        .await??
                        .map_err(|error| error.to_string())?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "quick loot native test thread failed")??;
    Ok(())
}

fn click_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or_else(|| format!("missing quick loot control {caption}"))?;
    click(app, entity);
    Ok(())
}

async fn wait_for_text(app: &mut App, wanted: &str) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            app.update();
            if app
                .world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == wanted)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| format!("quick loot panel did not show {wanted}"))?;
    Ok(())
}

const QUICK_LOOT_CAPS: &[u32] = &[4, 6, 98, 607];

fn quick_loot_id(marker: u8) -> [u8; 16] {
    let mut id = [marker; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

async fn quick_loot_destinations_peer(
    mut peer: tokio::io::DuplexStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use oteryn_session::{read_frame, test_support::wire, write_frame};
    use wire::{item_view::*, quick_loot::*, world_object, world_spatial_entities::*};
    read_frame(&mut peer).await?;
    write_frame(
        &mut peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&quick_loot_id(1))?,
            world_id: wire::WorldId::decode(&quick_loot_id(2))?,
            channel_id: wire::ChannelId::decode(&quick_loot_id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: QUICK_LOOT_CAPS,
        })?,
    )
    .await?;
    let position = wire::world_spatial::ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let own = EntityRef {
        identity: quick_loot_id(4),
        generation: 1,
    };
    let spatial =
        encode_world_spatial_entities_snapshot_with_item_handles(&WorldSpatialEntitiesSnapshot {
            content_generation: [1; 32],
            actor_position: position,
            own_identity: own.identity,
            entities: vec![WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: EntityKind::Player,
                entity: own,
                position,
                detail: EntityDetail::Actor {
                    direction: wire::world_spatial::StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            }],
        })
        .map_err(|error| format!("{error:?}"))?;
    let overlay = world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    let item = |handle: u64, definition: u32| -> Result<ItemEntry, std::num::TryFromIntError> {
        Ok(ItemEntry {
            appearance_id: 0,
            handle: handle.try_into()?,
            item_definition_ref: definition.try_into()?,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        })
    };
    let mut inventory = CharacterInventory {
        main_backpack: Some(item(5, 2854)?),
        entries: vec![item(6, 3031)?, item(7, 3035)?],
        equipment: vec![],
    };
    let initial = encode_character_inventory(&inventory).map_err(|error| format!("{error:?}"))?;
    for frame in wire::encode_single_chunk_snapshot(
        1,
        1,
        1,
        &[
            wire::DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                payload: &spatial,
            },
            wire::DomainSnapshot {
                domain_id: world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 1,
                snapshot_type: world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            wire::DomainSnapshot {
                domain_id: STATE_DOMAIN_CHARACTER_INVENTORY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload: &initial,
            },
        ],
    )? {
        write_frame(&mut peer, &frame).await?;
    }
    let main = QuickLootDestination {
        category: None,
        recipient: Recipient::MainContainer,
        recipient_id: None,
    };
    let mut saved = QuickLootDestinations {
        loot: vec![main],
        obtain: vec![],
        fallback_to_main: true,
        filters: Some(QuickLootFilters {
            mode: Mode::Accept,
            skipped: vec![],
            accepted: vec![3031],
        }),
    };
    let startup = encode_quick_loot_destinations(&saved).map_err(|error| format!("{error:?}"))?;
    saved.obtain.push(QuickLootDestination {
        category: Some(Category::Gold),
        ..main
    });
    let refreshed = encode_quick_loot_destinations(&saved).map_err(|error| format!("{error:?}"))?;
    let distributed = encode_quick_loot_result(&QuickLootResult {
        items: vec![
            QuickLootItemResult {
                item: quick_loot_id(6),
                accepted: true,
            },
            QuickLootItemResult {
                item: quick_loot_id(7),
                accepted: false,
            },
        ],
    })
    .map_err(|error| format!("{error:?}"))?;
    for (id, kind, request, reply) in [
        (1, 6376, &[8, 9, 80, 1][..], startup),
        (2, 6376, &[8, 9, 80, 1][..], refreshed),
        (
            3,
            6377,
            &[8, 14, 18, 8, 5, 0, 0, 0, 0, 0, 0, 0][..],
            distributed,
        ),
    ] {
        let frame = read_frame(&mut peer).await?;
        let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(
            (command.command_id, command.command_type, command.payload),
            (id, kind, request)
        );
        write_frame(
            &mut peer,
            &wire::encode_command_result(1, id + 1, id, CommandStatus::Accepted, &reply)?,
        )
        .await?;
    }
    inventory.entries.remove(0);
    let updated = encode_character_inventory(&inventory).map_err(|error| format!("{error:?}"))?;
    write_frame(
        &mut peer,
        &wire::encode_state_delta(
            1,
            5,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            1,
            2,
            DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &updated,
        )?,
    )
    .await?;
    let mut byte = [0];
    assert_eq!(
        tokio::io::AsyncReadExt::read(&mut peer, &mut byte).await?,
        0
    );
    Ok(())
}
