use super::*;
#[test]
fn task_board_shows_server_tasks_only() {
    let panel = TaskBoardPanel {
        weekly: Some(full_reply()),
        ..Default::default()
    };
    assert!(board_text(&panel, Tab::Weekly).contains("25/50"));
    assert!(!board_text(&panel, Tab::Bounty).contains("Invented race"));
    assert!(board_text(&TaskBoardPanel::default(), Tab::Weekly).contains("Refresh"));
}
#[test]
fn task_board_actions_are_bound_and_confirm_paid() {
    assert!(paid(&TaskBoardControl::Request(FullIntent::Reroll)));
    assert!(!paid(&TaskBoardControl::Request(FullIntent::ClaimBounty)));
}
#[test]
fn soulseals_and_weekly_results_show_server_data() {
    let panel = TaskBoardPanel::default();
    assert!(board_text(&panel, Tab::Soulseals).contains("No fight catalogue loaded"));
    assert!(!board_text(&panel, Tab::Results).contains("Weekly rewards received"));
    let mut reply = full_reply();
    reply.weekly_results = Some(task_board::task_board::full::WeeklyResults {
        period: 42,
        points: 731,
        soulseals: 19,
    });
    let panel = TaskBoardPanel {
        weekly: Some(reply),
        ..Default::default()
    };
    assert!(board_text(&panel, Tab::Results).contains("731"));
    assert!(board_text(&panel, Tab::Results).contains("19"));
}
#[test]
fn task_board_actions_rejected_when_session_replaced() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, _) = crate::play::play_channel();
    let panel = link.task_board_panel();
    assert!(!link.task_board_action_bound(&panel, TaskBoardControl::Request(FullIntent::Refresh)));
    assert_eq!(link.task_board_panel(), panel);
    assert!(incoming.try_recv().is_err());
    Ok(())
}

fn full_reply() -> task_board::task_board::full::Reply {
    use task_board::task_board::full::*;
    Reply {
        weekly_results: None,
        refusal: 0,
        period: 2965,
        points: 150,
        bounty_points: 20,
        soulseals: 2,
        weekly: Weekly {
            started: true,
            difficulty: 0,
            expanded: false,
            multiplier: 1,
            kill_experience: 1850,
            delivery_experience: 1850,
            pending_points: 25,
            pending_soulseals: 0,
            kills: vec![KillTask {
                race: "oteryn:creature.rat".into(),
                required: 50,
                current: 25,
            }],
            deliveries: vec![DeliveryTask {
                item: 3031,
                required: 100,
                current: 75,
                delivered: false,
            }],
        },
        bounty: Bounty {
            elite: false,
            state: 0,
            difficulty: 0,
            selected: 0,
            current: 0,
            rerolls: 3,
            daily_reroll_at: 0,
            offers: vec![],
            active: None,
            lists: vec![],
        },
        talisman: [TalismanPath {
            level: 1,
            bonus: 250,
            next_cost: 17,
        }; 4],
        shop: vec![ShopOffer {
            index: 0,
            kind: 2,
            price: 100_000,
            primary: 1907,
            secondary: 0,
            addon: 0,
            bought: false,
        }],
    }
}

type TestApp = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
);
fn fixture() -> Result<TestApp, Box<dyn std::error::Error>> {
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    let (link, incoming, _) = crate::play::play_channel();
    link.use_shared_frontend();
    link.task_board_test_panel(
        TaskBoardPanel {
            selected: true,
            open: true,
            weekly: Some(full_reply()),
            ..Default::default()
        },
        false,
    );
    let mut owner = HostOwner::new(None)?;
    owner.phase = super::super::super::owner::Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
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
    Ok((app, incoming))
}
fn find(
    app: &mut App,
    predicate: impl Fn(&Control) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| predicate(c).then_some(e))
        .ok_or("control missing".into())
}
fn snapshot(app: &App) -> TaskBoardPanel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|p| p.link.task_board_panel())
        .unwrap_or_default()
}
#[test]
fn task_board_activate_confirms_once_and_cancel_sends_nothing()
-> Result<(), Box<dyn std::error::Error>> {
    for intent in [FullIntent::Reroll, FullIntent::Deliver(0)] {
        let (mut app, mut incoming) = fixture()?;
        let panel = snapshot(&app);
        let owner = app.world().non_send::<HostOwner>();
        let control = Control {
            generation: owner.generation(),
            session: owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .task_board_session_identity(),
            snapshot: panel.clone(),
            action: TaskBoardControl::Request(intent),
            confirm: false,
        };
        let button = app
            .world_mut()
            .spawn((Node::default(), control))
            .observe(activate)
            .id();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(incoming.try_recv().is_err());
        assert_eq!(snapshot(&app), panel);
        let cancel = find(&mut app, |c| {
            matches!(c.action, TaskBoardControl::CancelPayment)
        })?;
        app.world_mut().trigger(Activate { entity: cancel });
        app.update();
        assert!(app.world().resource::<State>().confirmation.is_none());
        assert!(incoming.try_recv().is_err());
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        let confirm = find(&mut app, |c| c.confirm)?;
        app.world_mut().trigger(Activate { entity: confirm });
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(matches!(
            incoming.try_recv()?,
            crate::combat_input::PlayCommand::Feature(_)
        ));
        assert!(incoming.try_recv().is_err());
        assert!(snapshot(&app).pending);
    }
    Ok(())
}
#[test]
fn task_board_activate_rejects_replaced_closed_and_modal_sessions()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in 0..4 {
        let (mut app, mut incoming) = fixture()?;
        let button = find(&mut app, |c| {
            matches!(c.action, TaskBoardControl::Request(FullIntent::Reroll))
        })?;
        let mut replacement = None;
        let mut modal_fixture = None;
        match mode {
            0 => {
                let (link, receiver, _) = crate::play::play_channel();
                link.task_board_test_panel(snapshot(&app), false);
                app.world_mut()
                    .non_send_mut::<HostOwner>()
                    .play
                    .as_mut()
                    .ok_or("play")?
                    .link = link;
                replacement = Some(receiver);
            }
            1 => incoming.close(),
            2 => {
                app.world_mut().spawn((
                    super::super::GameModal,
                    Node::default(),
                    GlobalZIndex(100),
                ));
            }
            _ => {
                let owner = app.world().non_send::<HostOwner>();
                let play = owner.play.as_ref().ok_or("play")?;
                modal_fixture = Some(play.link.bevy_server_modal_fixture());
            }
        }
        let before = snapshot(&app);
        let _keep_modal_alive = &modal_fixture;
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(snapshot(&app), before);
        assert!(app.world().resource::<State>().confirmation.is_none());
        assert!(incoming.try_recv().is_err());
        if let Some(mut receiver) = replacement {
            assert!(receiver.try_recv().is_err());
        }
    }
    Ok(())
}
#[test]
fn task_board_close_works_pending_blocked_and_soulseals() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut app, mut incoming) = fixture()?;
    let mut panel = snapshot(&app);
    panel.pending = true;
    panel.open = false;
    panel.soulseals_open = true;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .task_board_test_panel(panel, true);
    app.update();
    let close = find(&mut app, |c| matches!(c.action, TaskBoardControl::Close))?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!snapshot(&app).open && !snapshot(&app).soulseals_open);
    assert!(incoming.try_recv().is_err());
    Ok(())
}
