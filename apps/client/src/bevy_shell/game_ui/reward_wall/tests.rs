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

#[test]
fn reward_wall_rules_controls_explain_server_owned_rewards() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    click_caption(&mut app, "Reward rules")?;
    let text = texts(&mut app);
    assert!(text.contains("Rewards are fixed"));
    assert!(text.contains("Free: 3 Cosmetic Tokens"));
    assert!(text.contains("Premium: 2 Prey Wildcards"));
    click_caption(&mut app, "Jokers")?;
    let text = texts(&mut app);
    assert!(text.contains("Available jokers: 2"));
    assert!(text.contains("automatically"));
    assert!(text.contains("Day 1: 1 joker(s) spent"));
    assert!(text.contains("another account character"));
    click_caption(&mut app, "Access")?;
    let text = texts(&mut app);
    assert!(text.contains("within reach of a reward shrine"));
    assert!(text.contains("Shrine access is ready"));
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Refresh)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(oteryn_client_runtime::daily_reward::DailyRewardCommand::Query)
    ));
    Ok(())
}

#[test]
fn reward_wall_help_tracks_snapshot_and_closes_with_wall() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    click_caption(&mut app, "Jokers")?;
    assert!(texts(&mut app).contains("Available jokers: 2"));
    let mut changed = panel();
    if let Some(wall) = &mut changed.wall {
        wall.jokers = 0;
    }
    outbox.bevy_reward_wall(changed);
    app.update();
    assert!(texts(&mut app).contains("Available jokers: 0"));
    assert!(!texts(&mut app).contains("Available jokers: 2"));
    click_caption(&mut app, "Jokers")?;
    assert!(texts(&mut app).contains("Day 7"));
    click(&mut app, Kind::History)?;
    assert!(!texts(&mut app).contains("Available jokers:"));
    click_caption(&mut app, "Access")?;
    assert!(texts(&mut app).contains("within reach of a reward shrine"));
    click(&mut app, Kind::Close)?;
    assert!(!texts(&mut app).contains("within reach of a reward shrine"));
    assert!(commands.try_recv().is_err());
    outbox.bevy_reward_wall(panel());
    app.update();
    assert!(!texts(&mut app).contains("within reach of a reward shrine"));
    Ok(())
}

fn click_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or("reward wall caption missing")?;
    assert!(app.world().get::<Control>(entity).is_some());
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

#[test]
fn reward_wall_store_buttons_request_filtered_catalogues() -> TestResult {
    for (caption, search) in [
        ("Premium offers", "Premium"),
        ("Instant reward offers", "Instant Reward"),
    ] {
        let (mut app, mut commands, outbox) = fixture()?;
        outbox.bevy_store_initialize();
        outbox.bevy_reward_wall(panel());
        app.update();
        let entity = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
            .ok_or_else(|| format!("missing {caption} button"))?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        assert!(link.store_panel().open && !link.daily_reward_panel().open);
        assert_eq!(link.store_panel().browser.search, search);
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::ServerMechanic(_)
        ));
    }
    Ok(())
}

#[test]
fn reward_reset_clock_uses_the_matching_server_save_notice() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    for (sequence, epoch) in [(1, 41), (2, 42)] {
        outbox.push(vec![oteryn_session::SessionEvent::ServerSaveCountdown(
            oteryn_session::AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: oteryn_session::ServerSaveCountdown {
                    ending_epoch: epoch,
                    minutes_remaining: 3,
                },
            },
        )]);
        app.update();
        assert_eq!(
            texts(&mut app).contains("Reward resets in about 3 min"),
            epoch == 42
        );
    }
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualInstant(
        std::time::Instant::now() + std::time::Duration::from_secs(181),
    ));
    app.update();
    assert!(!texts(&mut app).contains("Reward resets in"));
    assert!(matches!(commands.try_recv()?, PlayCommand::DailyReward(_)));
    outbox.bevy_reward_wall(panel());
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "one refresh per expired notice"
    );
    Ok(())
}

#[test]
fn review_unavailable_store_gives_feedback_on_the_wall() -> TestResult {
    let (mut app, _, outbox) = fixture()?;
    outbox.bevy_reward_wall(panel());
    app.update();
    click(&mut app, Kind::Offers("Premium"))?;
    assert!(texts(&mut app).contains("Store unavailable"));
    Ok(())
}

#[test]
fn reward_store_send_refusal_keeps_the_wall_open() -> TestResult {
    let (mut app, commands, outbox) = fixture()?;
    outbox.bevy_store_initialize();
    outbox.bevy_reward_wall(panel());
    app.update();
    drop(commands);
    click(&mut app, Kind::Offers("Premium"))?;
    assert!(texts(&mut app).contains("The session has ended"));
    let owner = app.world().non_send::<HostOwner>();
    assert!(!owner.play.as_ref().ok_or("play")?.link.store_panel().open);
    Ok(())
}

#[test]
#[ignore = "requires the live P09 capture directory"]
fn p09_live_reward_wall_replay() -> TestResult {
    use bevy::ui::InteractionDisabled;
    use oteryn_client_ui::daily_reward::{
        CommandStatus, DailyRewardCommand, DailyRewardResponse, decode_reply,
    };

    let directory = std::path::PathBuf::from(
        std::env::var_os("P09_LIVE_CAPTURE_DIR").ok_or("missing live P09 capture directory")?,
    );
    let (mut app, mut commands, outbox) = fixture()?;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    outbox.daily_reward_initialize(true);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .daily_reward_input(&[NormalizedInputEvent::Key {
                code: KeyCode::new(7)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            }])
    );
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(DailyRewardCommand::Query)
    ));
    app.update();

    let wall = decode_reply(
        DailyRewardCommand::Query,
        &std::fs::read(directory.join("p09-wall.bin"))?,
    )?;
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(wall),
    });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("opening the playable wall did not request the reward catalogue".into());
    };
    assert_eq!(
        (command.capability, command.kind),
        (182, daily_reward::details::COMMAND_TYPE)
    );
    assert!(command.payload.is_empty());
    let details = oteryn_session::GatedCommandResult {
        command_id: std::env::var("P09_DETAILS_COMMAND_ID")?.parse()?,
        server_sequence: std::env::var("P09_DETAILS_SERVER_SEQUENCE")?.parse()?,
        status: CommandStatus::Accepted,
        payload: std::fs::read(directory.join("p09-details.bin"))?,
    };
    outbox.daily_reward_details_result(&details);
    app.update();
    require_p09_reward_cards(&mut app)?;
    click(&mut app, Kind::Refresh)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(DailyRewardCommand::Query)
    ));
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(wall),
    });
    app.update();
    let PlayCommand::Feature(reloaded) = commands.try_recv()? else {
        return Err("refresh did not request the reward catalogue".into());
    };
    assert_eq!(reloaded, command);
    outbox.daily_reward_details_result(&details);
    app.update();
    assert!(
        texts(&mut app)
            .lines()
            .any(|line| line == "Streak: 0    Jokers: 1    Day 1 of 7")
    );
    let mut controls = app
        .world_mut()
        .query::<(&Control, Option<&InteractionDisabled>)>();
    assert_eq!(
        controls
            .iter(app.world())
            .filter(|(control, disabled)| control.kind == Kind::Collect && disabled.is_some())
            .count(),
        1
    );
    click(&mut app, Kind::Collect)?;
    assert!(commands.try_recv().is_err());

    let refusal = decode_reply(
        DailyRewardCommand::Claim,
        &std::fs::read(directory.join("p09-claim.bin"))?,
    )?;
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Rejected,
        reply: Some(refusal),
    });
    app.update();
    assert!(
        texts(&mut app)
            .lines()
            .any(|line| line == "Open the daily reward shrine within reach, then refresh")
    );
    let ready = decode_reply(
        DailyRewardCommand::Query,
        &std::fs::read(directory.join("p09-ready.bin"))?,
    )?;
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(ready),
    });
    app.update();
    click(&mut app, Kind::Collect)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::DailyReward(DailyRewardCommand::Claim)
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn require_p09_reward_cards(app: &mut App) -> TestResult {
    let world = app.world_mut();
    let mut cells: Vec<Vec<String>> = world
        .query_filtered::<&Children, With<DayCell>>()
        .iter(world)
        .map(|children| {
            children
                .iter()
                .filter_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
                .collect()
        })
        .collect();
    cells.sort();
    let rewards = [
        ("2 Cosmetic Tokens", "4 Cosmetic Tokens"),
        ("1 Prey Wildcards", "2 Prey Wildcards"),
        ("1 Temple Teleport Scroll", "1 Temple Teleport Scroll"),
        ("30 minutes XP Boost 10%", "30 minutes XP Boost 25%"),
        ("3 Cosmetic Tokens", "6 Cosmetic Tokens"),
        ("1 Prey Wildcards", "2 Prey Wildcards"),
        ("30 minutes XP Boost 20%", "30 minutes XP Boost 50%"),
    ];
    let expected: Vec<Vec<String>> = rewards
        .into_iter()
        .enumerate()
        .map(|(index, (free, premium))| {
            let today = if index == 0 { " — today" } else { "" };
            let tier = |reward: &str| {
                if reward.contains("XP Boost") {
                    format!("1 Bounty Reroll Token\n{reward}")
                } else {
                    format!("{reward}\n1 Bounty Reroll Token")
                }
            };
            vec![
                format!("Day {}{today}", index + 1),
                tier(free),
                tier(premium),
            ]
        })
        .collect();
    if cells != expected {
        return Err(
            format!("playable reward cards: expected {expected:?}, observed {cells:?}").into(),
        );
    }
    Ok(())
}

#[test]
fn p09_reward_cards_reject_missing_catalogue() -> TestResult {
    let (mut app, _, outbox) = fixture()?;
    let mut wall = panel();
    wall.details = None;
    outbox.bevy_reward_wall(wall);
    app.update();
    assert!(require_p09_reward_cards(&mut app).is_err());
    Ok(())
}
