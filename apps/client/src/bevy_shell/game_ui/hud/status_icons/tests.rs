use crate::bevy_shell::owner::HostOwner;
use bevy::prelude::*;
use oteryn_client_ui::{
    battle_instinct_status::*, condition_status::*, mechanics_status::*, rest_status::*,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn conditions(entries: &[(u8, u8)]) -> ConditionStatusPanel {
    ConditionStatusPanel {
        selected: true,
        snapshot: Some(ConditionStatus {
            conditions: entries
                .iter()
                .map(|&(icon, sub_id)| ActiveCondition {
                    icon,
                    sub_id,
                    remaining_ms: -1,
                })
                .collect(),
        }),
        ..default()
    }
}
fn publish(
    app: &App,
    conditions: ConditionStatusPanel,
    rest: RestStatusPanel,
    battle: BattleInstinctStatusPanel,
    mechanics: MechanicsStatusPanel,
) -> TestResult {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_status_fixture(conditions, rest, battle, mechanics);
    Ok(())
}
fn row(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Status icons").then_some(entity))
        .ok_or_else(|| "status row missing".into())
}
fn keys(app: &mut App) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let row = row(app)?;
    for icon in app.world().get::<Children>(row).into_iter().flatten() {
        let children = app
            .world()
            .get::<Children>(*icon)
            .ok_or("icon art missing")?;
        assert!(
            children.iter().any(|entity| app
                .world()
                .get::<crate::bevy_shell::game_ui::art::ArtSlot>(entity)
                .is_some_and(|slot| slot.key.starts_with("condition-")
                    || slot.key.starts_with("player-state-flags-"))
                && app
                    .world()
                    .get::<Node>(entity)
                    .is_some_and(|node| { (node.width, node.height) == (px(9.), px(9.)) })),
            "each state is a 9x9 condition art slot"
        );
    }
    Ok(app
        .world()
        .get::<Children>(row)
        .into_iter()
        .flatten()
        .filter_map(|entity| app.world().get::<Name>(*entity))
        .map(|name| name.as_str().to_owned())
        .collect())
}

#[test]
fn status_icons_show_only_active_in_order() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    publish(
        &app,
        conditions(&[(27, 0), (1, 0), (13, 0), (20, 1), (20, 0)]),
        default(),
        default(),
        default(),
    )?;
    app.update();
    // Tibia strip conditions come first in state-bit order, the rest in catalogue order.
    assert_eq!(keys(&mut app)?, ["Poison", "Haste", "Agony", "Skill bonus"]);
    // The row is centred under the status bar over the game window.
    let status_row = row(&mut app)?;
    let parent = app
        .world()
        .get::<ChildOf>(status_row)
        .ok_or("row parent")?
        .parent();
    assert_eq!(
        app.world()
            .get::<Node>(parent)
            .ok_or("centre node")?
            .justify_content,
        JustifyContent::Center
    );
    publish(&app, default(), default(), default(), default())?;
    app.update();
    assert!(keys(&mut app)?.is_empty());
    assert_eq!(
        app.world()
            .get::<Node>(status_row)
            .ok_or("row node")?
            .display,
        Display::None
    );
    Ok(())
}

fn art_keys(app: &mut App) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let row = row(app)?;
    let mut out = Vec::new();
    for icon in app.world().get::<Children>(row).into_iter().flatten() {
        for child in app.world().get::<Children>(*icon).into_iter().flatten() {
            if let Some(slot) = app
                .world()
                .get::<crate::bevy_shell::game_ui::art::ArtSlot>(*child)
            {
                out.push(slot.key.clone());
            }
        }
    }
    Ok(out)
}

#[test]
fn conditions_use_the_tibia_state_flag_cells_in_bit_order() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    // Agony, Feared, Bleeding, Paralysed, Drunk, Poison, Mana Shield, Invisible.
    publish(
        &app,
        conditions(&[
            (27, 0),
            (12, 0),
            (4, 0),
            (14, 0),
            (10, 0),
            (1, 0),
            (17, 0),
            (9, 0),
        ]),
        default(),
        default(),
        default(),
    )?;
    app.update();
    assert_eq!(
        keys(&mut app)?,
        [
            "Poison",
            "Drunk",
            "Paralysed",
            "Bleeding",
            "Feared",
            "Mana Shield",
            "Agony",
            "Invisible"
        ]
    );
    assert_eq!(
        art_keys(&mut app)?,
        [
            "player-state-flags-0",
            "player-state-flags-3",
            "player-state-flags-5",
            "player-state-flags-15",
            "player-state-flags-20",
            "player-state-flags-26",
            "player-state-flags-27",
            "condition-invisible"
        ]
    );
    Ok(())
}

#[test]
fn status_icons_include_rest_and_battle_instinct() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    let rest = RestStatusPanel {
        selected: true,
        snapshot: Some(RestStatus {
            inside_resting_area: true,
            food_remaining_ms: 0,
            ..default()
        }),
        ..default()
    };
    let battle = BattleInstinctStatusPanel {
        selected: true,
        snapshot: Some(BattleInstinctStatus {
            active: true,
            melee: 4,
            shielding: 24,
        }),
        ..default()
    };
    let mechanics = MechanicsStatusPanel {
        selected: true,
        snapshot: Some(MechanicsStatus {
            observed_unix_ms: 100,
            dust: 0,
            dust_limit: 100,
            deaths: vec![],
            members: vec![],
            timers: vec![EffectTimer {
                key: "wealth_duplex".into(),
                remaining_ms: 1000,
                online_only: true,
            }],
        }),
        ..default()
    };
    publish(&app, conditions(&[(1, 0)]), rest, battle, mechanics)?;
    app.update();
    assert_eq!(
        keys(&mut app)?,
        [
            "Poison",
            "Resting Area",
            "Hungry",
            "Battle Instinct",
            "wealth_duplex"
        ]
    );
    publish(&app, conditions(&[]), default(), default(), default())?;
    app.update();
    assert!(keys(&mut app)?.is_empty());
    Ok(())
}

#[test]
fn status_icon_badge_only_above_one() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    for (entries, expected) in [
        (vec![], vec![]),
        (vec![(20, 0)], vec![]),
        (vec![(20, 0), (20, 1)], vec!["2"]),
        (vec![(20, 0), (20, 1), (20, 2)], vec!["3"]),
        (vec![(20, 0)], vec![]),
    ] {
        publish(&app, conditions(&entries), default(), default(), default())?;
        app.update();
        row(&mut app)?;
        let badges = app
            .world_mut()
            .query::<(&Name, &Text)>()
            .iter(app.world())
            .filter(|(name, _)| name.as_str() == "Status count")
            .map(|(_, text)| text.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(badges, expected);
    }
    Ok(())
}

#[test]
fn status_icons_update_through_live_hud_system() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    for entries in [vec![(1, 0)], vec![(2, 0)], vec![]] {
        publish(&app, conditions(&entries), default(), default(), default())?;
        app.update();
        let expected = entries
            .iter()
            .map(|(icon, _)| icon_key(*icon))
            .collect::<Vec<_>>();
        assert_eq!(keys(&mut app)?, expected);
    }
    publish(&app, conditions(&[(1, 0)]), default(), default(), default())?;
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(keys(&mut app)?.is_empty(), "logout clears confirmed icons");
    Ok(())
}

#[test]
fn status_icons_hide_unavailable_and_hidden_observations() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    let mut panel = conditions(&[(1, 0), (2, 0), (13, 0)]);
    panel.hidden = vec![2];
    panel.order = vec![13, 1];
    publish(&app, panel.clone(), default(), default(), default())?;
    app.update();
    assert_eq!(
        keys(&mut app)?,
        ["Poison", "Haste"],
        "fixed catalogue order"
    );
    panel.error = Some("Session ended");
    publish(&app, panel.clone(), default(), default(), default())?;
    app.update();
    assert!(keys(&mut app)?.is_empty());
    panel.error = None;
    panel.selected = false;
    publish(&app, panel, default(), default(), default())?;
    app.update();
    assert!(keys(&mut app)?.is_empty());
    Ok(())
}

#[test]
fn status_icons_request_the_existing_view_models() -> TestResult {
    use crate::combat_input::PlayCommand;
    let (mut app, mut commands, _outbox) = super::super::tests::app()?;
    while commands.try_recv().is_ok() {}
    publish(
        &app,
        ConditionStatusPanel {
            selected: true,
            ..default()
        },
        RestStatusPanel {
            selected: true,
            ..default()
        },
        BattleInstinctStatusPanel {
            selected: true,
            ..default()
        },
        MechanicsStatusPanel {
            selected: true,
            ..default()
        },
    )?;
    app.update();
    let mut requests = Vec::new();
    while let Ok(command) = commands.try_recv() {
        if let PlayCommand::Feature(feature) = command {
            requests.push((feature.capability, feature.kind));
        }
    }
    requests.sort();
    assert_eq!(requests, [(276, 492), (277, 493), (278, 494), (384, 669)]);
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "pending requests are not duplicated"
    );
    Ok(())
}

#[test]
fn status_icons_follow_only_the_status_bar_column_for_every_condition() -> TestResult {
    let (mut app, _commands, _outbox) = super::super::tests::app()?;
    let rest = RestStatusPanel {
        selected: true,
        snapshot: Some(RestStatus {
            inside_resting_area: true,
            food_remaining_ms: 1000,
            ..default()
        }),
        ..default()
    };
    publish(
        &app,
        conditions(&[(1, 0)]),
        rest.clone(),
        default(),
        default(),
    )?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_taints_fixture(oteryn_client_runtime::boss_encounter::Taints {
            soul_war: 3,
            rotten_blood: 2,
            rotten_blood_flags: 0,
            rotten_blood_final: false,
        });
    let everything = ["Poison", "Resting Area", "Soul War", "Rotten Blood"];
    let hide = |app: &mut App, hud: bool, keys: &[&str]| -> TestResult {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let play = host.play.as_mut().ok_or("play")?;
        let mut preferences = play.view.preferences().clone();
        preferences.condition_hud_hidden.clear();
        preferences.condition_status_hidden.clear();
        for key in keys {
            preferences.set_condition_visible(key, hud, false);
        }
        play.view.set_preferences(preferences)?;
        Ok(())
    };
    app.update();
    assert_eq!(keys(&mut app)?, everything);
    // Hiding every HUD column leaves the status bar untouched.
    hide(&mut app, true, &everything)?;
    app.update();
    assert_eq!(keys(&mut app)?, everything);
    // Each Status Bar column hides exactly its own icon.
    for key in everything {
        hide(&mut app, false, &[key])?;
        app.update();
        let expected: Vec<_> = everything
            .into_iter()
            .filter(|other| *other != key)
            .collect();
        assert_eq!(keys(&mut app)?, expected, "{key}");
    }
    hide(&mut app, false, &[])?;
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let play = host.play.as_mut().ok_or("play")?;
        let mut prefs = play.view.preferences().clone();
        prefs.special_condition_order = prefs.ordered_condition_keys();
        prefs.special_condition_order.reverse();
        play.view.set_preferences(prefs)?;
    }
    app.update();
    assert_eq!(
        keys(&mut app)?,
        ["Rotten Blood", "Soul War", "Resting Area", "Poison"]
    );
    Ok(())
}
