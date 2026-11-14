use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, DailyRewardResponse, daily_reward as wire,
};
use oteryn_client_ui::{daily_boost::DailyBoostPanel, daily_reward::DailyRewardPanel};
use oteryn_session::{CommandStatus, GatedCommandResult};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn entitlement() -> [u8; 16] {
    let mut id = [9; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn server_panel() -> DailyRewardPanel {
    let mut details = wire::details::Details::empty(19);
    details.days[0].free.cosmetic_tokens = 37;
    details.history[0] = Some(wire::details::Receipt {
        epoch: 18,
        reset_day: 20000,
        lane_day: 6,
        streak: 42,
        jokers_spent: 1,
        own_character: true,
    });
    DailyRewardPanel {
        selected: true,
        open: true,
        wall: Some(wire::DailyRewardWall {
            lane_day: 1,
            streak: 43,
            jokers: 2,
            claimed: false,
            claim_enabled: true,
        }),
        details: Some(details),
        details_attempted: true,
        ..default()
    }
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_daily_state(server_panel());
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
fn link(app: &App) -> Result<&crate::play::PlayLink, Box<dyn std::error::Error>> {
    Ok(&app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link)
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("daily control absent".into())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn daily_reward_window_shows_server_state_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let shown = text(&mut app);
    assert!(shown.contains("Streak: 43"));
    assert!(shown.contains("37 Cosmetic Tokens"));
    assert!(shown.contains("streak 42"));
    assert!(commands.try_recv().is_err());
    outbox.bevy_daily_state(DailyRewardPanel {
        selected: true,
        open: true,
        ..default()
    });
    app.update();
    let shown = text(&mut app);
    assert!(shown.contains("No server state loaded"));
    assert!(!shown.contains("37 Cosmetic Tokens"));
    assert!(!shown.contains("Streak: 43"));
    Ok(())
}
#[test]
fn daily_reward_claim_is_bound_and_single() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let claim = control(&mut app, Kind::Claim)?;
    app.world_mut().trigger(Activate { entity: claim });
    app.world_mut().trigger(Activate { entity: claim });
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(DailyRewardCommand::Claim)
    ));
    assert!(commands.try_recv().is_err());
    let pending = link(&app)?.daily_reward_panel();
    assert!(pending.pending);
    assert_eq!(pending.wall, server_panel().wall);
    assert!(pending.claim_receipts.iter().all(Option::is_none));
    outbox.bevy_daily_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Result(wire::DailyRewardResult {
            disposition: 1,
            transaction_id: Some([7; 16]),
        })),
    });
    app.update();
    assert!(
        link(&app)?
            .daily_reward_panel()
            .wall
            .is_some_and(|w| w.claimed)
    );
    assert!(text(&mut app).contains("Daily reward claimed"));
    Ok(())
}
#[test]
fn returner_reward_claim_is_bound() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let mut panel = server_panel();
    panel.open = false;
    panel.returner_open = true;
    panel.returner_pending = Some(wire::returner::QUERY);
    outbox.bevy_daily_state(panel);
    outbox.bevy_daily_returner_result(
        wire::returner::QUERY,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::returner::Reply {
                state: 1,
                id: Some(entitlement()),
            }
            .encode()?,
        },
    );
    app.update();
    assert!(text(&mut app).contains("Returner reward available"));
    let claim = control(&mut app, Kind::ReturnerClaim)?;
    app.world_mut().trigger(Activate { entity: claim });
    app.world_mut().trigger(Activate { entity: claim });
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("returner command".into());
    };
    assert_eq!(
        (command.capability, command.kind, command.payload),
        (182, wire::returner::CLAIM, entitlement().to_vec())
    );
    assert!(commands.try_recv().is_err());
    assert!(!link(&app)?.daily_reward_panel().returner.is_claimed());
    outbox.bevy_daily_returner_result(
        wire::returner::CLAIM,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::DuplicateReplay,
            payload: wire::DailyRewardResult {
                disposition: 2,
                transaction_id: None,
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    assert!(link(&app)?.daily_reward_panel().returner.is_claimed());
    assert!(text(&mut app).contains("Claimed"));
    Ok(())
}
#[test]
fn daily_boost_window_shows_server_data() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_daily_state(DailyRewardPanel::default());
    outbox.bevy_daily_boost_state(DailyBoostPanel {
        selected: true,
        open: true,
        boost: Some(oteryn_client_runtime::cui3::daily_boost::DailyBoostView {
            reset_epoch: 54321,
            creature_key: "oteryn:creature.cave_rat".into(),
            boss_key: "oteryn:boss.dragon_lord".into(),
        }),
        ..default()
    });
    app.update();
    let shown = text(&mut app);
    for value in ["54321", "Cave Rat", "Dragon Lord"] {
        assert!(shown.contains(value));
    }
    assert!(control(&mut app, Kind::Claim).is_err());
    assert!(control(&mut app, Kind::ReturnerClaim).is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn daily_reward_claim_rejected_when_session_replaced() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let claim = control(&mut app, Kind::Claim)?;
    let (replacement, mut next_commands, next_outbox) = play_channel();
    next_outbox.bevy_daily_state(server_panel());
    let old = std::mem::replace(
        &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .link,
        replacement,
    );
    app.world_mut().trigger(Activate { entity: claim });
    assert_eq!(link(&app)?.daily_reward_panel(), server_panel());
    assert!(commands.try_recv().is_err());
    assert!(next_commands.try_recv().is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = old;
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(250), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: claim });
    assert_eq!(link(&app)?.daily_reward_panel(), server_panel());
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    outbox.bevy_daily_end();
    let ended = link(&app)?.daily_reward_panel();
    app.world_mut().trigger(Activate { entity: claim });
    assert_eq!(link(&app)?.daily_reward_panel(), ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}
