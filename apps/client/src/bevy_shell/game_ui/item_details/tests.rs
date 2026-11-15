use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    input::keyboard::{Key as LogicalKey, KeyboardInput},
    text::EditableText,
};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
use oteryn_client_ui::inspection::{InspectedItem, Reply, Request, encode_reply, encode_request};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn app(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    link.bevy_item_details_selected(selected);
    link.inspection_set_open(true);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view,
        link: link.bevy_item_details_link(),
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
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
    Ok((app, link, commands, outbox))
}
fn reply(name: &str) -> Reply {
    Reply {
        status: 1,
        requests: vec![],
        catalogue: None,
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: 42,
            count: 2,
            tier: 1,
            name: name.into(),
            attributes: vec![
                ("Attack".into(), "30".into()),
                ("Weight".into(), "35.00 oz".into()),
            ],
        }],
    }
}
fn response(link: &PlayLink, reply: &Reply) -> TestResult {
    link.bevy_item_details_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_reply(reply).map_err(|error| format!("inspection reply: {error:?}"))?,
    });
    Ok(())
}
fn inspect(
    link: &PlayLink,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    request: Request,
) -> TestResult {
    assert!(link.inspect(request));
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(ServerMechanicCommand {
            capability: 132,
            command_type: 153,
            payload: encode_request(request)
                .map_err(|error| format!("inspection request: {error:?}"))?,
        })
    );
    Ok(())
}
fn action(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("inspection action missing".into())
}
fn visible(app: &mut App) -> String {
    let mut text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    text.extend(
        app.world_mut()
            .query::<&EditableText>()
            .iter(app.world())
            .map(|text| text.value().to_string()),
    );
    text.join("\n")
}
#[test]
fn confirmed_response_is_rendered_and_refresh_reuses_the_existing_request_once() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    let request = Request::Item(7.try_into()?);
    inspect(&link, &mut commands, request)?;
    response(&link, &reply("Server sword"))?;
    app.update();
    let text = visible(&mut app);
    assert!(text.contains("Server sword"));
    assert!(text.contains("Attack: 30"));
    assert!(text.contains("Weight: 35.00 oz"));
    let refresh = action(&mut app, Kind::Refresh)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(ServerMechanicCommand {
            capability: 132,
            command_type: 153,
            payload: encode_request(request).map_err(|error| format!("request: {error:?}"))?
        })
    );
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    assert!(link.inspection_panel().pending);
    Ok(())
}
#[test]
fn close_changes_the_canonical_visibility_without_canceling_or_reopening_a_pending_request()
-> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    app.update();
    assert!(visible(&mut app).contains("Waiting for the server"));
    let close = action(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!link.inspection_panel().open);
    assert!(link.inspection_panel().pending);
    app.update();
    response(&link, &reply("Closed reply"))?;
    app.update();
    assert!(!link.inspection_panel().open);
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn unavailable_and_refused_inspection_never_display_unconfirmed_attributes() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(false)?;
    assert!(visible(&mut app).contains("unavailable"));
    let refresh = action(&mut app, Kind::Refresh)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    link.bevy_item_details_selected(true);
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    response(&link, &reply("Must not appear"))?;
    app.update();
    assert!(visible(&mut app).contains("Must not appear"));
    inspect(&link, &mut commands, Request::Item(8.try_into()?))?;
    response(&link, &Reply::refused(4))?;
    app.update();
    let text = visible(&mut app);
    assert!(text.contains("Too far away"));
    assert!(!text.contains("Must not appear"));
    Ok(())
}
#[test]
fn stale_model_revision_and_closed_session_cannot_dispatch_actions() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    response(&link, &reply("First"))?;
    app.update();
    let close = action(&mut app, Kind::Close)?;
    let old = app
        .world()
        .get::<Control>(close)
        .copied()
        .ok_or("old control")?;
    inspect(&link, &mut commands, Request::Item(8.try_into()?))?;
    response(&link, &reply("Second"))?;
    app.update();
    let panel = app
        .world()
        .resource::<State>()
        .widgets
        .as_ref()
        .ok_or("widgets")?
        .panel;
    let phantom = app
        .world_mut()
        .spawn((old, Node::default()))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(panel).add_child(phantom);
    app.world_mut().trigger(Activate { entity: phantom });
    assert!(link.inspection_panel().open);
    assert!(commands.try_recv().is_err());
    let refresh = action(&mut app, Kind::Refresh)?;
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn enter_escape_respect_the_top_modal_and_do_not_repeat_permission_actions() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(
        &link,
        &mut commands,
        Request::Permission {
            action: 4,
            target: None,
        },
    )?;
    response(&link, &Reply::refused(8))?;
    app.update();
    let refresh = action(&mut app, Kind::Refresh)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(180), Node::default()))
        .id();
    let escape = || KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: LogicalKey::Escape,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(escape());
    app.update();
    assert!(link.inspection_panel().open);
    app.world_mut().entity_mut(higher).despawn();
    app.world_mut().write_message(escape());
    app.update();
    assert!(!link.inspection_panel().open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn clipboard_payload_contains_only_accepted_item_information() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    response(&link, &reply("Server sword"))?;
    app.update();
    let text = copy_text(&link.inspection_panel()).ok_or("copy text")?;
    assert!(text.contains("Server sword"));
    assert!(text.contains("Attack: 30"));
    assert!(!text.contains("definition_ref"));
    let copy = action(&mut app, Kind::Copy)?;
    app.world_mut().trigger(Activate { entity: copy });
    assert!(
        !app.world()
            .resource::<State>()
            .clipboard_feedback
            .is_empty()
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn inspect_selected_uses_the_real_inventory_selection_and_bounded_owner_request() -> TestResult {
    let (mut app, link, mut commands, outbox) = app(true)?;
    let item = oteryn_session::ItemEntry {
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 2.try_into()?,
        sub_type: 0,
    };
    PlayLink::bevy_items_fixture(
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
    let inspect = action(&mut app, Kind::Inspect)?;
    app.world_mut().trigger(Activate { entity: inspect });
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(ServerMechanicCommand {
            capability: 132,
            command_type: 153,
            payload: encode_request(Request::Item(item.handle))
                .map_err(|error| format!("request: {error:?}"))?
        })
    );
    app.world_mut().trigger(Activate { entity: inspect });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn response_refresh_retains_source_geometry_and_drag_offset_and_enter_closes_only_its_modal()
-> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    let panel = app
        .world()
        .resource::<State>()
        .widgets
        .as_ref()
        .ok_or("widgets")?
        .panel;
    assert_eq!(
        app.world().get::<Node>(panel).ok_or("panel node")?.width,
        px(482.)
    );
    let translated = UiTransform::from_translation(Val2::px(20., 30.));
    app.world_mut().entity_mut(panel).insert(translated);
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    response(&link, &reply("Retained"))?;
    app.update();
    assert_eq!(
        app.world()
            .resource::<State>()
            .widgets
            .as_ref()
            .ok_or("widgets")?
            .panel,
        panel
    );
    assert_eq!(
        *app.world().get::<UiTransform>(panel).ok_or("transform")?,
        translated
    );
    let refresh = action(&mut app, Kind::Refresh)?;
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(refresh, bevy::input_focus::FocusCause::Navigated);
    let enter = || KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: LogicalKey::Enter,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(enter());
    app.update();
    assert!(link.inspection_panel().open);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(panel, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().write_message(enter());
    app.update();
    assert!(!link.inspection_panel().open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
