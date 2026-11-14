use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture()
-> Result<(App, tokio::sync::mpsc::UnboundedReceiver<PlayCommand>), Box<dyn std::error::Error>> {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    link.container_sorting_test_initialize()?;
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
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
    Ok((app, commands))
}
fn owner_snapshot(app: &App) -> Result<Snapshot, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .container_sorting_snapshot())
}
fn choose(app: &mut App, kind: Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn container_sorting_presenter_shows_server_data_only() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let snapshot = owner_snapshot(&app)?;
    let texts: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Server sort summary"))
    );
    assert!(
        texts
            .iter()
            .any(|text| text == "Open container view (0-15): 3")
    );
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .any(|control| control.kind == Action::Owner(Kind::Edit(0, "3".into())))
    );
    assert!(
        !app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .any(|control| control.kind == Action::Owner(Kind::Edit(0, "4".into())))
    );
    assert_eq!(owner_snapshot(&app)?, snapshot);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn container_sorting_presenter_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let snapshot = owner_snapshot(&app)?;
    choose(&mut app, Action::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert_eq!(owner_snapshot(&app)?, snapshot);
    choose(&mut app, Action::Owner(Kind::Sort { confirmed: true }))?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature".into());
    };
    assert_eq!(command.payload, [3, 2, 1, 0]);
    assert_eq!(owner_snapshot(&app)?.views, snapshot.views);
    assert_eq!(owner_snapshot(&app)?.panel.summary, snapshot.panel.summary);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn container_sorting_presenter_rejects_modal_and_replaced_session() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    choose(&mut app, Action::Confirm)?;
    let (entity, stale) = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == Action::Owner(Kind::Sort { confirmed: true }))
        .map(|(entity, control)| (entity, control.clone()))
        .ok_or("sort")?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    let (link, mut replacement_commands, _) = play_channel();
    link.container_sorting_test_initialize()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    app.world_mut()
        .resource_scope(|world, mut state: Mut<State>| {
            perform(world.non_send::<HostOwner>(), &mut state, &stale);
        });
    assert!(replacement_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn container_sorting_toolbar_and_shortcut_open_and_close() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let (mut app, mut commands) = fixture()?;
    choose(&mut app, Action::Owner(Kind::Close))?;
    assert!(!owner_snapshot(&app)?.panel.open);
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, crate::bevy_shell::ui::Control::ContainerSorting).then_some(entity)
        })
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(owner_snapshot(&app)?.panel.open);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::AltLeft);
    let event = || KeyboardInput {
        key_code: KeyCode::F9,
        logical_key: Key::F9,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(event());
    app.update();
    assert!(!owner_snapshot(&app)?.panel.open);
    app.update();
    app.world_mut().write_message(event());
    app.update();
    assert!(owner_snapshot(&app)?.panel.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn container_sorting_keyboard_rejects_replaced_session_and_modal() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let (mut app, mut commands) = fixture()?;
    let (link, mut replacement_commands, _) = play_channel();
    link.container_sorting_test_initialize()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let escape = || KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(escape());
    app.update();
    assert!(owner_snapshot(&app)?.panel.open);
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    assert!(app.world().non_send::<HostOwner>().modal_active());
    app.world_mut().write_message(escape());
    app.update();
    assert!(owner_snapshot(&app)?.panel.open);
    assert!(commands.try_recv().is_err());
    assert!(replacement_commands.try_recv().is_err());
    Ok(())
}
