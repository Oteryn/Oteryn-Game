use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::{
    player_report as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_renderer::TileCoord;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn fixture() -> std::result::Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_bug_report_selected();
    link.bug_report_open();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn action(app: &mut App, kind: Kind) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == kind).then_some(e))
        .ok_or_else(|| "missing action".into())
}
fn edit(app: &mut App, field: usize, value: &str) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(e, f)| (f.0 == field).then_some(e))
        .ok_or("missing field")?;
    let mut editor = app
        .world_mut()
        .get_mut::<EditableText>(entity)
        .ok_or("editor")?;
    *editor = EditableText::new(value);
    app.update();
    Ok(())
}
#[test]
fn bug_report_send_calls_flow() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    let category = action(&mut app, Kind::Category(2))?;
    app.world_mut().trigger(Activate { entity: category });
    app.update();
    edit(&mut app, 1, "Opening the map closes the client")?;
    let review = action(&mut app, Kind::Review)?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    assert!(commands.try_recv().is_err());
    let send = action(&mut app, Kind::Send)?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(send)
            .is_none(),
        "{:?}",
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
    );
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing report".into());
    };
    let request =
        wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.category, 2);
    assert_eq!(request.comment, "Opening the map closes the client");
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .all(|t| t.0 != "Bug report sent.")
    );
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    assert!(commands.try_recv().is_err());
    outbox.bevy_bug_report_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_player_report_reply(&wire::PlayerReportReply {
                code: wire::PlayerReportCode::Accepted,
                report: 42,
                wait_seconds: 0,
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Bug report sent.")
    );
    Ok(())
}
#[test]
fn bug_report_empty_text_disabled() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    for text in ["", " \n "] {
        edit(&mut app, 1, text)?;
        let send = action(&mut app, Kind::Send)?;
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(send)
                .is_some()
        );
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn bug_report_edits_require_new_review_and_close_clears_focus() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    let category = action(&mut app, Kind::Category(3))?;
    app.world_mut().trigger(Activate { entity: category });
    app.update();
    edit(&mut app, 1, "The depot window closes")?;
    let review = action(&mut app, Kind::Review)?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    edit(&mut app, 1, "The depot window closes after walking")?;
    let send = action(&mut app, Kind::Send)?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(send)
            .is_some()
    );
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    assert!(commands.try_recv().is_err());
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Field>>()
        .iter(app.world())
        .next()
        .ok_or("field")?;
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(field, bevy::input_focus::FocusCause::Navigated);
    let close = action(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get()
            .is_none()
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .bug
            .open
    );
    Ok(())
}
#[test]
fn bug_report_map_coordinates_and_refusal_reach_window() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    edit(&mut app, 0, "100,200,7")?;
    edit(&mut app, 1, "This tile cannot be reached")?;
    let review = action(&mut app, Kind::Review)?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    let send = action(&mut app, Kind::Send)?;
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing map report".into());
    };
    let request =
        wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            request.category,
            request.map_x,
            request.map_y,
            request.map_floor
        ),
        (0, 100, 200, 7)
    );
    outbox.bevy_bug_report_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_player_report_reply(&wire::PlayerReportReply {
                code: wire::PlayerReportCode::RateLimited,
                report: 0,
                wait_seconds: 30,
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    let mut texts = app.world_mut().query::<&Text>();
    assert!(
        texts
            .iter(app.world())
            .any(|t| t.0.contains("Wait 30 seconds"))
    );
    assert!(texts.iter(app.world()).all(|t| t.0 != "Bug report sent."));
    Ok(())
}
#[test]
fn bug_report_deferred_edits_cannot_send_reviewed_old_text() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    let category = action(&mut app, Kind::Category(3))?;
    app.world_mut().trigger(Activate { entity: category });
    app.update();
    edit(&mut app, 1, "The map closes")?;
    let review = action(&mut app, Kind::Review)?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    let field = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(e, f)| (f.0 == 1).then_some(e))
        .ok_or("field")?;
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")?
        .pending_edits
        .push(bevy::text::TextEdit::Insert(" unexpectedly".into()));
    let send = action(&mut app, Kind::Send)?;
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    assert!(commands.try_recv().is_err());
    edit(&mut app, 1, "The map closes unexpectedly")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn bug_report_ctrl_z_respects_input_fences() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, _, _) = fixture()?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.play.as_ref().ok_or("play")?.link.bug_report_close();
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(29)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .bug
            .open
    );
    owner.open_graphics();
    owner.route_input(std::slice::from_ref(&key), false);
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .bug
            .open
    );
    owner.cancel();
    owner.route_input(&[key], false);
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .bug
            .open
    );
    Ok(())
}
