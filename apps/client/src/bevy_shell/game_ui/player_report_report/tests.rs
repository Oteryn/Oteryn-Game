use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture()
-> Result<(App, tokio::sync::mpsc::UnboundedReceiver<PlayCommand>), Box<dyn std::error::Error>> {
    let (link, commands, events) = play_channel();
    link.use_shared_frontend();
    events.bevy_player_report_selected();
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
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    Ok((app, commands))
}
fn button(app: &mut App, kind: Action) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("button missing".into())
}
fn click(app: &mut App, kind: Action) -> TestResult {
    let entity = button(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn player_report_window_renders_owner_fields_and_feedback() -> TestResult {
    let (mut app, _commands) = fixture()?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.player_report_edit(2, "Actual target".into());
    link.player_report_edit(4, "Actual evidence".into());
    let expected = link.player_report_panel();
    app.update();
    let mut observed = std::array::from_fn::<_, 5, _>(|_| String::new());
    for (field, text) in app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
    {
        observed[field.0] = text.value().to_string();
    }
    assert_eq!(observed, expected.fields);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == oteryn_client_ui::player_report::panel_text(&expected))
    );
    Ok(())
}
#[test]
fn player_report_bug_confirmation_and_modal_fence_reach_real_owner() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        *text = EditableText::new(match field.0 {
            0 => "3",
            4 => "Actual bug evidence",
            _ => "",
        });
    }
    click(&mut app, Action::ReviewBug)?;
    assert!(commands.try_recv().is_err());
    let confirm = button(&mut app, Action::ConfirmBug)?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    click(&mut app, Action::ConfirmBug)?;
    let PlayCommand::Feature(sent) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    let request = oteryn_client_runtime::player_report::decode_player_report_request(&sent.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.comment, "Actual bug evidence");
    assert_eq!(request.category, 3);
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .summary
            .is_empty()
    );
    Ok(())
}
#[test]
fn player_report_toolbar_and_shortcut_open_and_close_window() -> TestResult {
    let (mut app, _commands) = fixture()?;
    click(&mut app, Action::Owner(Kind::Close))?;
    assert!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .next()
            .is_none()
    );
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "shell.player_report").then_some(entity))
        .ok_or("toolbar missing")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .next()
            .is_some()
    );
    click(&mut app, Action::Owner(Kind::Close))?;
    // Exercise the actual Bevy shortcut system.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::AltLeft);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::F12,
            logical_key: bevy::input::keyboard::Key::F12,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .next()
            .is_some()
    );
    Ok(())
}
