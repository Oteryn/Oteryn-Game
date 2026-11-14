use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::guildhall as wire;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, events) = play_channel();
    link.use_shared_frontend();
    events.guildhall_leadership_selected(true);
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
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, events))
}
fn panel(app: &App) -> Panel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|p| p.link.guildhall_panel())
        .unwrap_or_default()
}
fn open(app: &mut App) {
    toggle(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("missing guildhall button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn fields() -> [String; 3] {
    [
        "00000000-0000-7000-8000-000000000001".into(),
        "00000000-0000-7000-8000-000000000002".into(),
        "1".into(),
    ]
}
fn fill(app: &mut App) {
    let world = app.world_mut();
    for (field, mut text) in world.query::<(&Field, &mut EditableText)>().iter_mut(world) {
        *text = EditableText::new(fields()[field.0].clone());
    }
}
#[test]
fn guildhall_leadership_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, events) = fixture()?;
    open(&mut app);
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .map(|s| &s.1),
        Some(&panel(&app))
    );
    assert!(panel(&app).fields.iter().all(String::is_empty));
    assert!(panel(&app).summary.is_empty());
    assert!(commands.try_recv().is_err());
    fill(&mut app);
    click(&mut app, Kind::Transfer)?;
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    events.guildhall_leadership_reply(&command, 1)?;
    app.update();
    let expected = oteryn_client_ui::guildhall::panel_text(&panel(&app));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    assert!(panel(&app).summary.contains("2"));
    Ok(())
}
#[test]
fn guildhall_leadership_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, events) = fixture()?;
    open(&mut app);
    fill(&mut app);
    click(&mut app, Kind::Transfer)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app).pending);
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (99, 115));
    let request =
        wire::decode_guildhall_leadership(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.expected_revision, 1);
    assert_eq!(request.successor[15], 2);
    assert!(panel(&app).pending);
    assert!(panel(&app).summary.is_empty());
    let before = panel(&app);
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guildhall_leadership_transfer_bound(&before, fields(), true)
    );
    assert_eq!(panel(&app), before);
    assert!(commands.try_recv().is_err());
    events.guildhall_leadership_reply(&command, 2)?;
    app.update();
    assert!(panel(&app).summary.is_empty());
    assert!(panel(&app).feedback.contains("not permitted"));
    Ok(())
}
#[test]
fn guildhall_leadership_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, events) = fixture()?;
    open(&mut app);
    fill(&mut app);
    click(&mut app, Kind::Transfer)?;
    let confirm = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guildhall_edit(0, "changed target".into());
    let before = panel(&app);
    app.world_mut().trigger(Activate { entity: confirm });
    assert_eq!(panel(&app), before);
    assert!(commands.try_recv().is_err());
    app.update();
    fill(&mut app);
    click(&mut app, Kind::Transfer)?;
    let confirm = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    events.guildhall_leadership_end();
    app.world_mut().non_send_mut::<HostOwner>().poll([960, 640]);
    let (link, mut replacement, fresh) = play_channel();
    fresh.guildhall_leadership_selected(true);
    link.guildhall_open();
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    app.world_mut().non_send_mut::<HostOwner>().play =
        Some(crate::bevy_shell::owner::ActivePlay { view, link });
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(replacement.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn guildhall_leadership_window_registered() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            matches!(c, crate::bevy_shell::ui::Control::GuildhallLeadership).then_some(e)
        })
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app).open);
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app).open);
    let key = |key_code, logical_key| bevy::input::keyboard::KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    let chord = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(34)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord], false);
    app.update();
    assert!(panel(&app).open);
    app.world_mut()
        .write_message(key(KeyCode::Escape, bevy::input::keyboard::Key::Escape));
    app.update();
    assert!(!panel(&app).open);
    assert!(commands.try_recv().is_err());
    assert!(include_str!("../mod.rs").contains("guildhall_leadership::install(app)"));
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::None
    );
    Ok(())
}

#[test]
fn guildhall_leadership_rejected_confirmation_preserves_values_and_reports_feedback() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    open(&mut app);
    fill(&mut app);
    let world = app.world_mut();
    for (field, mut text) in world.query::<(&Field, &mut EditableText)>().iter_mut(world) {
        if field.0 == 2 {
            *text = EditableText::new("0");
        }
    }
    click(&mut app, Kind::Transfer)?;
    let expected = app.world().resource::<State>().confirmation.clone();
    let before = panel(&app);
    click(&mut app, Kind::Confirm)?;
    assert_eq!(app.world().resource::<State>().confirmation, expected);
    assert_eq!(panel(&app), before);
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Transfer was not submitted."))
    );
    click(&mut app, Kind::Cancel)?;
    fill(&mut app);
    click(&mut app, Kind::Transfer)?;
    click(&mut app, Kind::Confirm)?;
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(panel(&app).pending);
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Transfer was not submitted."))
    );
    Ok(())
}
