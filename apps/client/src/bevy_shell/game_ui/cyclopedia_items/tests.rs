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
                        ("Attack".into(), "30".into()),
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
    assert!(visible(&mut app).contains("Attack: 30"));
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
