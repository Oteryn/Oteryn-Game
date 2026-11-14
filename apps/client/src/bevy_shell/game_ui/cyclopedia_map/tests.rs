use super::*;
use crate::play::{PlayView, play_channel};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    outbox.bevy_cyclopedia_map_select();
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn show(app: &mut App) -> TestResult {
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::CyclopediaMap).then_some(e))
        .ok_or("Cyclopedia toolbar missing")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    Ok(())
}
#[test]
fn cyclopedia_map_window_registered() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    show(&mut app)?;
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == MapControl::Close).then_some(e))
        .ok_or("close absent")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let key = |key_code, logical_key| bevy::input::keyboard::KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    owner_shortcut(&mut app, false)?;
    assert!(app.world().resource::<State>().root.is_some());
    owner_shortcut(&mut app, false)?;
    assert!(app.world().resource::<State>().root.is_none());
    owner_shortcut(&mut app, false)?;
    assert!(app.world().resource::<State>().root.is_some());
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut()
        .write_message(key(KeyCode::Escape, bevy::input::keyboard::Key::Escape));
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world_mut()
        .write_message(key(KeyCode::Escape, bevy::input::keyboard::Key::Escape));
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn cyclopedia_map_window_shows_server_data_only() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::{
        cyclopedia_map as wire,
        raw_feature::{CommandStatus, GatedCommandResult},
    };
    let (mut app, mut commands, outbox) = fixture()?;
    show(&mut app)?;
    assert!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .is_some_and(|(_, s)| s.panel.map.is_none() && s.panel.atlas.is_none())
    );
    let query = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            (c.action == MapControl::Request(oteryn_client_ui::cyclopedia_map::Intent::Query))
                .then_some(e)
        })
        .ok_or("query absent")?;
    app.world_mut().trigger(Activate { entity: query });
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("query absent".into());
    };
    let PlayCommand::Feature(speed) = commands.try_recv()? else {
        return Err("speed query absent".into());
    };
    let confirmed = wire::MapResult {
        code: 0,
        revision: 77,
        discovered: false,
        found: 4,
        area_percent: 51,
        pool: 12,
        epoch: 9,
        improved: false,
        visible: vec![(123, 456, -7)],
    };
    assert!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .is_some_and(|(_, s)| s.panel.map.is_none())
    );
    let reply = |payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    };
    outbox.bevy_cyclopedia_map_reply(
        &command,
        &reply(wire::encode_result(&confirmed).map_err(|e| format!("{e:?}"))?),
    );
    outbox.bevy_cyclopedia_map_reply(&speed, &reply(3u16.to_le_bytes().to_vec()));
    app.update();
    let snapshot = app
        .world()
        .resource::<State>()
        .snapshot
        .as_ref()
        .ok_or("snapshot absent")?;
    assert_eq!(snapshot.1.panel.map.as_ref(), Some(&confirmed));
    let expected = snapshot.1.text.clone();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert!(expected.contains("Revision 77"));
    assert!(expected.contains("Found 4/7"));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn cyclopedia_map_toolbar_closes_window() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    show(&mut app)?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::CyclopediaMap).then_some(e))
        .ok_or("toolbar absent")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn choose(app: &mut App, action: &MapControl, confirmed: bool) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == *action && c.confirmed == confirmed).then_some(e))
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn cyclopedia_map_actions_are_bound_and_confirmed() -> TestResult {
    use oteryn_client_ui::cyclopedia_map::Intent;
    let (mut app, mut commands, _) = fixture()?;
    show(&mut app)?;
    {
        let mut fields = app
            .world_mut()
            .query_filtered::<&mut bevy::text::EditableText, With<widgets::Donation>>();
        *fields.single_mut(app.world_mut())? = bevy::text::EditableText::new("100");
    }
    choose(&mut app, &MapControl::Pois, false)?;
    choose(&mut app, &MapControl::Donation(String::new()), false)?;
    choose(&mut app, &MapControl::Request(Intent::Donate), false)?;
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Donate 100 gold?"))
    );
    choose(&mut app, &MapControl::Request(Intent::Donate), true)?;
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("donation absent".into());
    };
    let intent =
        oteryn_client_runtime::cyclopedia_map::decode_request(command.kind, &command.payload)
            .map_err(|e| format!("{e:?}"))?;
    assert_eq!((intent.operation, intent.amount), (4, 100));
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .is_some_and(|(_, s)| s.panel.pending && s.panel.map.is_none())
    );
    Ok(())
}

fn owner_shortcut(app: &mut App, blocked: bool) -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(24)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
        repeat: false,
    };
    let events = [event];
    assert!(oteryn_client_ui::cyclopedia_map::open_key(&events));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&events, blocked);
    app.update();
    Ok(())
}

#[test]
fn cyclopedia_map_owner_shortcut_respects_input_gate() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    owner_shortcut(&mut app, true)?;
    assert!(app.world().resource::<State>().root.is_none());
    owner_shortcut(&mut app, false)?;
    assert!(app.world().resource::<State>().root.is_some());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn cyclopedia_map_invalid_marker_kinds_are_skipped() -> TestResult {
    use oteryn_client_runtime::cyclopedia_map::atlas::{Atlas, HEIGHT, Marker, Query, WIDTH};
    let (mut app, _commands, _) = fixture()?;
    let mut snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play absent")?
        .link
        .cyclopedia_map_snapshot();
    snapshot.panel.atlas_layers = 0x81;
    snapshot.panel.atlas = Some(Atlas {
        query: Query {
            floor: -7,
            x: 100,
            y: 100,
            zoom: 0,
            page: 0,
        },
        digest: [0; 32],
        floors: 1 << 7,
        progress: None,
        cells: vec![0; WIDTH * HEIGHT],
        areas: Vec::new(),
        marker_total: 6,
        markers: [
            (0, "invalid zero"),
            (9, "invalid nine"),
            (255, "invalid max"),
            (1, "visible NPC"),
            (2, "hidden passage"),
            (8, "visible high bit"),
        ]
        .into_iter()
        .map(|(kind, label)| Marker {
            kind,
            x: 101,
            y: 101,
            label: label.into(),
        })
        .collect(),
    });
    app.world_mut().resource_scope(|world, skins: Mut<Skins>| {
        widgets::build(&mut world.commands(), &skins, 0, &snapshot, None, None);
    });
    app.world_mut().flush();
    for label in [
        "invalid zero",
        "invalid nine",
        "invalid max",
        "hidden passage",
    ] {
        assert!(
            !app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains(label))
        );
    }
    for label in ["visible NPC", "visible high bit"] {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains(label))
        );
    }
    assert_eq!(
        app.world_mut()
            .query::<&Node>()
            .iter(app.world())
            .filter(|node| node.width == px(8.) && node.height == px(8.))
            .count(),
        2
    );
    Ok(())
}
