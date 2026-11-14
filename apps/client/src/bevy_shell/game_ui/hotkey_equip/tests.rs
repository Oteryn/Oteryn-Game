use super::*;
use crate::play::PlayView;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<App, Box<dyn std::error::Error>> {
    let (link, commands, _) = crate::play::feature_0179_hotkey_equip::view::bound_tests::fixture();
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { link, view });
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
#[derive(Resource)]
struct TestCommands(tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>);
fn find(app: &mut App, action: Action) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == action).then_some(e))
        .ok_or_else(|| "control".into())
}
#[test]
fn hotkey_equip_window_shows_owner_data_only() -> Result {
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
        .hotkey_equip_panel();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == oteryn_client_ui::hotkey_equip::panel_text(&panel))
    );
    let items: Vec<_> = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .filter_map(|c| match c.action {
            Action::Owner(Kind::Assign(0, definition)) => Some(definition),
            _ => None,
        })
        .collect();
    assert_eq!(items, panel.items);
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter(|c| matches!(c.action, Action::Select(_)))
            .count(),
        panel.bindings.len()
    );
    Ok(())
}
#[test]
fn hotkey_equip_window_registered() -> Result {
    let mut app = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::HotkeyEquip).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    let close = find(&mut app, Action::Owner(Kind::Close))?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(8)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    Ok(())
}
#[test]
fn hotkey_equip_clear_requires_frozen_confirmation() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let assign = find(&mut app, Action::Owner(Kind::Assign(0, 256)))?;
    app.world_mut().trigger(Activate { entity: assign });
    app.update();
    let clear = find(&mut app, Action::Owner(Kind::Clear(0, false)))?;
    app.world_mut().trigger(Activate { entity: clear });
    app.update();
    assert_eq!(app.world().resource::<State>().confirm, Some(0));
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .hotkey_equip_panel()
            .bindings[0]
            .is_some()
    );
    let clear = find(&mut app, Action::Owner(Kind::Clear(0, false)))?;
    app.world_mut().trigger(Activate { entity: clear });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .hotkey_equip_panel()
            .bindings[0]
            .is_none()
    );
    Ok(())
}

#[test]
fn hotkey_equip_modal_rejects_actions_but_close_remains_available() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let equip = find(&mut app, Action::Definition)?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .hotkey_equip_panel();
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()));
    app.world_mut().trigger(Activate { entity: equip });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .hotkey_equip_panel(),
        before
    );
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    let close = find(&mut app, Action::Owner(Kind::Close))?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: close });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .hotkey_equip_panel()
            .open
    );
    Ok(())
}
#[test]
fn hotkey_equip_presenter_rejects_identical_replacement_session() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let assign = find(&mut app, Action::Owner(Kind::Assign(0, 256)))?;
    let (link, mut commands, _) =
        crate::play::feature_0179_hotkey_equip::view::bound_tests::fixture();
    link.use_shared_frontend();
    assert!(link.hotkey_equip_open_bound());
    let before = link.hotkey_equip_panel();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    app.world_mut().trigger(Activate { entity: assign });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .hotkey_equip_panel(),
        before
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
