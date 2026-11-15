use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::daily_reward::{DailyRewardPanel, daily_reward};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
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
fn panel() -> DailyRewardPanel {
    let mut details = daily_reward::details::Details::empty(42);
    details.days[0].free.cosmetic_tokens = 3;
    details.days[0].premium = daily_reward::details::Tier {
        prey_wildcards: 2,
        xp_percent: 25,
        xp_minutes: 45,
        temple_scrolls: 1,
        ..Default::default()
    };
    details.history[0] = Some(daily_reward::details::Receipt {
        epoch: 42,
        reset_day: 0,
        lane_day: 1,
        streak: 9,
        jokers_spent: 1,
        own_character: false,
    });
    DailyRewardPanel {
        selected: true,
        open: true,
        details_attempted: true,
        details: Some(details),
        wall: Some(daily_reward::DailyRewardWall {
            lane_day: 8,
            streak: 9,
            jokers: 2,
            claimed: false,
            claim_enabled: true,
        }),
        ..Default::default()
    }
}
fn texts(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control missing")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn reward_wall_grid() -> TestResult {
    let (mut app, _, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    let text = texts(&mut app);
    for day in 1..=7 {
        assert!(text.contains(&format!("Day {day}")));
    }
    assert!(text.contains("Free"));
    assert!(text.contains("Premium"));
    assert!(text.contains("Streak: 9"));
    assert!(text.contains("today"));
    assert_eq!(
        app.world_mut()
            .query::<&DayCell>()
            .iter(app.world())
            .count(),
        7
    );
    Ok(())
}
#[test]
fn reward_wall_kinds() -> TestResult {
    let (mut app, _, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    let text = texts(&mut app);
    assert!(text.contains("3 Cosmetic Tokens"));
    assert!(text.contains("2 Prey Wildcards"));
    assert!(text.contains("45 minutes XP Boost 25%"));
    assert!(text.contains("1 Temple Teleport Scroll"));
    Ok(())
}
#[test]
fn reward_wall_collect_guard() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    for (selected, pending, claimed, enabled) in [
        (false, false, false, true),
        (true, true, false, true),
        (true, false, true, true),
        (true, false, false, false),
    ] {
        let mut panel = panel();
        panel.selected = selected;
        panel.pending = pending;
        if let Some(wall) = &mut panel.wall {
            wall.claimed = claimed;
            wall.claim_enabled = enabled;
        }
        outbox.bevy_reward_wall(panel);
        app.update();
        click(&mut app, Kind::Collect)?;
        assert!(commands.try_recv().is_err());
    }
    outbox.bevy_reward_wall(panel());
    app.update();
    click(&mut app, Kind::Collect)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(oteryn_client_runtime::daily_reward::DailyRewardCommand::Claim)
    ));
    click(&mut app, Kind::Collect)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn reward_wall_history() -> TestResult {
    let (mut app, _, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    assert!(!texts(&mut app).contains("1970-01-01"));
    click(&mut app, Kind::History)?;
    let text = texts(&mut app);
    assert!(text.contains("1970-01-01"));
    assert!(text.contains("another account character"));
    click(&mut app, Kind::Wall)?;
    assert!(texts(&mut app).contains("Day 7"));
    Ok(())
}
#[test]
fn reward_wall_shortcut() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands, outbox) = fixture()?;
    let mut panel = panel();
    panel.open = false;
    outbox.bevy_reward_wall(panel);
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(7)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(oteryn_client_runtime::daily_reward::DailyRewardCommand::Query)
    ));
    app.update();
    assert!(texts(&mut app).contains("Daily Reward"));
    click(&mut app, Kind::Close)?;
    assert_eq!(
        app.world_mut()
            .query::<&DayCell>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn reward_wall_stale_and_modal_controls() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    let mut changed = panel();
    if let Some(wall) = &mut changed.wall {
        wall.claim_enabled = false;
    }
    outbox.bevy_reward_wall(changed);
    click(&mut app, Kind::Collect)?;
    assert!(commands.try_recv().is_err());
    outbox.bevy_reward_wall(panel());
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    click(&mut app, Kind::Collect)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn reward_wall_loads_catalogue_and_refreshes() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let mut loading = panel();
    loading.details = None;
    loading.details_attempted = false;
    outbox.bevy_reward_wall(loading);
    app.update();
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(command) if command.capability == 182 && command.kind == 738 && command.payload.is_empty())
    );
    app.update();
    assert!(commands.try_recv().is_err());
    outbox.bevy_reward_wall(panel());
    app.update();
    click(&mut app, Kind::Refresh)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(oteryn_client_runtime::daily_reward::DailyRewardCommand::Query)
    ));
    let snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .daily_reward_panel();
    assert!(snapshot.pending);
    assert!(!snapshot.details_attempted);
    Ok(())
}
