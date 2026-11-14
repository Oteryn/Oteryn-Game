use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, text::EditableText};
use oteryn_client_ui::exiva_settings::{ExivaPanel, PlayerList};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn data() -> PlayerList {
    PlayerList {
        modes: 10,
        players: vec![([3; 16], "Al".into()), ([4; 16], "Beth".into())],
    }
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_exiva_initialize(true);
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
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = control(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("Exiva control absent".into())
}
fn panel(app: &App) -> Result<ExivaPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .exiva_panel())
}
fn wire(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match commands.try_recv()? {
        PlayCommand::Exiva(payload) => Ok(payload),
        _ => Err("wrong command".into()),
    }
}
fn publish(outbox: &Outbox, list: &PlayerList) -> TestResult {
    outbox.bevy_exiva_response(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: list.encode().ok_or("encode")?,
    });
    Ok(())
}
fn loaded(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> TestResult {
    open_exiva_settings(app.world().non_send::<HostOwner>());
    assert_eq!(wire(commands)?, vec![5]);
    publish(outbox, &data())?;
    app.update();
    Ok(())
}
#[test]
fn exiva_settings_window_shows_server_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    open_exiva_settings(app.world().non_send::<HostOwner>());
    assert_eq!(wire(&mut commands)?, vec![5]);
    app.update();
    assert!(panel(&app)?.list.is_none());
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Al")
    );
    publish(&outbox, &data())?;
    app.update();
    for name in ["Al", "Beth"] {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == name)
        );
    }
    assert_eq!(panel(&app)?.list, Some(data()));
    for bit in 0..5 {
        let entity = control(&mut app, Kind::Toggle(bit))?;
        assert_eq!(
            app.world().get::<bevy::ui::Checked>(entity).is_some(),
            data().modes & (1 << bit) != 0
        );
    }
    Ok(())
}
#[test]
fn exiva_settings_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    loaded(&mut app, &mut commands, &outbox)?;
    for bit in 0..5 {
        click(&mut app, Kind::Toggle(bit))?;
        assert_eq!(wire(&mut commands)?, vec![0, data().modes ^ (1 << bit)]);
        assert_eq!(panel(&app)?.list, Some(data()));
        click(&mut app, Kind::Toggle(bit))?;
        assert!(commands.try_recv().is_err());
        publish(&outbox, &data())?;
        app.update();
    }
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<NameField>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("Cara"));
    click(&mut app, Kind::Add)?;
    assert_eq!(wire(&mut commands)?, b"\x06Cara");
    assert_eq!(panel(&app)?.list, Some(data()));
    publish(&outbox, &data())?;
    app.update();
    click(&mut app, Kind::Select([3; 16]))?;
    app.update();
    click(&mut app, Kind::Remove)?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::CancelRemove)?;
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Remove)?;
    app.update();
    click(&mut app, Kind::ConfirmRemove)?;
    let mut remove = vec![2];
    remove.extend([3; 16]);
    assert_eq!(wire(&mut commands)?, remove);
    assert_eq!(panel(&app)?.list, Some(data()));
    assert!(commands.try_recv().is_err());
    let mut reply = data();
    reply.players.remove(0);
    publish(&outbox, &reply)?;
    app.update();
    assert_eq!(panel(&app)?.list, Some(reply));
    click(&mut app, Kind::Refresh)?;
    assert_eq!(wire(&mut commands)?, vec![5]);
    Ok(())
}
#[test]
fn exiva_settings_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    loaded(&mut app, &mut commands, &outbox)?;
    click(&mut app, Kind::Select([3; 16]))?;
    app.update();
    click(&mut app, Kind::Remove)?;
    app.update();
    let stale = control(&mut app, Kind::ConfirmRemove)?;
    let mut changed = data();
    changed.players[0].1 = "Alice".into();
    publish(&outbox, &changed)?;
    app.world_mut().trigger(Activate { entity: stale });
    assert!(commands.try_recv().is_err());
    app.update();
    let button = control(&mut app, Kind::Refresh)?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    let before = panel(&app)?;
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    let (link, mut replacement, outbox2) = play_channel();
    link.use_shared_frontend();
    outbox2.bevy_exiva_initialize(true);
    let old = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .exiva_snapshot();
    assert!(link.exiva_dispatch_bound(&link.exiva_snapshot(), ExivaControl::Open, false));
    assert_eq!(wire(&mut replacement)?, vec![5]);
    publish(&outbox2, &changed)?;
    assert_eq!(link.exiva_panel(), old.panel);
    assert!(!link.exiva_dispatch_bound(&old, ExivaControl::Refresh, false));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    app.world_mut().trigger(Activate { entity: button });
    assert!(replacement.try_recv().is_err());
    let replacement_before = panel(&app)?;
    publish(&outbox, &data())?;
    outbox.bevy_exiva_end();
    assert_eq!(panel(&app)?, replacement_before);
    app.update();
    click(&mut app, Kind::Refresh)?;
    assert_eq!(wire(&mut replacement)?, vec![5]);
    Ok(())
}
#[test]
fn exiva_settings_window_registered() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.id() == "shell.exiva_settings").then_some(entity))
        .ok_or("Exiva toolbar absent")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    assert_eq!(wire(&mut commands)?, vec![5]);
    publish(&outbox, &data())?;
    app.update();
    assert!(panel(&app)?.open);
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!panel(&app)?.open);
    app.world_mut().init_resource::<ButtonInput<KeyCode>>();
    for alt in [true, false] {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.press(KeyCode::ControlLeft);
        if alt {
            keys.press(KeyCode::AltLeft);
        }
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::KeyE,
                logical_key: bevy::input::keyboard::Key::Character("e".into()),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        assert!(!panel(&app)?.open, "Exiva must open only from its toolbar");
        assert!(commands.try_recv().is_err());
    }
    let registration = include_str!("../mod.rs");
    assert!(registration.contains("exiva_settings::install(app)"));
    assert!(registration.contains("pub(super) use exiva_settings::open_exiva_settings;"));
    assert!(!registration.contains("fn open_exiva_settings"));
    Ok(())
}

#[test]
fn exiva_settings_add_waits_for_resolved_text_once() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    loaded(&mut app, &mut commands, &outbox)?;
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<NameField>>()
        .single(app.world())?;
    let mut text = EditableText::new("Ca");
    text.queue_edit(bevy::text::TextEdit::Insert("ra".into()));
    app.world_mut().entity_mut(field).insert(text);
    click(&mut app, Kind::Add)?;
    click(&mut app, Kind::Add)?;
    app.update();
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("Cara"));
    app.update();
    assert_eq!(wire(&mut commands)?, b"\x06Cara");
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.list, Some(data()));
    Ok(())
}
