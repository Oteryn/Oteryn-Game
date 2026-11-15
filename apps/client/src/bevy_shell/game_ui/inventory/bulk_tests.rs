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
    let destination = ItemMoveDestination::Container(20.try_into()?);
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
