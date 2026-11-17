use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, play_channel},
};
use oteryn_session::container_tree::{ContainerView, ContainerViews};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn views() -> std::result::Result<ContainerViews, Box<dyn std::error::Error>> {
    Ok(ContainerViews {
        views: vec![
            ContainerView {
                view_id: 0,
                container_handle: 10.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![tests::entry(11, 7)?, tests::entry(12, 3)?],
                browse: None,
            },
            ContainerView {
                view_id: 1,
                container_handle: 20.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![],
                browse: None,
            },
        ],
    })
}
fn app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    use bevy::asset::AssetPlugin;
    let (link, commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(views()?),
        true,
        true,
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<bevy::text::RemSize>()
    .init_resource::<UiScale>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .insert_non_send(tests::host(link)?)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            crate::bevy_shell::game_ui::setup,
        )
            .chain(),
    )
    .add_systems(Update, crate::bevy_shell::game_ui::project);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, name: &str) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, value)| value.as_str() == name)
        .map(|(entity, _)| entity)
        .ok_or_else(|| format!("missing actual control: {name}"))?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn container_header_and_destination_choice_dispatch_ordered_real_handles_once() -> Result {
    let (mut app, mut commands, _) = app()?;
    click(&mut app, "inventory.bulk.open")?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .container_activate(1)
    );
    click(&mut app, "inventory.bulk.destination")?;
    let PlayCommand::MoveContainerContents(requests) = commands.try_recv()? else {
        return Err("bulk move".into());
    };
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].source, 11.try_into()?);
    assert_eq!(requests[1].source, 12.try_into()?);
    let destination = ItemMoveDestination::Container(20.try_into()?, None);
    assert!(
        requests
            .iter()
            .all(|request| request.destination == destination && request.count.is_none())
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .container_panel()
            .views,
        Some(views()?)
    );
    click(&mut app, "inventory.bulk.destination")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn changed_source_or_destination_snapshot_never_uses_recycled_open_view() -> Result {
    for source in [true, false] {
        let (mut app, mut commands, outbox) = app()?;
        click(&mut app, "inventory.bulk.open")?;
        app.update();
        let mut changed = views()?;
        if source {
            changed.views[0].entries.pop();
        } else {
            changed.views[1].container_handle = 30.try_into()?;
        }
        outbox.push(vec![oteryn_session::SessionEvent::ContainerViews(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 0,
                new_revision: 1,
                value: changed.clone(),
            },
        )]);
        click(&mut app, "inventory.bulk.destination")?;
        assert!(commands.try_recv().is_err());
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .container_panel()
                .views,
            Some(changed)
        );
    }
    Ok(())
}
#[test]
fn higher_modal_pending_container_and_expired_session_block_real_destination_choice() -> Result {
    for mode in 0..3 {
        let (mut app, mut commands, _) = app()?;
        click(&mut app, "inventory.bulk.open")?;
        app.update();
        match mode {
            0 => {
                app.world_mut().spawn((
                    super::super::GameModal,
                    GlobalZIndex(180),
                    Node::default(),
                ));
            }
            1 => {
                assert!(
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .ok_or("play")?
                        .link
                        .container_open(10.try_into()?)
                );
                let _ = commands.try_recv()?;
            }
            _ => app.world_mut().non_send_mut::<HostOwner>().close(),
        }
        click(&mut app, "inventory.bulk.destination")?;
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn bound_owner_rejects_actual_server_modal_and_closed_command_channel() -> Result {
    let (link, commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(views()?),
        true,
        true,
    );
    let views = views()?;
    {
        let _modal = link.bevy_server_modal_fixture();
        assert!(link.server_modal_active());
        assert!(!link.container_move_all_bound(&views.views[0], &views.views[1]));
        assert!(!link.item_movement().pending);
    }
    assert!(!link.server_modal_active());
    drop(commands);
    assert!(!link.container_move_all_bound(&views.views[0], &views.views[1]));
    assert!(!link.item_movement().pending);
    assert_eq!(link.container_panel().views, Some(views));
    assert!(
        link.item_movement()
            .feedback
            .is_some_and(|message| message.contains("Session ended"))
    );
    Ok(())
}

fn has_text(app: &mut App, text: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|value| value.0 == text)
}

fn replace_views(outbox: &crate::play::Outbox, views: ContainerViews) {
    PlayLink::bevy_items_fixture(outbox, Some(Default::default()), Some(views), true, true);
}

#[test]
fn container_windows_minimize_and_resize_independently_without_closing() -> Result {
    let (mut app, mut commands, _) = app()?;
    assert!(has_text(&mut app, "Container #1 (2/20)"));
    let body = |app: &mut App, id: u8| -> Result<(Display, Val)> {
        let name = format!("inventory.container.{id}.body");
        app.world_mut()
            .query::<(&Name, &Node)>()
            .iter(app.world())
            .find(|(value, _)| value.as_str() == name)
            .map(|(_, node)| (node.display, node.height))
            .ok_or_else(|| name.into())
    };
    assert_eq!(body(&mut app, 0)?.1, px(182.));
    click(&mut app, "inventory.container.0.shrink")?;
    app.update();
    assert_eq!(body(&mut app, 0)?.1, px(145.));
    assert_eq!(body(&mut app, 1)?.1, px(182.));
    click(&mut app, "inventory.container.0.minimize")?;
    app.update();
    assert_eq!(body(&mut app, 0)?.0, Display::None);
    assert_eq!(body(&mut app, 1)?.0, Display::Flex);
    click(&mut app, "inventory.container.0.minimize")?;
    app.update();
    assert_eq!(body(&mut app, 0)?, (Display::Flex, px(145.)));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn browse_pagination_uses_server_indexes_and_rejects_stale_page_buttons() -> Result {
    use oteryn_session::browse_field::{
        BrowseFieldIntent, BrowseFieldPage, decode_browse_field_intent,
    };
    let (mut app, mut commands, outbox) = app()?;
    let mut page_views = views()?;
    page_views.views.truncate(1);
    let view = &mut page_views.views[0];
    view.capacity = 30;
    view.entries = (0..30)
        .map(|id| tests::entry(100 + id, 1))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    view.browse = Some(BrowseFieldPage {
        position: oteryn_session::item_view::WorldTilePosition {
            x: 100,
            y: 100,
            floor: -7,
        },
        index: 30,
        total: 61,
    });
    replace_views(&outbox, page_views.clone());
    app.update();
    assert!(has_text(&mut app, "Browse Field (30/61)"));
    assert!(has_text(&mut app, "Page 2 of 3"));
    // Change the authoritative page while its old Previous control still exists.
    page_views.views[0].browse.as_mut().ok_or("browse")?.index = 0;
    replace_views(&outbox, page_views.clone());
    click(&mut app, "inventory.container.0.previous")?;
    assert!(commands.try_recv().is_err(), "stale page seek");
    app.update();
    assert!(click(&mut app, "inventory.container.0.previous").is_err());
    page_views.views[0].browse.as_mut().ok_or("browse")?.index = 30;
    replace_views(&outbox, page_views.clone());
    app.update();
    click(&mut app, "inventory.container.0.next")?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("page command".into());
    };
    assert_eq!((command.capability, command.command_type), (230, 440));
    assert_eq!(
        decode_browse_field_intent(&command.payload).map_err(|e| format!("{e:?}"))?,
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 60
        }
    );
    click(&mut app, "inventory.container.0.next")?;
    assert!(commands.try_recv().is_err(), "duplicate pending seek");
    // The server returns the final partial page; navigation reflects that page only.
    page_views.views[0].browse.as_mut().ok_or("browse")?.index = 60;
    page_views.views[0].entries.truncate(1);
    replace_views(&outbox, page_views);
    app.update();
    assert!(
        click(&mut app, "inventory.container.0.next").is_err(),
        "last page has no next control"
    );
    Ok(())
}

#[test]
fn container_geometry_rejects_recycled_views_and_bounds_row_controls() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    let original = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "inventory.container.0.minimize")
        .map(|(entity, _)| entity)
        .ok_or("minimize")?;
    let mut replacement = views()?;
    replacement.views[0].container_handle = 99.try_into()?;
    replace_views(&outbox, replacement);
    app.world_mut().trigger(Activate { entity: original });
    app.update();
    for _ in 0..4 {
        click(&mut app, "inventory.container.0.shrink")?;
        app.update();
    }
    assert!(click(&mut app, "inventory.container.0.shrink").is_err());
    assert!(click(&mut app, "inventory.container.0.grow").is_ok());
    app.update();
    for _ in 0..3 {
        click(&mut app, "inventory.container.0.grow")?;
        app.update();
    }
    assert!(click(&mut app, "inventory.container.0.grow").is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn inventory_browse_ground_opens_the_live_player_tile() -> Result {
    use oteryn_session::browse_field::{
        BrowseFieldIntent, BrowseFieldPage, decode_browse_field_intent,
    };
    let (mut app, mut commands, outbox) = app()?;
    assert!(
        click(&mut app, "inventory.container.browse").is_err(),
        "unnegotiated entry"
    );
    let position = oteryn_session::item_view::WorldTilePosition {
        x: 101,
        y: 102,
        floor: -7,
    };
    let mut fields = views()?;
    fields.views.truncate(1);
    fields.views[0].capacity = 30;
    fields.views[0].entries.clear();
    fields.views[0].browse = Some(BrowseFieldPage {
        position,
        index: 0,
        total: 0,
    });
    PlayLink::bevy_items_fixture(&outbox, Some(Default::default()), Some(fields), true, true);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(101, 102), -7, &[])?;
    view.use_shared_frontend();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view = view;
    app.update();
    click(&mut app, "inventory.container.browse")?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("browse command".into());
    };
    assert_eq!((command.capability, command.command_type), (230, 440));
    assert_eq!(
        decode_browse_field_intent(&command.payload).map_err(|e| format!("{e:?}"))?,
        BrowseFieldIntent::Open { position }
    );
    click(&mut app, "inventory.container.browse")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn container_names_resolve_live_inspection_handles_without_opening_or_stealing_inspector() -> Result
{
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::inspection::{
        InspectedItem, Reply, Request, decode_request, encode_reply,
    };
    let (mut app, mut commands, outbox) = app()?;
    let link = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_item_details_link();
    link.bevy_item_details_selected(true);
    app.update();
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("name lookup".into());
    };
    assert_eq!((command.capability, command.command_type), (132, 153));
    assert_eq!(
        decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Item(10.try_into()?)
    );
    assert!(!link.inspection_panel().open);
    let reply = |name: &str| -> Result<ServerMechanicResponse> {
        let reply = Reply {
            status: 1,
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 42,
                count: 1,
                tier: 0,
                name: name.into(),
                attributes: vec![],
            }],
            requests: vec![],
            catalogue: None,
        };
        Ok(ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&reply).map_err(|e| format!("{e:?}"))?,
        })
    };
    link.bevy_item_details_response(&reply("Leather backpack")?);
    link.inspection_set_open(true);
    app.update();
    assert!(has_text(&mut app, "Leather backpack (2/20)"));
    assert!(
        commands.try_recv().is_err(),
        "active inspector owns its request slot"
    );
    link.inspection_set_open(false);
    app.update();
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("second name lookup".into());
    };
    assert_eq!(
        decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Item(20.try_into()?)
    );
    let mut replacement = views()?;
    replacement.views[1].container_handle = 99.try_into()?;
    replace_views(&outbox, replacement);
    link.bevy_item_details_response(&reply("Stale bag")?);
    app.update();
    assert!(has_text(&mut app, "Container #2 (0/20)"));
    assert!(!has_text(&mut app, "Stale bag (0/20)"));
    assert!(has_text(&mut app, "Leather backpack (2/20)"));
    Ok(())
}

fn ground_up(app: &mut App, view: u8) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, Control::GroundUp { view: at, .. } if *at == view).then_some(entity)
        })
}

#[test]
fn ground_container_up_opens_its_browse_field_and_shows_the_server_view() -> Result {
    use oteryn_session::{
        CommandStatus, GatedCommandResult,
        browse_field::BrowseFieldPage,
        container_tree::{
            ContainerViewIntent, ContainerViewOutcome, decode_container_view_intent,
            encode_container_view_result,
        },
    };
    let (mut app, mut commands, outbox) = app()?;
    assert!(
        ground_up(&mut app, 0).is_none(),
        "no ground Up without capability 230"
    );
    outbox.bevy_browse_field_supported();
    app.update();
    let up = ground_up(&mut app, 0).ok_or("ground Up")?;
    app.world_mut().trigger(Activate { entity: up });
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("up command".into());
    };
    assert_eq!((command.capability, command.command_type), (14, 21));
    assert_eq!(
        decode_container_view_intent(&command.payload).map_err(|e| format!("{e:?}"))?,
        ContainerViewIntent::Up { view_id: 0 }
    );
    app.world_mut().trigger(Activate { entity: up });
    assert!(
        commands.try_recv().is_err(),
        "a pending Up is not sent twice"
    );
    outbox.bevy_browse_field_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_container_view_result(ContainerViewOutcome::Opened),
    });
    // The server replaces the same window id by the tile's browse field.
    let mut field = views()?;
    field.views.truncate(1);
    field.views[0].capacity = 30;
    field.views[0].browse = Some(BrowseFieldPage {
        position: oteryn_session::item_view::WorldTilePosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        index: 0,
        total: 2,
    });
    replace_views(&outbox, field);
    app.update();
    assert!(has_text(&mut app, "Browse Field (2/2)"));
    assert!(ground_up(&mut app, 0).is_none(), "a browse field has no Up");
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.browse_field_panel();
    assert!(!panel.pending);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Field contents updated by the server")
    );
    Ok(())
}

#[test]
fn carried_container_without_parent_has_no_ground_up() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_browse_field_supported();
    let inventory = oteryn_session::CharacterInventory {
        main_backpack: Some(tests::entry(10, 1)?),
        entries: vec![],
        equipment: vec![],
    };
    PlayLink::bevy_items_fixture(&outbox, Some(inventory), Some(views()?), true, true);
    app.update();
    assert!(
        ground_up(&mut app, 0).is_none(),
        "the main backpack is carried"
    );
    assert!(
        ground_up(&mut app, 1).is_some(),
        "the other view lies on the ground"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn item_menu(app: &mut App, handle: u64, write: bool) -> Result {
    let slot = app
        .world_mut()
        .query::<&Slot>()
        .iter(app.world())
        .find(|slot| slot.item.is_some_and(|item| item.handle.get() == handle))
        .copied()
        .ok_or("item slot")?;
    app.world_mut().resource_mut::<InventoryUi>().menu = Some((slot, Vec2::ZERO));
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| match control {
            Control::Write(_) if write => Some(entity),
            Control::Timed(_) if !write => Some(entity),
            _ => None,
        })
        .ok_or("item menu entry")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn property_form(link: &PlayLink, title: &str) -> Result<crate::play::FeatureForm> {
    crate::play::FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(link) == title)
        .ok_or_else(|| format!("missing form: {title}").into())
}
fn property_command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand> {
    match commands.try_recv()? {
        PlayCommand::Feature(command) => Ok(command),
        _ => Err("feature command".into()),
    }
}
fn property_reply(payload: Vec<u8>) -> oteryn_client_runtime::raw_feature::GatedCommandResult {
    oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        payload,
    }
}

#[test]
fn item_menu_write_opens_the_writing_window_and_saves_the_item_text() -> Result {
    use oteryn_session::property_services::mail::{MailIntent, decode_intent};
    let (mut app, mut commands, outbox) = app()?;
    assert!(
        item_menu(&mut app, 11, true).is_err(),
        "no Write before the session admits capability 115"
    );
    outbox.bevy_property_items_selected();
    app.world_mut().resource_mut::<InventoryUi>().menu = None;
    app.update();
    item_menu(&mut app, 11, true)?;
    assert!(app.world().resource::<InventoryUi>().menu.is_none());
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let window = property_form(link, "Writing and mail")?;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert_eq!(view.fields[0], "11");
    link.feature_form_submit(window, 0, vec!["11".into(), String::new(), "hello".into()]);
    let write = property_command(&mut commands)?;
    assert_eq!(write.kind, 124);
    assert_eq!(
        decode_intent(&write.payload).map_err(|e| format!("{e:?}"))?,
        MailIntent::WriteText {
            item_handle: 11,
            text: "hello".into(),
        }
    );
    outbox.bevy_properties_result(&write, &property_reply(vec![2]));
    let text = link.feature_form_view(window).text;
    assert!(text.contains("Item text saved"), "{text}");
    Ok(())
}

#[test]
fn item_menu_timed_item_inspects_the_item_then_uses_the_shown_revision() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    app.update();
    item_menu(&mut app, 11, false)?;
    let inspect = property_command(&mut commands)?;
    assert_eq!(inspect.kind, 38);
    let intent = wire::decode_timed_item_intent(&inspect.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.action, wire::TimedAction::Inspect);
    assert_eq!(intent.target.get(), 11);
    assert_eq!(intent.expected_revision, 0);
    let reply = wire::encode_timed_item_result(wire::TimedItemResult {
        disposition: wire::TimedDisposition::Ok,
        revision: 5,
        charges: Some(8),
        remaining_ms: Some(1000),
        retired: false,
        fee_gold: 0,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.bevy_properties_result(&inspect, &property_reply(reply));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let window = property_form(link, "Timed items")?;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert_eq!(view.fields[..2], ["11".to_string(), "5".to_string()]);
    assert!(view.text.contains("Timed item updated"), "{}", view.text);
    link.feature_form_submit(window, 1, view.fields);
    let used = property_command(&mut commands)?;
    let intent = wire::decode_timed_item_intent(&used.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (intent.action, intent.target.get(), intent.expected_revision),
        (wire::TimedAction::Use, 11, 5)
    );
    Ok(())
}

#[test]
fn item_menu_write_while_waiting_targets_the_newly_chosen_item_after_the_reply() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    app.update();
    item_menu(&mut app, 11, true)?;
    let window = {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let window = property_form(link, "Writing and mail")?;
        link.feature_form_submit(window, 0, vec!["11".into(), String::new(), "first".into()]);
        link.feature_form_close(window);
        window
    };
    let first = property_command(&mut commands)?;
    // The real inventory menu reopens the pending form for another item.
    item_menu(&mut app, 12, true)?;
    assert!(
        commands.try_recv().is_err(),
        "nothing is sent while waiting"
    );
    outbox.bevy_properties_result(&first, &property_reply(vec![2]));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert_eq!(view.fields[0], "12");
    assert!(
        view.fields[2].is_empty(),
        "item 11's text must not carry over"
    );
    // Item 12's text is unknown without capability 511: an empty Write would erase it.
    link.feature_form_submit(window, 0, view.fields);
    assert!(commands.try_recv().is_err(), "no blank replacement is sent");
    let text = link.feature_form_view(window).text;
    assert!(text.contains("an empty text would erase it"), "{text}");
    Ok(())
}

fn inscription_reply(text: &str, writable: bool) -> Result<Vec<u8>> {
    use oteryn_client_ui::item_inscription::Reply;
    Ok(Reply {
        text: text.into(),
        max_length: 1999,
        writable,
    }
    .encode()
    .ok_or("inscription reply")?)
}

#[test]
fn item_menu_write_opens_the_writing_window_with_the_items_current_text() -> Result {
    use oteryn_client_ui::item_inscription::{COMMAND_TYPE_ITEM_INSCRIPTION_READ, Request};
    use oteryn_session::property_services::mail::{MailIntent, decode_intent};
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    outbox.bevy_item_inscription_selected();
    app.update();
    item_menu(&mut app, 11, true)?;
    let read = property_command(&mut commands)?;
    assert_eq!(read.kind, COMMAND_TYPE_ITEM_INSCRIPTION_READ);
    assert_eq!(
        Request::decode(&read.payload),
        Some(Request { item_handle: 11 })
    );
    let window = {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let window = property_form(link, "Writing and mail")?;
        let view = link.feature_form_view(window);
        assert!(view.open && view.pending, "the editor waits for the text");
        link.feature_form_submit(window, 0, vec!["11".into(), String::new(), String::new()]);
        assert!(
            commands.try_recv().is_err(),
            "no Write before the text arrives"
        );
        window
    };
    outbox.bevy_properties_result(
        &read,
        &property_reply(inscription_reply("Old inscription", true)?),
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let view = link.feature_form_view(window);
    assert!(!view.pending);
    assert_eq!(view.fields[2], "Old inscription");
    let mut fields = view.fields;
    fields[2].push_str(", and a new line");
    link.feature_form_submit(window, 0, fields);
    let write = property_command(&mut commands)?;
    assert_eq!(
        decode_intent(&write.payload).map_err(|e| format!("{e:?}"))?,
        MailIntent::WriteText {
            item_handle: 11,
            text: "Old inscription, and a new line".into(),
        }
    );
    Ok(())
}

#[test]
fn item_menu_write_never_replaces_a_text_it_could_not_read_or_may_not_write() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    outbox.bevy_item_inscription_selected();
    app.update();
    item_menu(&mut app, 11, true)?;
    let read = property_command(&mut commands)?;
    let rejected = oteryn_client_runtime::raw_feature::GatedCommandResult {
        status: oteryn_client_runtime::raw_feature::CommandStatus::Rejected,
        ..property_reply(Vec::new())
    };
    outbox.bevy_properties_result(&read, &rejected);
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let window = property_form(link, "Writing and mail")?;
        let view = link.feature_form_view(window);
        assert!(view.text.contains("refused to read"), "{}", view.text);
        link.feature_form_submit(window, 0, vec!["11".into(), String::new(), String::new()]);
        assert!(
            commands.try_recv().is_err(),
            "a refused read blocks the Write"
        );
    }
    item_menu(&mut app, 12, true)?;
    let read = property_command(&mut commands)?;
    outbox.bevy_properties_result(
        &read,
        &property_reply(inscription_reply("Sealed text", false)?),
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let window = property_form(link, "Writing and mail")?;
    let view = link.feature_form_view(window);
    assert_eq!(view.fields[2], "Sealed text", "a read-only text is shown");
    assert!(view.text.contains("cannot be written on"), "{}", view.text);
    link.feature_form_submit(window, 0, view.fields);
    assert!(
        commands.try_recv().is_err(),
        "a read-only text is not rewritten"
    );
    Ok(())
}

fn timed_ok(revision: u64) -> Result<Vec<u8>> {
    use oteryn_client_runtime::timed_item as wire;
    Ok(wire::encode_timed_item_result(wire::TimedItemResult {
        disposition: wire::TimedDisposition::Ok,
        revision,
        charges: Some(8),
        remaining_ms: Some(1000),
        retired: false,
        fee_gold: 0,
    })
    .map_err(|e| format!("{e:?}"))?)
}

#[test]
fn item_menu_timed_item_while_waiting_inspects_the_newly_chosen_item_after_the_reply() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    app.update();
    item_menu(&mut app, 11, false)?;
    let inspect = property_command(&mut commands)?;
    outbox.bevy_properties_result(&inspect, &property_reply(timed_ok(5)?));
    let window = {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let window = property_form(link, "Timed items")?;
        let fields = link.feature_form_view(window).fields;
        link.feature_form_submit(window, 1, fields);
        link.feature_form_close(window);
        window
    };
    let used = property_command(&mut commands)?;
    item_menu(&mut app, 12, false)?;
    assert!(
        commands.try_recv().is_err(),
        "nothing is sent while waiting"
    );
    outbox.bevy_properties_result(&used, &property_reply(timed_ok(6)?));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert_eq!(view.fields[..2], ["12".to_string(), "0".to_string()]);
    // The form window's per-frame upkeep sends the deferred inspection.
    link.feature_form_tick(window);
    let next = property_command(&mut commands)?;
    let intent = wire::decode_timed_item_intent(&next.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (intent.action, intent.target.get(), intent.expected_revision),
        (wire::TimedAction::Inspect, 12, 0)
    );
    Ok(())
}

#[test]
fn item_menu_timed_item_for_another_item_drops_the_previous_details_on_rejection() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    app.update();
    item_menu(&mut app, 11, false)?;
    let inspect = property_command(&mut commands)?;
    outbox.bevy_properties_result(&inspect, &property_reply(timed_ok(5)?));
    let window = {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let window = property_form(link, "Timed items")?;
        assert!(link.feature_form_view(window).text.contains("Charges"));
        link.feature_form_close(window);
        window
    };
    item_menu(&mut app, 12, false)?;
    let other = property_command(&mut commands)?;
    let mut rejected = property_reply(
        wire::encode_timed_item_result(wire::TimedItemResult::refused(
            wire::TimedDisposition::Rejected,
        ))
        .map_err(|e| format!("{e:?}"))?,
    );
    rejected.status = oteryn_client_runtime::raw_feature::CommandStatus::Rejected;
    outbox.bevy_properties_result(&other, &rejected);
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let view = link.feature_form_view(window);
    assert_eq!(view.fields[0], "12");
    assert!(
        !view.text.contains("Charges") && !view.text.contains("Revision 5"),
        "item 11's details must not describe item 12: {}",
        view.text
    );
    Ok(())
}

#[test]
fn item_context_menu_near_the_bottom_keeps_every_row_inside_the_viewport() -> Result {
    use bevy::window::PrimaryWindow;
    let (mut app, _commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    let window = Window::default();
    let height = window.height();
    app.world_mut().spawn((window, PrimaryWindow));
    app.update();
    let slot = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find(|(_, slot)| slot.item.is_some_and(|item| item.handle.get() == 11))
        .map(|(entity, _)| entity)
        .ok_or("item slot")?;
    app.world_mut()
        .trigger(bevy::picking::events::PointerClick {
            entity: slot,
            pointer: bevy::picking::events::Pointer::new(
                bevy::picking::pointer::PointerId::Mouse,
                bevy::picking::pointer::Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 1280,
                        height: 720,
                    },
                    position: Vec2::new(10., height - 2.),
                },
            ),
            button: bevy::picking::pointer::PointerButton::Secondary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        });
    app.update();
    let (_, position) = app
        .world()
        .resource::<InventoryUi>()
        .menu
        .ok_or("the secondary click opens the menu")?;
    let rows = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .filter(|control| {
            matches!(
                control,
                Control::Open(_)
                    | Control::Context(..)
                    | Control::Write(_)
                    | Control::Timed(_)
                    | Control::Cancel { prompt: None, .. }
            )
        })
        .count();
    assert_eq!(
        rows, 7,
        "Write ... and Timed item ... are shown; Tibia has no Cancel row"
    );
    // Seven 16-pixel entries, six 3-pixel gaps and 4-pixel popup padding on both edges.
    let bottom = position.y + 7. * 16. + 6. * 3. + 8.;
    assert!(bottom <= height, "menu ends at {bottom}, viewport {height}");
    Ok(())
}
