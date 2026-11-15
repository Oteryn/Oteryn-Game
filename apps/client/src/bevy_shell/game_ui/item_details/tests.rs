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

#[test]
fn inspect_multi_item_list() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    let mut inspected = reply("Server sword");
    let mut shield = reply("Server shield").items.remove(0);
    shield.attributes = vec![("Defense".into(), "20".into())];
    inspected.items.push(shield);
    response(&link, &inspected)?;
    app.update();
    let text = visible(&mut app);
    assert!(text.contains("Server sword"));
    assert!(text.contains("Server shield"));
    assert!(text.contains("Attack: 30"));
    assert!(!text.contains("Defense: 20"));
    let row = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| {
            (text.0.contains("Server shield")
                && app.world().get::<Control>(parent.parent()).is_some())
            .then_some(parent.parent())
        })
        .ok_or("shield selection row")?;
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(180), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    assert!(!visible(&mut app).contains("Defense: 20"));
    app.world_mut().entity_mut(higher).despawn();
    let old = app
        .world()
        .get::<Control>(row)
        .copied()
        .ok_or("row control")?;
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let text = visible(&mut app);
    assert!(text.contains("Defense: 20"));
    assert!(!text.contains("Attack: 30"));
    assert_eq!(
        app.world_mut()
            .query::<&EditableText>()
            .iter(app.world())
            .count(),
        1
    );
    assert!(commands.try_recv().is_err());
    // A newly confirmed result resets the local selection safely.
    inspect(&link, &mut commands, Request::Item(8.try_into()?))?;
    response(&link, &reply("Replacement"))?;
    app.update();
    assert!(visible(&mut app).contains("Attack: 30"));
    assert!(!visible(&mut app).contains("Server shield"));
    // The old row must not select an index in a different confirmed reply.
    let panel = app
        .world()
        .resource::<State>()
        .widgets
        .as_ref()
        .ok_or("widgets")?
        .panel;
    let stale = app
        .world_mut()
        .spawn((old, Node::default()))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(panel).add_child(stale);
    app.world_mut().trigger(Activate { entity: stale });
    app.update();
    assert!(visible(&mut app).contains("Attack: 30"));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn inspect_player_window() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(
        &link,
        &mut commands,
        Request::Player(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: [7; 16],
            generation: 1,
        }),
    )?;
    let mut equipment = reply("Player helmet");
    equipment.items[0].slot = 1;
    let mut backpack = reply("Player backpack").items.remove(0);
    backpack.slot = 10;
    equipment.items.push(backpack);
    response(&link, &equipment)?;
    app.update();
    let text = visible(&mut app);
    assert!(text.contains("Slot 1: Player helmet"));
    assert!(text.contains("Slot 10: Player backpack"));
    assert!(text.contains("Attack: 30"));
    assert_eq!(
        app.world_mut()
            .query::<&EditableText>()
            .iter(app.world())
            .count(),
        1
    );
    Ok(())
}
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
        appearance_id: 0,
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

#[test]
fn parity_a_inspection_cyclopedia_navigation_keeps_displayed_identity() -> TestResult {
    let (mut app, link, mut commands, _outbox) = app(true)?;
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    response(&link, &reply("Displayed sword"))?;
    app.update();
    let expected = link.inspection_panel().reply.ok_or("reply")?.items[0].definition_ref;
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| {
            (text.0 == "Cyclopedia" && app.world().get::<Control>(parent.parent()).is_some())
                .then_some(parent.parent())
        })
        .ok_or("Cyclopedia action")?;
    let control = *app.world().get::<Control>(button).ok_or("control")?;
    assert!(current(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        control.key
    ));
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        link.inspection_panel().catalogue_open,
        "{:?}",
        link.inspection_panel()
    );
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("catalogue request".into());
    };
    assert_eq!(
        oteryn_client_ui::inspection::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?,
        Request::Catalogue(expected)
    );
    assert!(link.inspection_panel().catalogue_open);
    assert!(!link.inspection_panel().open);
    Ok(())
}

#[test]
fn parity_a_inspection_proficiency_navigation_keeps_displayed_weapon() -> TestResult {
    let (mut app, link, mut commands, outbox) = app(true)?;
    outbox.bevy_proficiency_initialize();
    inspect(&link, &mut commands, Request::Item(7.try_into()?))?;
    let mut inspected = reply("Displayed sword");
    inspected.items[0]
        .attributes
        .push(("Weapon Type".into(), "sword".into()));
    response(&link, &inspected)?;
    app.update();
    let button = action(&mut app, Kind::Proficiency)?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(link.weapon_proficiency_panel().open);
    assert_eq!(link.weapon_proficiency_panel().weapon, 42);
    assert!(!link.inspection_panel().open);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(command) if command.capability == 195)
    );
    Ok(())
}
