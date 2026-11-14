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
        .init_resource::<ButtonInput<KeyCode>>()
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
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::ShiftLeft);
    }
    let key = |key_code, logical_key| bevy::input::keyboard::KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut()
        .write_message(key(KeyCode::F8, bevy::input::keyboard::Key::F8));
    app.update();
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
        .find_map(|(e, c)| (c.action == MapControl::Request(wire_intent())).then_some(e))
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
fn wire_intent() -> oteryn_client_ui::cyclopedia_map::Intent {
    oteryn_client_ui::cyclopedia_map::Intent::Query
}
