use super::*;
use oteryn_session::test_support::wire::{self, DomainSnapshot};
use oteryn_session::{Admission, CharacterInventory, ItemEntry, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::io::DuplexStream;

type Error = Box<dyn std::error::Error + Send + Sync>;
const DEADLINE: Duration = Duration::from_secs(10);

#[test]
fn gap_0589_npc_shop_options_reach_the_server() -> std::result::Result<(), Error> {
    scenario(true)
}

#[test]
fn gap_0589_npc_shop_options_preserve_legacy_bytes() -> std::result::Result<(), Error> {
    scenario(false)
}

fn scenario(selected: bool) -> std::result::Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async { tokio::time::timeout(DEADLINE, play(selected)).await? })
        })?
        .join()
        .map_err(|_| "NPC shop test thread failed")?
}

fn id(marker: u8) -> [u8; 16] {
    let mut id = [marker; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

async fn join(
    peer: &mut DuplexStream,
    caps: &[u32],
    inventory: &[u8],
) -> std::result::Result<(), Error> {
    read_frame(peer).await?;
    write_frame(
        peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&id(1))?,
            world_id: wire::WorldId::decode(&id(2))?,
            channel_id: wire::ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: caps,
        })?,
    )
    .await?;
    use wire::world_spatial_entities::{
        EntityDetail, EntityFields, EntityKind, EntityLights, WorldSpatialEntitiesSnapshot,
        WorldSpatialEntity, encode_world_spatial_entities_snapshot_with_fields,
    };
    let position = wire::world_spatial::ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let own = id(4);
    let spatial = encode_world_spatial_entities_snapshot_with_fields(
        &WorldSpatialEntitiesSnapshot {
            content_generation: [17; 32],
            actor_position: position,
            own_identity: own,
            entities: vec![WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: EntityKind::Player,
                entity: EntityRef {
                    identity: own,
                    generation: 1,
                },
                position,
                detail: EntityDetail::Actor {
                    direction: wire::world_spatial::StepDirection::South,
                    appearance_ref: 0,
                    health_percent: 100,
                },
            }],
        },
        EntityFields::selected(caps),
        &EntityLights::new(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    let domains = [
        DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 2,
            payload: &spatial,
        },
        DomainSnapshot {
            domain_id: 2,
            revision: 1,
            snapshot_type: 1,
            payload: &overlay,
        },
        DomainSnapshot {
            domain_id: 7,
            revision: 1,
            snapshot_type: 1,
            payload: &[],
        },
        DomainSnapshot {
            domain_id: 8,
            revision: 1,
            snapshot_type: 1,
            payload: &[],
        },
        DomainSnapshot {
            domain_id: 9,
            revision: 1,
            snapshot_type: 1,
            payload: inventory,
        },
    ];
    for frame in wire::encode_single_chunk_snapshot(1, 1, 0, &domains)? {
        write_frame(peer, &frame).await?;
    }
    Ok(())
}

fn activate(
    app: &mut App,
    predicate: impl Fn(&Kind) -> bool,
    description: &str,
) -> std::result::Result<(), Error> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| predicate(&action.kind).then_some(entity))
        .ok_or_else(|| format!("missing {description} control"))?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(entity)
            .is_none(),
        "{description} is disabled"
    );
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

fn option(app: &mut App, caption: &str) -> std::result::Result<(), Error> {
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or_else(|| format!("missing {caption} control"))?;
    assert!(app.world().get::<Action>(button).is_some());
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(button)
            .is_none()
    );
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    Ok(())
}

async fn play(selected: bool) -> std::result::Result<(), Error> {
    use wire::item_view::{
        EquipmentSlot, EquippedItem, encode_character_inventory_with_equip_drop,
    };
    let caps = if selected {
        &[3, 4, 6, 12, 81, 589][..]
    } else {
        &[3, 4, 6, 12, 81][..]
    };
    let item = ItemEntry {
        appearance_id: 0,
        handle: 30.try_into()?,
        item_definition_ref: 99.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    let mut inventory = CharacterInventory {
        main_backpack: Some(ItemEntry {
            handle: 31.try_into()?,
            item_definition_ref: 100.try_into()?,
            ..item
        }),
        ..Default::default()
    };
    if selected {
        inventory.equipment.push(EquippedItem {
            slot: EquipmentSlot::Head,
            item,
        });
    } else {
        inventory.entries.push(item);
    }
    let inventory = encode_character_inventory_with_equip_drop(&inventory, true)
        .map_err(|error| format!("{error:?}"))?;
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let (session, joined) = tokio::join!(
        Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: wire::CharacterId::decode(&id(4))?,
                admission_material: b"grant",
                client_build_id: "npc-shop-options-test",
                supported_capabilities: caps,
                deadline: DEADLINE,
            }
        ),
        join(&mut peer, caps, &inventory)
    );
    joined?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let task = tokio::spawn(crate::play::run_session(session?, commands, outbox));
    let npc = EntityRef {
        identity: id(9),
        generation: 2,
    };
    let window = npc_service::NpcTradeWindow {
        details: vec![],
        open: Some((npc, 7)),
        offers: vec![
            npc_service::NpcOffer {
                item_definition_ref: 42,
                side: NpcTradeSide::Buy,
                count: 1,
                unit_price: 12,
            },
            npc_service::NpcOffer {
                item_definition_ref: 99,
                side: NpcTradeSide::Sell,
                count: 1,
                unit_price: 7,
            },
        ],
    };
    let payload =
        npc_service::encode_npc_trade_window(&window).map_err(|error| format!("{error:?}"))?;
    write_frame(
        &mut peer,
        &wire::encode_state_delta(1, 1, 8, 1, 2, 1, &payload)?,
    )
    .await?;
    while !link.npc_shop_is_open() {
        tokio::task::yield_now().await;
    }
    assert_eq!(link.npc_shop_panel().live.window, window);

    let mut host = HostOwner::new(None).map_err(|error| error.to_string())?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    if selected {
        option(&mut app, "Buy in backpacks")?;
    } else {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .all(|text| text.0 != "Buy in backpacks")
        );
    }
    activate(
        &mut app,
        |kind| matches!(kind, Kind::Select(_)),
        "buy offer",
    )?;
    activate(&mut app, |kind| matches!(kind, Kind::Trade), "buy")?;

    let reply = |sequence, id, payload: &[u8]| {
        wire::encode_command_result(1, sequence, id, wire::CommandStatus::Accepted, payload)
    };
    let frame = read_frame(&mut peer).await?;
    let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
    assert_eq!(
        command.command_type,
        npc_service::COMMAND_TYPE_NPC_TRADE_INTENT
    );
    let (buy, packed) = npc_service::decode_npc_trade_intent_for(command.payload, selected)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!((buy.offer_index, buy.quantity, packed), (0, 1, selected));
    assert_payload(
        command.payload,
        npc.identity,
        &[0x20, 1, 0x28, 1, 0x30, 12],
        selected,
    );
    write_frame(
        &mut peer,
        &reply(
            2,
            command.command_id,
            &npc_service::encode_npc_intent_result(oteryn_session::NpcIntentDisposition::Ok),
        )?,
    )
    .await?;
    while panel(&app)?.live.pending {
        tokio::task::yield_now().await;
    }
    app.update();

    activate(
        &mut app,
        |kind| matches!(kind, Kind::Side(NpcTradeSide::Sell)),
        "sell tab",
    )?;
    if selected {
        option(&mut app, "Include equipped in Sell All")?;
    }
    activate(&mut app, |kind| matches!(kind, Kind::SellAll), "sell all")?;
    let frame = read_frame(&mut peer).await?;
    let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
    assert_eq!(command.command_type, wire::npc_shop::COMMAND_TYPE_NPC_SHOP);
    let (sale, equipped) = wire::npc_shop::decode_npc_shop_intent_for(command.payload, selected)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        (sale.action, sale.offer_index, equipped),
        (wire::npc_shop::NpcShopAction::SellAll, 1, selected)
    );
    assert_payload(
        command.payload,
        npc.identity,
        &[0x18, 1, 0x20, 2, 0x28, 2, 0x30, 7],
        selected,
    );
    let sold = wire::npc_shop::encode_npc_shop_result(&wire::npc_shop::NpcShopResult {
        disposition: oteryn_session::NpcIntentDisposition::Ok,
        look_text: String::new(),
        sold_units: 1,
        total_gold_units: 7,
    })
    .map_err(|error| format!("{error:?}"))?;
    write_frame(&mut peer, &reply(3, command.command_id, &sold)?).await?;
    while panel(&app)?.pending {
        tokio::task::yield_now().await;
    }
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Sold 1 units for 7 gold")
    );
    task.abort();
    Ok(())
}

// Both commands use the same actor and catalogue fields before their action-specific fields.
fn assert_payload(payload: &[u8], npc: [u8; 16], fields: &[u8], selected: bool) {
    let mut expected = vec![0x0a, 20, 0x0a, 16];
    expected.extend(npc);
    expected.extend([0x10, 2, 0x10, 7]);
    expected.extend_from_slice(fields);
    if selected {
        expected.extend([0x38, 1]);
    }
    assert_eq!(payload, expected);
}

fn panel(app: &App) -> std::result::Result<oteryn_client_ui::npc_shop::NpcShopPanel, Error> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .npc_shop_panel())
}
