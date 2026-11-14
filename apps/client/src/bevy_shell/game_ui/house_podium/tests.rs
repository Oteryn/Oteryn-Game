#[derive(Resource)]
struct TestCommands(tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>);
use super::*;
use crate::play::{PlayView, play_channel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<App, Box<dyn std::error::Error>> {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .insert_resource(TestCommands(commands))
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok(app)
}
#[test]
fn house_podium_window_shows_server_data_only() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_podium_panel();
    let expected = oteryn_client_ui::house_podium::panel_text(&panel);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert_eq!(
        app.world_mut().query::<&Field>().iter(app.world()).count(),
        4
    );
    assert!(
        !app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .any(|c| matches!(c.kind, Kind::Toggle(_)))
    );
    Ok(())
}
#[test]
fn house_podium_window_registered() -> Result {
    let mut app = fixture()?;
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::HousePodium).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Close).then_some(e))
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(19)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    Ok(())
}

#[test]
fn house_podium_confirmation_modal_and_replaced_session() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let find = |app: &mut App, kind: Kind| {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(e, c)| (c.kind == kind).then_some(e))
    };
    let set = find(&mut app, Kind::Request(Intent::Set)).ok_or("set")?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_podium_panel();
    app.world_mut().trigger(Activate { entity: set });
    assert!(app.world().resource::<State>().confirm.is_some());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .house_podium_panel(),
        before
    );
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    let close = find(&mut app, Kind::Close).ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .house_podium_panel(),
        before
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let stale = find(&mut app, Kind::Request(Intent::Refresh)).ok_or("refresh")?;
    let old = app
        .world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .take()
        .ok_or("old play")?;
    let snapshot = old.link.house_podium_panel();
    app.world_mut().trigger(Activate { entity: stale });
    assert_eq!(old.link.house_podium_panel(), snapshot);
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    Ok(())
}
