use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::cast::Message;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.cast_chat_test_start()?;
    let mut owner = HostOwner::new(None)?;
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
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
    toggle(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("missing button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn set_field(app: &mut App, kind: Field, value: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (*field == kind).then_some(entity))
        .ok_or("missing field")?;
    app.world_mut()
        .entity_mut(entity)
        .insert(EditableText::new(value));
    Ok(())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn cast_chat_window_shows_server_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(text(&mut app).contains("Viewer-1: server line"));
    let expected = {
        let owner = app.world().non_send::<HostOwner>();
        oteryn_client_ui::cast_chat::panel_text(
            &owner.play.as_ref().ok_or("play")?.link.cast_chat_panel(),
        )
    };
    assert!(text(&mut app).contains(&expected));
    assert!(text(&mut app).contains(&"04".repeat(16)));
    assert!(commands.try_recv().is_err());
    outbox.cast_chat_test_end();
    app.update();
    assert!(!text(&mut app).contains("Viewer-1: server line"));
    Ok(())
}
#[test]
fn cast_chat_actions_are_bound_and_confirmed() -> TestResult {
    for action in 0..=4 {
        let (mut app, mut commands, outbox) = fixture()?;
        set_field(&mut app, Field::Reply, "caster reply")?;
        set_field(&mut app, Field::Subject, &"04".repeat(16))?;
        click(&mut app, Kind::Send(action))?;
        if action != 0 {
            assert!(commands.try_recv().is_err());
            assert!(text(&mut app).contains("Confirm"));
            click(&mut app, Kind::Confirm)?;
        }
        let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("wrong command".into());
        };
        let message = Message::decode(&command.payload).map_err(|e| format!("{e:?}"))?;
        if action == 0 {
            assert_eq!((command.capability, command.command_type), (291, 504));
            assert!(matches!(message, Message::Reply(ref text) if text == "caster reply"));
        } else {
            assert_eq!((command.capability, command.command_type), (290, 505));
            assert!(
                matches!(message,Message::Moderate { revision:9, subject, action:a, key } if subject==[4;16] && a==action && key!=[0;16])
            );
        }
        click(&mut app, Kind::Send(action))?;
        assert!(commands.try_recv().is_err());
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.cast_chat_panel().pending);
        assert_eq!(link.cast_control_panel().revision, 9);
        assert_eq!(link.cast_chat_panel().subjects, vec![[4; 16]]);
        outbox.cast_chat_test_result()?;
        app.update();
        assert!(text(&mut app).contains("Cast action applied"));
    }
    Ok(())
}
#[test]
fn cast_chat_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    set_field(&mut app, Field::Subject, &"04".repeat(16))?;
    click(&mut app, Kind::Send(4))?;
    outbox.cast_chat_test_roster(vec![[5; 16]]);
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    // A binding cannot migrate to another session even with identical server data.
    let owner = app.world().non_send::<HostOwner>();
    let old = &owner.play.as_ref().ok_or("play")?.link;
    let bound = old.cast_chat_bound();
    let (new, mut new_commands, new_outbox) = play_channel();
    new_outbox.cast_chat_test_start()?;
    new_outbox.cast_chat_test_roster(vec![[5; 16]]);
    new.cast_chat_set_open(true);
    assert_eq!(
        oteryn_client_ui::cast_chat::panel_text(&bound.panel),
        oteryn_client_ui::cast_chat::panel_text(&new.cast_chat_panel())
    );
    assert!(!new.cast_chat_submit_bound(&bound, "reply", "", 0, true));
    let new_bound = new.cast_chat_bound();
    assert!(!new.cast_chat_submit_bound(&new_bound, "", &"04".repeat(16), 4, false));
    assert_eq!(new.cast_chat_panel().feedback, new_bound.panel.feedback);
    let modal = new.bevy_server_modal_fixture();
    assert!(!new.cast_chat_submit_bound(&new_bound, "reply", "", 0, true));
    assert!(!new.cast_chat_panel().pending);
    drop(modal);
    new.cast_chat_set_open(false);
    assert!(!new.cast_chat_submit_bound(&new_bound, "reply", "", 0, true));
    outbox.cast_chat_test_end();
    assert!(!old.cast_chat_submit_bound(&bound, "reply", "", 0, true));
    old.cast_chat_set_open(true);
    let ended = old.cast_chat_bound();
    assert!(!old.cast_chat_submit_bound(&ended, "reply", "", 0, true));
    assert_eq!(old.cast_chat_panel().feedback, ended.panel.feedback);
    assert!(commands.try_recv().is_err());
    assert!(new_commands.try_recv().is_err());
    let (mut modal_app, mut modal_commands, _) = fixture()?;
    modal_app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(200), Node::default()));
    click(&mut modal_app, Kind::Send(0))?;
    assert!(modal_commands.try_recv().is_err());
    assert!(
        !modal_app
            .world()
            .resource::<State>()
            .bound
            .as_ref()
            .ok_or("bound")?
            .panel
            .pending
    );
    Ok(())
}
#[test]
fn cast_chat_window_registered() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    assert_eq!(
        crate::bevy_shell::ui::Control::CastChat.id(),
        "shell.cast_chat"
    );
    click(&mut app, Kind::Close)?;
    assert!(
        !app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .any(|_| true)
    );
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(11)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), false);
    app.update();
    assert!(text(&mut app).contains("Live cast chat"));
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::CastChat).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), false);
    app.update();
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
