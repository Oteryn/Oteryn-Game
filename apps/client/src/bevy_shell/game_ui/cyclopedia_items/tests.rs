use super::*;
use crate::{
    bevy_shell::owner::{HostOwner, Phase},
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_ui::inspection::{self as wire, catalogue};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type CatalogueWindow = (
    App,
    crate::play::PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);

fn catalogue_window() -> Result<CatalogueWindow, Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    outbox.bevy_market_initialize(true);
    link.use_shared_frontend();
    link.bevy_item_details_selected(true);
    link.item_catalogue_open();
    let PlayCommand::ServerMechanic(command) = receiver
        .try_recv()
        .map_err(|error| format!("command: {error}"))?
    else {
        return Err("catalogue request".into());
    };
    assert!(matches!(
        wire::decode_request(&command.payload),
        Ok(wire::Request::CatalogueQuery(_))
    ));
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view,
        link: link.bevy_item_details_link(),
    });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert!(visible(&mut app).contains("Cyclopedia Items"));
    let reply = wire::Reply {
        status: 1,
        requests: vec![],
        catalogue: None,
        items: vec![],
    };
    link.bevy_item_details_response(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            payload: wire::encode_reply(&wire::Reply {
                items: vec![wire::InspectedItem {
                    slot: 0,
                    definition_ref: 42,
                    count: 1,
                    tier: 0,
                    name: "Confirmed sword".into(),
                    attributes: vec![
                        ("Catalogue next".into(), "42".into()),
                        ("Catalogue categories".into(), "Weapons".into()),
                        ("Category".into(), "Weapons".into()),
                    ],
                }],
                catalogue: None,
                ..reply
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    assert!(visible(&mut app).contains("Confirmed sword"));
    Ok((app, link, receiver, outbox))
}

#[test]
fn cyclopedia_items_rejects_invalid_search_for_all_filters() -> TestResult {
    let (mut app, _link, mut receiver, _outbox) = catalogue_window()?;
    app.world_mut()
        .query_filtered::<&mut EditableText, With<Search>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text(&format!("{}x", "é".repeat(128)));
    for action in [
        Action::Browse,
        Action::Next,
        Action::Category(true),
        Action::Category(false),
    ] {
        click(&mut app, action)?;
        assert!(receiver.try_recv().is_err(), "invalid search submitted");
    }
    Ok(())
}

#[test]
fn wire_limit_preserves_catalogue_filters() -> TestResult {
    let (mut app, link, mut receiver, _outbox) = catalogue_window()?;
    let original = link.inspection_panel();
    app.world_mut()
        .query_filtered::<&mut EditableText, With<Search>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text(&"x".repeat(65));
    for action in [
        Action::Category(false),
        Action::Category(true),
        Action::Next,
        Action::Browse,
    ] {
        click(&mut app, action)?;
        assert_eq!(link.inspection_panel(), original);
        assert!(receiver.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn cyclopedia_items_reaches_catalogue_and_renders_confirmed_details() -> TestResult {
    let (mut app, link, mut receiver, outbox) = catalogue_window()?;
    click(&mut app, Action::Select(42))?;
    let confirmed = link.inspection_panel().reply.ok_or("confirmed catalogue")?;
    click(&mut app, Action::Details)?;
    let PlayCommand::ServerMechanic(details) = receiver
        .try_recv()
        .map_err(|error| format!("details: {error}"))?
    else {
        return Err("item details".into());
    };
    assert_eq!(
        wire::decode_request(&details.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::Catalogue(42)
    );
    link.bevy_item_details_response(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            payload: wire::encode_reply(&confirmed).map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    click(&mut app, Action::Prices)?;
    reply_to_prices(&mut receiver, &outbox)?;
    app.update();
    assert!(visible(&mut app).contains("average 19"));
    // Topmost modal owns activation, even for a directly triggered Bevy event.
    let blocker = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(200), Node::default()))
        .id();
    click(&mut app, Action::Details)?;
    assert!(receiver.try_recv().is_err());
    app.world_mut().entity_mut(blocker).despawn();
    click(&mut app, Action::Next)?;
    let PlayCommand::ServerMechanic(next) = receiver
        .try_recv()
        .map_err(|error| format!("next: {error}"))?
    else {
        return Err("next catalogue page".into());
    };
    assert!(matches!(
        wire::decode_request(&next.payload),
        Ok(wire::Request::CatalogueQuery(catalogue::Query::Items {
            after: 42,
            ..
        }))
    ));
    click(&mut app, Action::Details)?;
    assert!(
        receiver.try_recv().is_err(),
        "pending catalogue must not submit again"
    );
    click(&mut app, Action::Close)?;
    assert!(!link.inspection_panel().catalogue_open);
    assert!(!visible(&mut app).contains("Cyclopedia Items"));
    // The native shortcut reopens retained pending work without a duplicate request.
    use oteryn_input_actions::{
        ButtonState, KeyCode as InputKey, Modifier, Modifiers, NormalizedInputEvent,
    };
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: InputKey::new(12).map_err(|e| format!("{e:?}"))?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    app.update();
    assert!(visible(&mut app).contains("Cyclopedia Items"));
    assert!(receiver.try_recv().is_err());
    Ok(())
}
fn visible(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn click(app: &mut App, action: Action) -> Result<(), Box<dyn std::error::Error>> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.action == action).then_some(entity))
        .ok_or("catalogue button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

#[test]
fn cyclopedia_items_pages_selected_item_drop_sources() -> TestResult {
    drop_sources_pages(false)
}

#[test]
fn drop_sources_continue_after_inventory_inspection() -> TestResult {
    drop_sources_pages(true)
}

fn reply_to_prices(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &crate::play::Outbox,
) -> TestResult {
    let PlayCommand::Market(command) = receiver
        .try_recv()
        .map_err(|error| format!("command: {error}"))?
    else {
        return Err("market offers".into());
    };
    assert_eq!((command.0.action, command.0.definition), (2, 42));
    use oteryn_client_runtime::market::{MarketResponse, market};
    let mut offer = [7; 16];
    offer[6] = 0x70;
    offer[8] = 0x80;
    outbox.bevy_market_response(&MarketResponse {
        status: oteryn_session::CommandStatus::Accepted,
        reply: Some(market::MarketReply {
            action: command.0.action,
            code: 1,
            offer: [0; 16],
            rows: vec![market::MarketRow {
                offer,
                definition: 42,
                side: 2,
                amount: 5,
                price: 100,
                state: 1,
                anonymous: false,
            }],
        }),
    });
    let PlayCommand::Feature(statistics) = receiver
        .try_recv()
        .map_err(|error| format!("statistics: {error}"))?
    else {
        return Err("price statistics".into());
    };
    use oteryn_client_runtime::market::market_details::PriceSummary;
    use oteryn_client_ui::market::details::{Query, Reply, Statistics};
    assert_eq!(
        Query::Statistics { definition: 42 },
        oteryn_client_runtime::market::market_details::decode_query(&statistics.payload)
            .map_err(|e| format!("{e:?}"))?
    );
    let summary = PriceSummary {
        trades: 1,
        units: 1,
        minimum: Some(19),
        maximum: Some(19),
        average: Some(19),
    };
    outbox.bevy_market_reply(
        &statistics,
        &oteryn_session::GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_client_runtime::market::market_details::encode_reply(
                &Reply::Statistics(Statistics {
                    definition: 42,
                    as_of_us: 2592000000000,
                    trades: 1,
                    units: 1,
                    minimum: Some(19),
                    maximum: Some(19),
                    average: Some(19),
                    sell: summary,
                    ..default()
                }),
            )
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    Ok(())
}

const RELATED_OFFER: &str =
    "Loaded page sell: 1 offers, 5 units, price 100–100, weighted average 100";

fn drop_sources_pages(inspect_between_pages: bool) -> TestResult {
    let (mut app, link, mut receiver, outbox) = catalogue_window()?;
    click(&mut app, Action::Select(42))?;
    let confirmed = link.inspection_panel().reply.ok_or("confirmed catalogue")?;
    for (caption, after) in [("Drop sources", 0), ("Next drop sources", 10)] {
        let entity = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
            .ok_or("drop source button")?;
        assert!(app.world().get::<InteractionDisabled>(entity).is_none());
        app.world_mut().trigger(Activate { entity });
        app.update();
        let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
            return Err("drop source request".into());
        };
        assert_eq!(
            wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
            wire::Request::CatalogueQuery(catalogue::Query::Drops {
                reference: 42,
                after
            })
        );
        let mut page = confirmed.clone();
        if after == 0 {
            page.items[0]
                .attributes
                .push(("Catalogue drop next".into(), "10".into()));
        }
        link.bevy_item_details_response(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 3,
                server_sequence: 3,
                status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
                payload: wire::encode_reply(&page).map_err(|e| format!("{e:?}"))?,
            },
        );
        if after == 0 && inspect_between_pages {
            let item = oteryn_session::ItemEntry {
                handle: 7.try_into()?,
                item_definition_ref: 42.try_into()?,
                appearance_id: 0,
                count: 1.try_into()?,
                sub_type: 0,
            };
            crate::play::PlayLink::bevy_items_fixture(
                &outbox,
                Some(oteryn_session::CharacterInventory {
                    main_backpack: None,
                    entries: vec![item],
                    equipment: vec![],
                }),
                Some(Default::default()),
                false,
                true,
            );
            assert!(link.inventory_select(item.handle));
            app.update();
            click(&mut app, Action::Prices)?;
            assert_eq!(link.inspection_panel().catalogue_market, Some(42));
            reply_to_prices(&mut receiver, &outbox)?;
            app.update();
            assert!(visible(&mut app).contains("average 19"));
            assert!(
                visible(&mut app).contains(RELATED_OFFER),
                "initial market offer"
            );
            super::super::item_details::install(&mut app);
            crate::bevy_shell::ui::window_layout::install(&mut app);
            link.inspection_set_open(true);
            app.update();
            let inspect = app
                .world_mut()
                .query::<(&Text, &ChildOf)>()
                .iter(app.world())
                .find_map(|(text, parent)| {
                    (text.0 == "Inspect selected").then_some(parent.parent())
                })
                .ok_or("inspect button")?;
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(inspect, bevy::input_focus::FocusCause::Navigated);
            app.update();
            app.world_mut().trigger(Activate { entity: inspect });
            app.update();
            let PlayCommand::ServerMechanic(inspect) = receiver.try_recv()? else {
                return Err("inventory inspection command".into());
            };
            assert_eq!(
                wire::decode_request(&inspect.payload).map_err(|e| format!("{e:?}"))?,
                Request::Item(item.handle)
            );
            link.bevy_item_details_response(
                &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                    command_id: 4,
                    server_sequence: 4,
                    status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
                    payload: wire::encode_reply(&confirmed).map_err(|e| format!("{e:?}"))?,
                },
            );
            app.update();
            assert!(
                visible(&mut app).contains(RELATED_OFFER),
                "offer after inventory inspection"
            );
            let close = app
                .world_mut()
                .query::<(&Text, &ChildOf)>()
                .iter(app.world())
                .find_map(|(text, parent)| {
                    (text.0 == "Close" && app.world().get::<Control>(parent.parent()).is_none())
                        .then_some(parent.parent())
                })
                .ok_or("item details close")?;
            app.world_mut().trigger(Activate { entity: close });
            app.update();
        }
        app.update();
    }
    if inspect_between_pages {
        assert!(
            visible(&mut app).contains(RELATED_OFFER),
            "offer after paging drop sources"
        );
        assert!(visible(&mut app).contains("Related market offers:"));
        assert!(visible(&mut app).contains("average 19"));
        assert_eq!(link.inspection_panel().catalogue_market, Some(42));
    }
    Ok(())
}

#[test]
fn trimmed_search_fits_wire_limit() -> TestResult {
    let (mut app, _, mut commands, _) = catalogue_window()?;
    app.world_mut()
        .query_filtered::<&mut EditableText, With<Search>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text(&format!("{} ", "x".repeat(64)));
    click(&mut app, Action::Browse)?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("missing browse request".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::CatalogueQuery(
            catalogue::items(0, 0, &"x".repeat(64)).map_err(|e| format!("{e:?}"))?
        )
    );
    Ok(())
}

#[test]
fn catalogue_search_can_acquire_focus() -> TestResult {
    let (mut app, _, _, _) = catalogue_window()?;
    let world = app.world_mut();
    let field = world
        .query_filtered::<Entity, With<Search>>()
        .single(world)?;
    crate::bevy_shell::ui::assert_editor_acquires_focus(world, field);
    Ok(())
}

#[test]
fn selected_item_reaches_quick_loot_lists_and_container_manager() -> TestResult {
    use oteryn_client_ui::quick_loot::{Action as LootAction, QuickLootFilterMode};
    let (mut app, link, mut receiver, outbox) = catalogue_window()?;
    outbox.bevy_quick_loot_selected(true);
    click(&mut app, Action::Select(42))?;
    click(&mut app, Action::LootFilter(QuickLootFilterMode::Skip))?;
    assert!(matches!(
        receiver.try_recv()?,
        PlayCommand::ServerMechanic(_)
    ));
    assert_eq!(
        link.quick_loot_panel().pending,
        Some(LootAction::Add {
            mode: QuickLootFilterMode::Skip,
            definition: 42,
        })
    );
    click(&mut app, Action::ManageLoot)?;
    assert!(link.quick_loot_panel().open);
    assert!(!link.inspection_panel().catalogue_open);
    Ok(())
}

#[test]
fn item_equipment_filters_match_confirmed_attributes_and_player_stats() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{GeneralStats, TabExtension, TabView};
    let (mut app, link, mut receiver, outbox) = catalogue_window()?;
    let mut details = link.inspection_panel().reply.ok_or("browse reply")?;
    outbox.bevy_cyclopedia_initialize(true, true, true);
    let selectable = |app: &mut App| {
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .any(|c| c.action == Action::Select(42))
    };
    click(&mut app, Action::Hands)?;
    let PlayCommand::ServerMechanic(cmd) = receiver.try_recv()? else {
        return Err("equipment details".into());
    };
    assert_eq!(
        wire::decode_request(&cmd.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::Catalogue(42)
    );
    details.items[0].attributes = vec![
        ("Attack".into(), "30".into()),
        ("Classification".into(), "2".into()),
        ("Body Position".into(), "both hands".into()),
        ("Professions".into(), "druid".into()),
        ("Required Level".into(), "50".into()),
    ];
    link.bevy_item_details_response(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            payload: wire::encode_reply(&details).map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    assert!(!selectable(&mut app));
    click(&mut app, Action::Hands)?;
    assert!(selectable(&mut app));
    click(&mut app, Action::Select(42))?;
    assert!(visible(&mut app).contains("Attack: 30"));
    click(&mut app, Action::Class)?;
    assert!(!selectable(&mut app));
    click(&mut app, Action::Class)?;
    click(&mut app, Action::Class)?;
    assert!(selectable(&mut app));
    click(&mut app, Action::Level)?;
    assert!(matches!(receiver.try_recv()?, PlayCommand::Feature(c) if c.capability == 199));
    outbox.bevy_cyclopedia_result(&oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        payload: TabView::GeneralStats(GeneralStats {
            level: 40,
            ..Default::default()
        })
        .encode_extended(&TabExtension {
            vocation_id: Some(6),
            deaths: None,
        })
        .map_err(|_| "stats")?,
    });
    app.update();
    assert!(!selectable(&mut app));
    click(&mut app, Action::Level)?;
    assert!(selectable(&mut app));
    click(&mut app, Action::Vocation)?;
    assert!(!selectable(&mut app));
    Ok(())
}

#[test]
fn selected_item_tracks_drops_and_excludes_quick_sales() -> TestResult {
    let (mut app, link, _, outbox) = catalogue_window()?;
    outbox.bevy_analyser_initialize();
    click(&mut app, Action::Select(42))?;
    click(&mut app, Action::Track)?;
    assert!(link.analyser_panel().loot_tracked.contains(&42));
    click(&mut app, Action::ExcludeSale)?;
    assert!(link.npc_shop_panel().sale_exclusions.contains(&42));
    click(&mut app, Action::Track)?;
    assert!(!link.analyser_panel().loot_tracked.contains(&42));
    Ok(())
}

#[test]
fn item_valuation_uses_confirmed_market_or_entered_unit_price() -> TestResult {
    let (mut app, link, mut receiver, outbox) = catalogue_window()?;
    outbox.bevy_analyser_initialize();
    click(&mut app, Action::Select(42))?;
    click(&mut app, Action::Prices)?;
    reply_to_prices(&mut receiver, &outbox)?;
    app.update();
    click(&mut app, Action::MarketValue)?;
    assert_eq!(link.analyser_panel().loot_prices.get(&42), Some(&19));
    let first = link.loot_prices();
    let second = link.loot_prices();
    assert!(std::ptr::eq(&first[&42], &second[&42]));
    use oteryn_client_ui::market::details::{Query, Reply, Statistics};
    assert!(link.market_query(Query::Statistics { definition: 42 }));
    let PlayCommand::Feature(statistics) = receiver.try_recv()? else {
        return Err("market statistics".into());
    };
    outbox.bevy_market_reply(
        &statistics,
        &oteryn_session::GatedCommandResult {
            command_id: 3,
            server_sequence: 3,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_client_runtime::market::market_details::encode_reply(
                &Reply::Statistics(Statistics {
                    definition: 42,
                    as_of_us: 2592000000001,
                    since_us: 1,
                    ..default()
                }),
            )
            .map_err(|error| format!("{error:?}"))?,
        },
    );
    assert!(
        !link.analyser_panel().loot_prices.contains_key(&42),
        "a confirmed empty market history clears the old analyser quote"
    );
    app.world_mut()
        .query_filtered::<&mut EditableText, With<OwnPrice>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text("73");
    click(&mut app, Action::OwnValue)?;
    assert_eq!(link.analyser_panel().loot_prices.get(&42), Some(&73));
    use oteryn_client_ui::npc_shop::NpcTradeSide;
    use oteryn_session::{AppliedDelta, EntityRef, SessionEvent, npc_service};
    outbox.bevy_npc_shop_supported();
    let shop = |revision, price| {
        SessionEvent::NpcTradeWindow(AppliedDelta {
            server_sequence: revision,
            base_revision: revision - 1,
            new_revision: revision,
            value: npc_service::NpcTradeWindow {
                open: Some((
                    EntityRef {
                        identity: [9; 16],
                        generation: 1,
                    },
                    revision,
                )),
                offers: vec![npc_service::NpcOffer {
                    item_definition_ref: 42,
                    side: NpcTradeSide::Sell,
                    count: 1,
                    unit_price: price,
                }],
                details: vec![],
            },
        })
    };
    app.world_mut()
        .query_filtered::<&mut EditableText, With<OwnPrice>>()
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text("123");
    outbox.push(vec![shop(1, 32)]);
    app.update();
    assert!(visible(&mut app).contains("Current merchant buys 1 for 32 gold"));
    assert_eq!(
        app.world_mut()
            .query_filtered::<&EditableText, With<OwnPrice>>()
            .single(app.world())?
            .value()
            .to_string(),
        "123"
    );
    click(&mut app, Action::MerchantValue)?;
    assert_eq!(link.analyser_panel().loot_prices.get(&42), Some(&32));
    outbox.push(vec![shop(2, 41)]);
    app.update();
    assert_eq!(link.analyser_panel().loot_prices.get(&42), Some(&41));
    click(&mut app, Action::ExcludeSale)?;
    assert!(link.npc_shop_panel().sale_exclusions.contains(&42));
    let quote = link.npc_shop_panel().live.quotes()[0];
    link.npc_sell_all_request(quote);
    assert!(
        receiver.try_recv().is_err(),
        "excluded catalogue item was sold"
    );
    click(&mut app, Action::ExcludeSale)?;
    link.npc_sell_all_request(quote);
    assert!(matches!(receiver.try_recv()?, PlayCommand::NpcShop(_)));
    Ok(())
}
