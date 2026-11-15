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

#[test]
fn taskboard_talisman_paths_and_purchase_are_reachable() {
    let p = panel();
    let rows = entries(&p, Tab::Bounty, false);
    for path in ["Damage", "Life leech", "Loot", "Double Bestiary chance"] {
        assert!(
            rows.iter()
                .any(|r| r.text.contains(path) && r.text.contains("level") && r.action.is_some()),
            "{path}"
        );
    }
    assert!(
        rows.iter()
            .any(|r| r.text.contains("Purchase: 5000 gold") && r.action.is_some())
    );
    assert!(
        rows.iter()
            .any(|r| r.text.contains("bounty points") && r.text.contains("Next level"))
    );
}

#[test]
fn taskboard_lists_expose_values_and_edit_choices() -> Result<(), &'static str> {
    let mut p = panel();
    p.weekly.as_mut().ok_or("reply")?.bounty.lists = vec![
        ListSlot {
            unlocked: true,
            preferred: "rat".into(),
            unwanted: "spider".into(),
        },
        ListSlot {
            unlocked: false,
            preferred: String::new(),
            unwanted: String::new(),
        },
    ];
    let rows = entries(&p, Tab::Bounty, false);
    for text in [
        "Preferred: rat",
        "Unwanted: spider",
        "Unlock list",
        "Prefer monster0",
        "Avoid monster0",
        "Clear preferred",
        "Clear unwanted",
    ] {
        assert!(
            rows.iter()
                .any(|r| r.text.contains(text) && r.action.is_some()),
            "{text}"
        );
    }
    Ok(())
}
#[test]
fn taskboard_elite_mode_is_authoritative_and_capability_gated() -> Result<(), &'static str> {
    let mut p = panel();
    assert!(
        !entries(&p, Tab::Bounty, false)
            .iter()
            .any(|r| r.text.starts_with("Elite mode:"))
    );
    p.weekly_results_selected = true;
    assert!(
        entries(&p, Tab::Bounty, false)
            .iter()
            .any(|r| r.text == "Elite mode: disabled — enable" && r.action.is_some())
    );
    p.weekly.as_mut().ok_or("reply")?.bounty.elite = true;
    assert!(
        entries(&p, Tab::Bounty, false)
            .iter()
            .any(|r| r.text == "Elite mode: enabled — disable" && r.action.is_some())
    );
    Ok(())
}
#[test]
fn taskboard_every_offer_and_separate_active_offer_show_both_rewards() -> Result<(), &'static str> {
    let mut p = panel();
    let reply = p.weekly.as_mut().ok_or("reply")?;
    reply.bounty.active = Some(BountyOffer {
        race: "separate".into(),
        experience: 777,
        points: 13,
        ..reply.bounty.offers[0].clone()
    });
    let rows = entries(&p, Tab::Bounty, false);
    for name in ["monster0", "monster1", "monster2", "separate"] {
        let row = rows
            .iter()
            .find(|r| r.text.contains(name))
            .ok_or("offer row")?;
        let (xp, points) = if name == "separate" {
            (777, 13)
        } else {
            (100, 5)
        };
        assert!(
            row.text.contains(&format!("{xp} XP"))
                && row.text.contains(&format!("{points} bounty points")),
            "{}",
            row.text
        );
    }
    Ok(())
}
fn spending_click(kind: Kind, stale: bool) -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    let mut p = panel();
    p.weekly
        .as_mut()
        .ok_or("reply")?
        .weekly
        .deliveries
        .push(DeliveryTask {
            item: 1,
            required: 5,
            current: 5,
            delivered: false,
        });
    let expected = intent(&p, &mut false, &kind).ok_or("spending intent")?;
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_task_board_fixture(p.clone());
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?,
    });
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner).insert_resource(State {
        generation,
        revision: 1,
        seen: Some(p.clone()),
        ..default()
    });
    let root = app.world_mut().spawn((GameModal, Node::default())).id();
    let button = app
        .world_mut()
        .spawn((
            Node::default(),
            Action {
                generation,
                revision: 1,
                kind,
            },
        ))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(root).add_child(button);
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        commands.try_recv().is_err(),
        "spending requires displayed confirmation"
    );
    let confirm = app
        .world_mut()
        .spawn((
            Node::default(),
            Action {
                generation,
                revision: 1,
                kind: Kind::ConfirmAction,
            },
        ))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(root).add_child(confirm);
    if stale {
        let mut updated = p.clone();
        updated.weekly.as_mut().ok_or("reply")?.points += 1;
        outbox.bevy_task_board_fixture(updated);
    }
    app.world_mut().trigger(Activate { entity: confirm });
    if stale {
        assert!(
            commands.try_recv().is_err(),
            "a replacement board invalidates consent"
        );
    } else {
        let crate::combat_input::PlayCommand::Feature(actual) = commands.try_recv()? else {
            return Err("feature command".into());
        };
        let expected = model::full_request(&mut p, expected).ok_or("expected command")?;
        assert_eq!(actual.payload, expected.payload);
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(commands.try_recv().is_err(), "confirmation dispatches once");
    }
    Ok(())
}
#[test]
fn taskboard_purchase_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    spending_click(Kind::Buy(7), true)?;
    spending_click(Kind::Buy(7), false)
}
#[test]
fn taskboard_delivery_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    spending_click(Kind::Deliver(0), true)?;
    spending_click(Kind::Deliver(0), false)
}
#[test]
fn taskboard_claim_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    spending_click(Kind::Claim, true)?;
    spending_click(Kind::Claim, false)
}

#[test]
fn taskboard_projection_preserves_dragged_position_on_reply()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    let (link, _, outbox) = play_channel();
    outbox.bevy_task_board_fixture(panel());
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?,
    });
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<State>()
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
        .add_systems(Update, project);
    app.update();
    let position = UiTransform::from_translation(Val2::px(31., 47.));
    *app.world_mut()
        .query_filtered::<&mut UiTransform, With<crate::bevy_shell::window_drag::DraggableDialog>>()
        .single_mut(app.world_mut())? = position;
    let mut updated = panel();
    updated.weekly.as_mut().ok_or("reply")?.points += 1;
    outbox.bevy_task_board_fixture(updated);
    app.update();
    assert_eq!(
        *app.world_mut()
            .query_filtered::<&UiTransform, With<crate::bevy_shell::window_drag::DraggableDialog>>()
            .single(app.world())?,
        position
    );
    Ok(())
}

#[test]
fn taskboard_new_controls_dispatch_the_existing_owners_and_reject_replaced_board()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    let mut p = panel();
    p.weekly_results_selected = true;
    for offer in &mut p.weekly.as_mut().ok_or("reply")?.bounty.offers {
        offer.race = format!("oteryn:creature.{}", offer.race);
    }
    p.weekly.as_mut().ok_or("reply")?.bounty.lists = vec![
        ListSlot {
            unlocked: true,
            preferred: "rat".into(),
            unwanted: "spider".into(),
        },
        ListSlot {
            unlocked: false,
            preferred: String::new(),
            unwanted: String::new(),
        },
    ];
    for (label, action, selection) in [
        ("Purchase: 5000 gold", 17, 0),
        ("Damage — level", 18, 0),
        ("Life leech — level", 18, 1),
        ("Loot — level", 18, 2),
        ("Double Bestiary chance — level", 18, 3),
        ("Prefer monster0", 13, 0),
        ("Avoid monster0", 14, 0),
        ("Unlock list", 15, 0),
        ("Elite mode:", 19, 1),
    ] {
        let kind = entries(&p, Tab::Bounty, false)
            .into_iter()
            .find(|r| r.text.contains(label))
            .and_then(|r| r.action)
            .ok_or_else(|| format!("missing {label}"))?;
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_task_board_fixture(p.clone());
        let mut owner = HostOwner::new(None)?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?,
        });
        let generation = owner.generation();
        let mut app = App::new();
        app.insert_non_send(owner).insert_resource(State {
            generation,
            revision: 1,
            seen: Some(p.clone()),
            ..default()
        });
        let root = app.world_mut().spawn((GameModal, Node::default())).id();
        let button = app
            .world_mut()
            .spawn((
                Node::default(),
                Action {
                    generation,
                    revision: 1,
                    kind,
                },
            ))
            .observe(activate)
            .id();
        app.world_mut().entity_mut(root).add_child(button);
        let mut replaced = p.clone();
        replaced.weekly.as_mut().ok_or("reply")?.points += 1;
        outbox.bevy_task_board_fixture(replaced);
        app.world_mut().trigger(Activate { entity: button });
        assert!(
            commands.try_recv().is_err(),
            "{label} must bind to displayed board"
        );
        outbox.bevy_task_board_fixture(p.clone());
        app.world_mut().trigger(Activate { entity: button });
        if action == 17 || action == 18 {
            assert!(
                commands.try_recv().is_err(),
                "talisman spending requires confirmation"
            );
            let confirm = app
                .world_mut()
                .spawn((
                    Node::default(),
                    Action {
                        generation,
                        revision: 1,
                        kind: Kind::ConfirmAction,
                    },
                ))
                .observe(activate)
                .id();
            app.world_mut().entity_mut(root).add_child(confirm);
            app.world_mut().trigger(Activate { entity: confirm });
        }
        let crate::combat_input::PlayCommand::Feature(command) = commands
            .try_recv()
            .map_err(|error| format!("{label}: {error}"))?
        else {
            return Err("feature command".into());
        };
        let request = decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (request.action, request.selection),
            (action, selection),
            "{label}"
        );
    }
    Ok(())
}
