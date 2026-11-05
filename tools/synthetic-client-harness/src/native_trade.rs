//! C21: a scripted wire peer drives the production Session, task, picker and panel.
use oteryn_client::play::{PlayLink, PlayView, play_channel, run_session};
use oteryn_client_ui::player_trade::{Intent, panel_text};
use oteryn_protocol_oteryn::{
    self as wire, container_tree as containers, item_view, player_trade as trade, world_object,
    world_spatial, world_spatial_entities as entities,
};
use oteryn_renderer::TileCoord;
use oteryn_session::{Admission, Session, read_frame as read, write_frame as write};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};
type Error = Box<dyn std::error::Error + Send + Sync>;
const CAPS: &[u32] = &[4, 6, 12, 14, 60];
fn uuid(marker: u8) -> [u8; 16] {
    [
        1, 147, 79, 16, 124, 0, 112, 1, 128, 91, 59, 17, 34, 51, 68, marker,
    ]
}
fn partner() -> entities::EntityRef {
    entities::EntityRef {
        identity: [4; 16],
        generation: 2,
    }
}
fn actor() -> entities::WorldSpatialEntity {
    entities::WorldSpatialEntity {
        display_name: None,
        kind: entities::EntityKind::Player,
        entity: partner(),
        position: world_spatial::ActorPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
        detail: entities::EntityDetail::Actor {
            direction: world_spatial::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn encoded<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, Error> {
    result.map_err(|error| format!("{error:?}").into())
}
async fn peer(mut stream: DuplexStream) -> Result<(), Error> {
    read(&mut stream).await?;
    write(
        &mut stream,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&uuid(1))?,
            world_id: wire::WorldId::decode(&uuid(2))?,
            channel_id: wire::ChannelId::decode(&uuid(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: CAPS,
        })?,
    )
    .await?;
    let item = item_view::ItemEntry {
        handle: 70.try_into()?,
        item_definition_ref: 123.try_into()?,
        count: 2.try_into()?,
        sub_type: 0,
    };
    let inventory = encoded(item_view::encode_character_inventory_with_equip_drop(
        &item_view::CharacterInventory {
            main_backpack: Some(item_view::ItemEntry {
                handle: 71.try_into()?,
                item_definition_ref: 999.try_into()?,
                count: 1.try_into()?,
                sub_type: 0,
            }),
            entries: vec![item],
            equipment: vec![],
        },
        true,
    ))?;
    let own = entities::WorldSpatialEntity {
        entity: entities::EntityRef {
            identity: uuid(4),
            generation: 1,
        },
        position: world_spatial::ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        ..actor()
    };
    let spatial = encoded(
        entities::encode_world_spatial_entities_snapshot_with_item_handles(
            &entities::WorldSpatialEntitiesSnapshot {
                content_generation: [1; 32],
                actor_position: own.position,
                own_identity: own.entity.identity,
                entities: vec![own, actor()],
            },
        ),
    )?;
    let overlay = encoded(world_object::encode_world_object_overlay_snapshot(&[]))?;
    let initial = encoded(trade::encode_state(&Default::default()))?;
    let domains = [
        (1, 2, spatial),
        (2, 1, overlay),
        (9, 1, inventory),
        (11, 1, vec![]),
        (26, 1, initial),
        (14, 1, vec![]),
    ];
    let domains: Vec<_> = domains
        .iter()
        .map(|(id, kind, payload)| wire::DomainSnapshot {
            domain_id: *id,
            revision: 1,
            snapshot_type: *kind,
            payload,
        })
        .collect();
    for frame in wire::encode_single_chunk_snapshot(1, 1, 40, &domains)? {
        write(&mut stream, &frame).await?;
    }
    let bytes = read(&mut stream).await?;
    let envelope = wire::decode_wire_envelope(&bytes)?;
    let command = envelope.client_command(1)?;
    assert_eq!(command.command_type, 21);
    assert_eq!(
        encoded(containers::decode_container_view_intent(command.payload))?,
        containers::ContainerViewIntent::Open {
            handle: 71.try_into()?,
            replace_view: None
        }
    );
    let outcome =
        containers::encode_container_view_result(containers::ContainerViewOutcome::Opened);
    write(
        &mut stream,
        &wire::encode_command_result(
            1,
            41,
            command.command_id,
            wire::CommandStatus::Accepted,
            &outcome,
        )?,
    )
    .await?;
    let views = containers::ContainerViews {
        views: vec![containers::ContainerView {
            view_id: 0,
            container_handle: 71.try_into()?,
            parent_handle: None,
            capacity: 20,
            entries: vec![item_view::ItemEntry {
                handle: 90.try_into()?,
                item_definition_ref: 321.try_into()?,
                ..item
            }],
            browse: None,
        }],
    };
    let payload = encoded(containers::encode_container_views(&views))?;
    write(
        &mut stream,
        &wire::encode_state_delta(1, 42, 14, 1, 2, 1, &payload)?,
    )
    .await?;
    let mut sequence = 42;
    for (index, expected) in [
        trade::Intent::Offer {
            partner: partner(),
            item: 70,
        },
        trade::Intent::Accept { trade_id: [3; 16] },
        trade::Intent::Offer {
            partner: partner(),
            item: 80,
        },
        trade::Intent::Cancel,
        trade::Intent::Offer {
            partner: partner(),
            item: 80,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let revision = u64::try_from(index)? + 1;
        let bytes = read(&mut stream).await?;
        let envelope = wire::decode_wire_envelope(&bytes)?;
        let command = envelope.client_command(1)?;
        assert_eq!(
            command.command_type,
            trade::COMMAND_TYPE_PLAYER_TRADE_INTENT
        );
        assert_eq!(encoded(trade::decode_intent(command.payload))?, expected);
        sequence += 1;
        let result = match index {
            0 | 2 => trade::ResultCode::Offered,
            1 => trade::ResultCode::Accepted,
            3 => trade::ResultCode::Cancelled,
            _ => trade::ResultCode::TooFar,
        };
        write(
            &mut stream,
            &wire::encode_command_result(
                1,
                sequence,
                command.command_id,
                wire::CommandStatus::Accepted,
                &result.payload(),
            )?,
        )
        .await?;
        // A refusal has no changed state and therefore no delta.
        if index == 4 {
            break;
        }
        let state = if index == 0 || index == 2 {
            trade::State {
                trade_id: Some([3; 16]),
                partner: Some(partner()),
                phase: 2,
                reason: 0,
                own: Some(trade::Offer {
                    definition: 123,
                    count: 2,
                    sub_type: 0,
                }),
                counter: Some(trade::Offer {
                    definition: 456,
                    count: 1,
                    sub_type: 0,
                }),
                ..Default::default()
            }
        } else {
            trade::State {
                reason: if index == 1 { 1 } else { 2 },
                message: if index == 1 {
                    "Trade completed."
                } else {
                    "Trade cancelled."
                }
                .into(),
                ..Default::default()
            }
        };
        let payload = encoded(trade::encode_state(&state))?;
        sequence += 1;
        write(
            &mut stream,
            &wire::encode_state_delta(1, sequence, 26, revision, revision + 1, 1, &payload)?,
        )
        .await?;
        if index == 1 {
            let updated = item_view::CharacterInventory {
                main_backpack: Some(item_view::ItemEntry {
                    handle: 71.try_into()?,
                    item_definition_ref: 999.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                }),
                entries: vec![item_view::ItemEntry {
                    handle: 80.try_into()?,
                    item_definition_ref: 456.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                }],
                equipment: vec![],
            };
            let payload = encoded(item_view::encode_character_inventory_with_equip_drop(
                &updated, true,
            ))?;
            sequence += 1;
            write(
                &mut stream,
                &wire::encode_state_delta(1, sequence, 9, 1, 2, 1, &payload)?,
            )
            .await?;
        }
    }
    // Keep transport alive until the client drops, so the refusal can be inspected.
    let mut rest = Vec::new();
    stream.read_to_end(&mut rest).await?;
    Ok(())
}
async fn until(
    link: &PlayLink,
    check: impl Fn(&oteryn_client_ui::player_trade::PlayerTradePanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if check(&link.player_trade_panel()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn native_trade_world_selection_wire_settlement_cancel_and_receipt_only_refusal()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (client, server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(peer(server));
                    let session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: wire::CharacterId::decode(&uuid(4))?,
                            admission_material: b"synthetic",
                            client_build_id: "c21",
                            supported_capabilities: CAPS,
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let view = PlayView::new(TileCoord::new(0, 0), 0, &[])?
                        .with_entities(vec![actor()])?;
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(session, commands, outbox));
                    until(&link, |panel| {
                        panel.selected && !panel.items.inventory.entries.is_empty()
                    })
                    .await?;
                    link.player_trade_open_container(0);
                    until(&link, |panel| {
                        !panel.items.pending && !panel.items.containers.views.is_empty()
                    })
                    .await?;
                    assert!(
                        panel_text(&link.player_trade_panel()).contains("Container: item 321 x2")
                    );
                    for round in 0..3 {
                        let target = view
                            .player_trade_context_target(TileCoord::new(1, 0))
                            .ok_or("context target")?;
                        assert!(link.player_trade_choose_partner(target));
                        assert!(link.player_trade_choose_item(1)); // backpack slot precedes its contents
                        link.player_trade_offer_selected();
                        if round == 2 {
                            until(&link, |panel| {
                                !panel.pending && panel_text(panel).contains("Move closer")
                            })
                            .await?;
                            break;
                        }
                        until(&link, |panel| {
                            !panel.pending
                                && panel.state.as_ref().is_some_and(|state| state.phase == 2)
                        })
                        .await?;
                        let text = panel_text(&link.player_trade_panel());
                        assert!(text.contains("Your offer: item 123 x2"));
                        assert!(text.contains("Their offer: item 456 x1"));
                        link.player_trade_request(if round == 0 {
                            Intent::Accept
                        } else {
                            Intent::Cancel
                        });
                        let expected = if round == 0 {
                            "Trade completed."
                        } else {
                            "Trade cancelled."
                        };
                        until(&link, |panel| {
                            !panel.pending && panel_text(panel).contains(expected)
                        })
                        .await?;
                        if round == 0 {
                            until(&link, |panel| {
                                panel
                                    .items
                                    .inventory
                                    .entries
                                    .first()
                                    .is_some_and(|item| item.handle.get() == 80)
                            })
                            .await?;
                            assert!(
                                panel_text(&link.player_trade_panel())
                                    .contains("Backpack: item 456 x1")
                            );
                        }
                    }
                    drop(link);
                    task.await?;
                    peer.await??;
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "native trade thread panicked")?
}
