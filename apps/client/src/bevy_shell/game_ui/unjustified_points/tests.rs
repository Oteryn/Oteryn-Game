use super::*;
use crate::play::{PlayView, play_channel};
use oteryn_client_runtime::{party_skull::PvpStatus, raw_feature::GatedCommandResult};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn toolbar_pvp_reply_projects_risk_timer_and_controls() -> Result {
    let mut owner = HostOwner::new(None)?;
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_pvp_initialize();
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    open(&owner);
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("expected PvP query".into());
    };
    assert_eq!(command.kind, 155);
    assert_eq!(command.payload, [8, 1]);
    let status = PvpStatus {
        points_milli: [1500, 5000, 25000],
        red_limits_milli: [3000, 5000, 10000],
        black_limits_milli: [6000, 10000, 20000],
        skull: 4,
        remaining_ms: 60001,
        situations: vec![oteryn_client_runtime::party_skull::PvpSituation {
            incoming: true,
            remaining_ms: 9000,
        }],
    };
    outbox.bevy_pvp_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: status.encode().map_err(|error| format!("{error:?}"))?,
        },
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let root = app.world().resource::<State>().root;
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(labels.iter().any(|t| t.contains("Daily: 1.500 / 3.000")));
    assert!(
        labels
            .iter()
            .any(|t| t.contains("Incoming attack: 9s at observation"))
    );
    assert!(
        labels
            .iter()
            .any(|t| t.contains("Red skull:") && t.contains("s remaining"))
    );
    let widths: Vec<_> = app
        .world_mut()
        .query::<(&RiskFill, &Node)>()
        .iter(app.world())
        .map(|(_, node)| node.width)
        .collect();
    assert_eq!(
        widths,
        [
            percent(50.),
            percent(25.),
            percent(100.),
            percent(50.),
            percent(100.),
            percent(100.)
        ]
    );
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_skull_panel();
    assert_eq!(timer_text(&panel, 0), "Red skull: 61s remaining");
    assert!(timer_text(&panel, 60001).contains("0s remaining"));
    let refresh = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| !c.close)
        .map(|(e, _)| e)
        .ok_or("refresh")?;
    app.world_mut().trigger(Activate { entity: refresh });
    let crate::combat_input::PlayCommand::Feature(refresh_command) = receiver.try_recv()? else {
        return Err("refresh command".into());
    };
    outbox.bevy_pvp_result(
        &refresh_command,
        &GatedCommandResult {
            command_id: 2,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![0],
        },
    );
    app.update();
    assert_eq!(app.world().resource::<State>().root, root);
    assert!(app.world().get::<Control>(refresh).is_some());
    assert_eq!(risk_count(&mut app), 0);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Invalid PvP status reply"))
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| c.close)
        .map(|(e, _)| e)
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(risk_count(&mut app), 0);
    Ok(())
}

fn risk_count(app: &mut App) -> usize {
    app.world_mut()
        .query::<&RiskFill>()
        .iter(app.world())
        .count()
}
