use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_raid_schedule_selected(selected);
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
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}

fn show(app: &mut App) -> TestResult {
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        open(&mut owner);
    }
    app.update();
    Ok(())
}
#[test]
fn raid_schedule_window_shows_server_data_only() -> TestResult {
    let (mut app, _, _) = fixture(true)?;
    show(&mut app)?;
    let panel = &app
        .world()
        .resource::<State>()
        .snapshot
        .as_ref()
        .ok_or("snapshot")?
        .1;
    assert!(panel.summary.is_empty());
    let expected = oteryn_client_ui::raid_schedule::panel_text(panel);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    Ok(())
}
#[test]
fn raid_schedule_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    show(&mut app)?;
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let before = link.raid_schedule_panel();
    assert!(link.raid_schedule_dispatch_bound(&before, Kind::Refresh));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(command.capability, 149);
    assert!(command.payload.is_empty());
    assert_eq!(link.raid_schedule_panel().summary, before.summary);
    assert!(!link.raid_schedule_dispatch_bound(&before, Kind::Refresh));
    assert!(commands.try_recv().is_err());
    let payload = oteryn_client_runtime::raid_schedule::encode_raid_schedule_state(
        &oteryn_client_runtime::raid_schedule::RaidScheduleState {
            raid_key: "raid:test".into(),
            running: true,
            committed_steps: 2,
            active_creatures: 5,
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    outbox.bevy_raid_schedule_reply(
        &command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
            payload,
        },
    );
    assert_eq!(
        link.raid_schedule_panel().summary,
        "raid:test | Running\r\nCommitted steps: 2 | Active creatures: 5"
    );
    Ok(())
}
#[test]
fn raid_schedule_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, _) = fixture(true)?;
    show(&mut app)?;
    let owner = app.world().non_send::<HostOwner>();
    let state = app.world().resource::<State>();
    let (key, panel) = state.snapshot.as_ref().ok_or("snapshot")?;
    perform(
        owner,
        state,
        &Control {
            key: Key {
                generation: key.generation.wrapping_add(1),
                revision: key.revision,
            },
            kind: Kind::Refresh,
        },
    );
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let mut changed = panel.clone();
    changed.summary = "changed target".into();
    assert!(!link.raid_schedule_dispatch_bound(&changed, Kind::Refresh));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn raid_schedule_window_registered() -> TestResult {
    let (mut app, _, _) = fixture(true)?;
    show(&mut app)?;
    assert!(app.world().resource::<State>().widgets.is_some());
    assert!(
        app.world_mut()
            .query::<&crate::bevy_shell::ui::Control>()
            .iter(app.world())
            .any(|control| matches!(control, crate::bevy_shell::ui::Control::RaidSchedule))
    );
    Ok(())
}
