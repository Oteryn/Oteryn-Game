use super::*;
use crate::{
    bevy_shell::{
        owner::{ActivePlay, HostOwner},
        ui::Skins,
    },
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use oteryn_client_ui::task_board::{TaskBoardPanel, task_board::full::*};
use oteryn_client_ui_bevy as source;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn board(current: u32) -> TaskBoardPanel {
    TaskBoardPanel {
        selected: true,
        weekly: Some(Reply {
            refusal: 0,
            period: 42,
            points: 0,
            bounty_points: 0,
            soulseals: 0,
            weekly: Weekly {
                started: true,
                difficulty: 1,
                expanded: false,
                multiplier: 2,
                kill_experience: 100,
                delivery_experience: 50,
                pending_points: 5,
                pending_soulseals: 2,
                kills: vec![KillTask {
                    race: "rat".into(),
                    current,
                    required: 10,
                }],
                deliveries: vec![],
            },
            weekly_results: None,
            bounty: Bounty {
                elite: false,
                state: 1,
                difficulty: 1,
                selected: 1,
                current,
                rerolls: 0,
                daily_reroll_at: 0,
                offers: vec![],
                active: Some(BountyOffer {
                    elite: false,
                    dust: 0,
                    race: "dragon".into(),
                    required: 10,
                    experience: 500,
                    points: 7,
                    grade: 1,
                }),
                lists: vec![],
            },
            talisman: Default::default(),
            shop: vec![],
        }),
        ..Default::default()
    }
}
fn fixture() -> Result<
    (
        App,
        Outbox,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    outbox.bevy_task_board_fixture(board(3));
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.insert_non_send(owner)
        .insert_resource(Time::<()>::default())
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        })
        .add_systems(Startup, super::super::setup);
    install(&mut app);
    Ok((app, outbox, commands))
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
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
#[test]
fn task_tracker_survives_closed_board_and_refreshes_live_progress() -> TestResult {
    let (mut app, outbox, mut commands) = fixture()?;
    app.update();
    let rendered = text(&mut app);
    assert!(rendered.contains("Task tracker"), "{rendered}");
    assert!(rendered.contains("dragon: 3/10"), "{rendered}");
    assert!(rendered.contains("rat: 3/10"), "{rendered}");
    assert!(!link(&app)?.task_board_panel().open);
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    outbox.bevy_task_board_fixture(board(8));
    app.update();
    assert!(text(&mut app).contains("dragon: 8/10"));
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(5));
    app.update();
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(5));
    app.update(); // pending query must not be duplicated
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(!text(&mut app).contains("Task tracker"));
    Ok(())
}
#[test]
fn task_tracker_prey_uses_server_hunting_time_and_capability_gates() -> TestResult {
    use oteryn_client_ui::prey::{CommandStatus, PreyResponse, prey};
    let (mut app, outbox, mut commands) = fixture()?;
    outbox.bevy_task_board_fixture(TaskBoardPanel::default());
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(text(&mut app).contains("Prey unavailable"));
    outbox.bevy_prey_initialize(true);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(5));
    app.update();
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Query)
    ));
    let reply = prey::Reply {
        status: 0,
        wildcard_balance: 5,
        list_size: 9,
        expansion_owned: false,
        offered_today: vec![],
        offered_next: String::new(),
        slots: vec![prey::Slot {
            slot: 1,
            state: 2,
            creatures: vec![],
            selected: "oteryn:creature.dragon".into(),
            bonus: 1,
            grade: 7,
            percent: 19,
            hunting_seconds: 3660,
            option: 0,
            free_reroll_at: 0,
            chosen_tier: 1,
            allowed_tier: 1,
            selection_epoch: 1,
        }],
    };
    let apply = |reply: &prey::Reply| -> TestResult {
        outbox.bevy_prey_result(&PreyResponse {
            request: prey::Request::Query,
            status: CommandStatus::Accepted,
            payload: prey::encode_reply(reply).map_err(|e| format!("{e:?}"))?,
            current: None,
        });
        Ok(())
    };
    apply(&reply)?;
    app.update();
    assert!(text(&mut app).contains("Damage +19%"));
    assert!(text(&mut app).contains("Hunting time: 01:01:00"));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(60));
    app.update();
    assert!(text(&mut app).contains("Hunting time: 01:01:00"));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Query)
    ));
    let mut spent = reply;
    spent.slots[0].hunting_seconds = 3600;
    apply(&spent)?;
    app.update();
    assert!(text(&mut app).contains("Hunting time: 01:00:00"));
    let mut panel = oteryn_client_ui::prey::PreyPanel {
        selected: true,
        confirmed: Some(spent),
        ..default()
    };
    panel.confirmed.as_mut().ok_or("reply")?.slots[0].free_reroll_at = 1060;
    let first = content(&TaskBoardPanel::default(), &panel, 1000);
    let later = content(&TaskBoardPanel::default(), &panel, 1030);
    assert!(first.contains("Free reroll in 00:01:00"));
    assert!(later.contains("Free reroll in 00:00:30"));
    assert!(!content(&TaskBoardPanel::default(), &panel, 1060).contains("Free reroll in"));
    Ok(())
}
#[test]
fn task_tracker_click_opens_board_and_rejects_stale_session() -> TestResult {
    use bevy::ui_widgets::Activate;
    let (mut app, _, _commands) = fixture()?;
    app.update();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<Tracker>>()
        .single(app.world())?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .entity_mut(root)
        .insert(Tracker(generation.wrapping_add(1)));
    app.world_mut().trigger(Activate { entity: root });
    assert!(!link(&app)?.task_board_panel().open);
    app.world_mut().entity_mut(root).insert(Tracker(generation));
    app.world_mut().trigger(Activate { entity: root });
    assert!(link(&app)?.task_board_panel().open);
    link(&app)?.task_board_set_open(false);
    app.update();
    assert!(text(&mut app).contains("dragon: 3/10"));
    Ok(())
}
