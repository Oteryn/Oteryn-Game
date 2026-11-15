use super::*;
use oteryn_client_ui::task_board::task_board::full::*;
fn panel() -> TaskBoardPanel {
    TaskBoardPanel {
        selected: true,
        open: true,
        weekly: Some(Reply {
            refusal: 0,
            period: 42,
            points: 20,
            bounty_points: 10,
            soulseals: 3,
            weekly: Weekly {
                started: true,
                difficulty: 1,
                expanded: false,
                multiplier: 1,
                kill_experience: 100,
                delivery_experience: 50,
                pending_points: 5,
                pending_soulseals: 2,
                kills: vec![KillTask {
                    race: "rat".into(),
                    required: 10,
                    current: 3,
                }],
                deliveries: vec![],
            },
            weekly_results: Some(WeeklyResults {
                period: 41,
                points: 8,
                soulseals: 2,
            }),
            bounty: Bounty {
                elite: false,
                state: 1,
                difficulty: 1,
                selected: 2,
                current: 10,
                rerolls: 2,
                daily_reroll_at: 0,
                offers: (0..3)
                    .map(|grade| BountyOffer {
                        elite: false,
                        dust: 0,
                        race: format!("monster{grade}"),
                        required: 10,
                        experience: 100,
                        points: 5,
                        grade,
                    })
                    .collect(),
                active: None,
                lists: vec![],
            },
            talisman: Default::default(),
            shop: vec![ShopOffer {
                index: 7,
                kind: 1,
                price: 5,
                primary: 123,
                secondary: 1,
                addon: 0,
                bought: false,
            }],
        }),
        ..Default::default()
    }
}
#[test]
fn taskboard_bounty_rows() {
    let p = panel();
    let rows = entries(&p, Tab::Bounty, false);
    for text in [
        "monster0: 0/10",
        "monster1: 10/10",
        "monster2: 0/10",
        "Silver",
        "Completed",
    ] {
        assert!(rows.iter().any(|r| r.text.contains(text)), "{text}");
    }
    assert_eq!(
        rows.iter().filter(|r| r.text.contains("monster")).count(),
        3
    );
}
#[test]
fn taskboard_reroll_confirm() -> Result<(), &'static str> {
    let mut p = panel();
    let mut confirm = false;
    assert!(intent(&p, &mut confirm, &Kind::Reroll).is_none());
    assert!(confirm);
    let i = intent(&p, &mut confirm, &Kind::ConfirmReroll).ok_or("confirmation")?;
    let command = oteryn_client_ui::task_board::full_request(&mut p, i).ok_or("command")?;
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "decode")?
            .action,
        11
    );
    assert!(!confirm);
    Ok(())
}
#[test]
fn taskboard_claim_completed() -> Result<(), &'static str> {
    let mut p = panel();
    let mut confirm = false;
    let i = intent(&p, &mut confirm, &Kind::Claim).ok_or("claim")?;
    let command = oteryn_client_ui::task_board::full_request(&mut p, i).ok_or("command")?;
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "decode")?
            .action,
        9
    );
    p.pending = false;
    p.weekly.as_mut().ok_or("reply")?.bounty.current = 9;
    assert!(intent(&p, &mut confirm, &Kind::Claim).is_none());
    Ok(())
}
#[test]
fn taskboard_daily_claim_cap() -> Result<(), &'static str> {
    let mut p = panel();
    p.weekly.as_mut().ok_or("reply")?.bounty.rerolls = 10;
    let rows = entries(&p, Tab::Bounty, false);
    assert!(rows.iter().any(|r| r.text.contains("10 / 10")));
    assert!(
        rows.iter()
            .any(|r| r.text.contains("Daily Reward Wall") && r.action.is_none())
    );
    Ok(())
}
#[test]
fn taskboard_weekly_tab() {
    let p = panel();
    let rows = entries(&p, Tab::Weekly, false);
    assert!(rows.iter().any(|r| r.text.contains("rat: 3/10")));
    assert!(
        rows.iter()
            .any(|r| r.text.contains("41") && r.text.contains("8"))
    );
    let rows = entries(&p, Tab::Shop, false);
    assert!(rows.iter().any(|r| matches!(r.action, Some(Kind::Buy(7)))));
}
#[test]
fn taskboard_selection_and_difficulty() -> Result<(), &'static str> {
    let mut p = panel();
    p.weekly.as_mut().ok_or("reply")?.bounty.selected = 0;
    let mut confirm = false;
    for (kind, action, selection) in [
        (Kind::Select(0), 8, 0),
        (Kind::Select(2), 8, 2),
        (Kind::Difficulty(3), 10, 3),
        (Kind::Buy(7), 16, 7),
    ] {
        let i = intent(&p, &mut confirm, &kind).ok_or("intent")?;
        let command = oteryn_client_ui::task_board::full_request(&mut p, i).ok_or("command")?;
        let request = decode_request(&command.payload).map_err(|_| "decode")?;
        assert_eq!((request.action, request.selection), (action, selection));
        p.pending = false;
    }
    p.pending = true;
    assert!(intent(&p, &mut confirm, &Kind::Reroll).is_none());
    assert!(!confirm);
    Ok(())
}
#[test]
fn taskboard_reroll_cancel_and_empty_balance() -> Result<(), &'static str> {
    let mut p = panel();
    let mut confirm = false;
    assert!(intent(&p, &mut confirm, &Kind::ConfirmReroll).is_none());
    assert!(intent(&p, &mut confirm, &Kind::Reroll).is_none());
    assert!(confirm);
    assert!(intent(&p, &mut confirm, &Kind::Cancel).is_none());
    assert!(!confirm);
    p.weekly.as_mut().ok_or("reply")?.bounty.rerolls = 0;
    assert!(intent(&p, &mut confirm, &Kind::Reroll).is_none());
    assert!(!confirm);
    Ok(())
}
#[test]
fn taskboard_native_activate_sends_confirmed_intent_and_rejects_stale_revision()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_task_board_fixture(panel());
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner).insert_resource(State {
        generation,
        revision: 1,
        seen: Some(panel()),
        ..default()
    });
    let root = app.world_mut().spawn((GameModal, Node::default())).id();
    let reroll = app
        .world_mut()
        .spawn((
            Node::default(),
            Action {
                generation,
                revision: 1,
                kind: Kind::Reroll,
            },
        ))
        .observe(activate)
        .id();
    let confirm = app
        .world_mut()
        .spawn((
            Node::default(),
            Action {
                generation,
                revision: 1,
                kind: Kind::ConfirmReroll,
            },
        ))
        .observe(activate)
        .id();
    app.world_mut()
        .entity_mut(root)
        .add_children(&[reroll, confirm]);
    app.world_mut().trigger(Activate { entity: reroll });
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().confirm);
    app.world_mut().trigger(Activate { entity: confirm });
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .action,
        11
    );
    app.world_mut().resource_mut::<State>().revision = 2;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn taskboard_open_bounty_when_no_offers() -> Result<(), &'static str> {
    let mut p = panel();
    if let Some(r) = p.weekly.as_mut() {
        r.bounty.selected = 0;
        r.bounty.offers.clear();
        r.bounty.active = None;
    }
    let rows = entries(&p, Tab::Bounty, false);
    let row = rows
        .iter()
        .find(|r| matches!(r.action, Some(Kind::OpenBounty)))
        .ok_or("open bounty row")?;
    assert!(row.text.contains("Open bounty"));
    let mut confirm = false;
    let i = intent(&p, &mut confirm, &Kind::OpenBounty).ok_or("open intent")?;
    let command = oteryn_client_ui::task_board::full_request(&mut p, i).ok_or("command")?;
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "decode")?
            .action,
        7
    );
    let offered = panel();
    assert!(
        !entries(&offered, Tab::Bounty, false)
            .iter()
            .any(|r| matches!(r.action, Some(Kind::OpenBounty)))
    );
    assert!(intent(&offered, &mut confirm, &Kind::OpenBounty).is_none());
    Ok(())
}

type ExtraFixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
fn extra_fixture(mut value: TaskBoardPanel) -> Result<ExtraFixture, Box<dyn std::error::Error>> {
    value.open = true;
    let (link, commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_task_board_fixture(value);
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), -7, &[])?,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn extra_click(
    app: &mut App,
    test: impl Fn(&Kind) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| test(&a.kind).then_some(e))
        .ok_or("task-board action missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn taskboard_preference_panels_confirm_both_assignments_through_owner()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    for preferred in [true, false] {
        let mut p = panel();
        p.weekly.as_mut().ok_or("board")?.weekly.kills[0].race = "oteryn:creature.rat".into();
        p.weekly.as_mut().ok_or("board")?.bounty.lists = vec![ListSlot {
            unlocked: true,
            preferred: String::new(),
            unwanted: String::new(),
        }];
        let (mut app, mut commands, _) = extra_fixture(p)?;
        extra_click(&mut app, |kind| matches!(kind, Kind::Tab(Tab::Preferences)))?;
        extra_click(&mut app, |kind| match kind {
            Kind::Extra(extras::Action::Preferred(0, race)) if preferred => {
                race == "oteryn:creature.rat"
            }
            Kind::Extra(extras::Action::Unwanted(0, race)) if !preferred => {
                race == "oteryn:creature.rat"
            }
            _ => false,
        })?;
        assert!(commands.try_recv().is_err());
        extra_click(&mut app, |kind| matches!(kind, Kind::ConfirmExtra(_)))?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("preference command".into());
        };
        let request = decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(request.action, if preferred { 13 } else { 14 });
        assert_eq!(request.selection, 0);
        assert_eq!(request.race, "oteryn:creature.rat");
    }
    Ok(())
}
#[test]
fn taskboard_soulseal_panel_reviews_cost_then_enters_exact_authorized_fight()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use model::task_board::soulpit as wire;
    let mut p = panel();
    p.fights.reply = Some(wire::Reply {
        status: wire::Status::Catalogue,
        balance: Some(37),
        authorization: Some([7; 16]),
        next: None,
        total: 1,
        rules: wire::EntryRules {
            required_level: 8,
            time_limit_seconds: 600,
            authorization_ms: 10000,
        },
        entries: vec![wire::Fight {
            race: 5,
            name: "Grove watcher".into(),
            stars: 2,
            cost: 30,
        }],
        reason: String::new(),
    });
    p.fights.received_at = Some(std::time::Instant::now());
    let (mut app, mut commands, outbox) = extra_fixture(p.clone())?;
    extra_click(&mut app, |kind| matches!(kind, Kind::Tab(Tab::Soulseals)))?;
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("catalogue query".into());
    };
    assert_eq!(
        wire::Request::decode(&query.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::Catalogue { after: 0 }
    );
    outbox.bevy_task_board_fixture(p);
    app.update();
    extra_click(&mut app, |kind| {
        matches!(kind, Kind::Extra(extras::Action::Fight(5)))
    })?;
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("30 Soulseals?"))
    );
    extra_click(&mut app, |kind| matches!(kind, Kind::ConfirmExtra(_)))?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("fight command".into());
    };
    assert_eq!(
        wire::Request::decode(&command.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request::Enter {
            authorization: [7; 16],
            race: 5
        }
    );
    Ok(())
}
