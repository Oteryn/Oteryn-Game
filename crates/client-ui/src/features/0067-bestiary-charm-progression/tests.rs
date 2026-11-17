use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc01_charm_progression_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(6)?;
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, modifiers, true)]));
    assert!(!open_key(&[event(ButtonState::Released, modifiers, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        modifiers.with(Modifier::Shift),
        false
    )]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    let mut panel = Panel::default();
    assert!(!request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Unavailable"));
    panel.selected = true;
    assert!(request(&mut panel, 1));
    assert!(!request(&mut panel, 2));
    assert_eq!(panel.pending_kind, 1);
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The session has ended");
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("ended"));
    Ok(())
}

#[test]
fn bestiary_browser_filters_confirmed_stages_and_selects_without_index_entry()
-> Result<(), &'static str> {
    use oteryn_client_runtime::charm_progression::{
        CharmProgressionOutcome, CharmProgressionReply, bestiary::BestiaryRaceProgress,
        charm::CharmView,
    };
    let mut panel = Panel {
        selected: true,
        reply: Some(CharmProgressionReply {
            outcome: CharmProgressionOutcome::Query,
            fee_gold: 0,
            charms: CharmView::default(),
            bestiary: vec![
                BestiaryRaceProgress {
                    race: std::num::NonZeroU32::new(1).ok_or("race")?,
                    kill_count: 3,
                    kill_thresholds: [5, 10, 20],
                },
                BestiaryRaceProgress {
                    race: std::num::NonZeroU32::new(2).ok_or("race")?,
                    kill_count: 20,
                    kill_thresholds: [5, 10, 20],
                },
            ],
        }),
        ..Default::default()
    };
    panel.race_filter = RaceFilter::Complete;
    panel.select_race(false);
    assert_eq!(panel.fields[1], "2");
    assert!(bestiary_text(&panel).contains("Stage 3: 20/20"));
    assert!(!bestiary_text(&panel).contains("Race 1:"));
    panel.race_filter = RaceFilter::InProgress;
    panel.select_race(false);
    assert_eq!(panel.fields[1], "1");
    assert!(bestiary_text(&panel).contains("Stage 1: 3/5"));
    panel.pending = true;
    panel.select_race(true);
    assert_eq!(panel.fields[1], "1");
    Ok(())
}

#[test]
fn charm_selector_uses_confirmed_costs_and_eligible_races() -> Result<(), &'static str> {
    use oteryn_client_runtime::charm_progression::{
        CharmProgressionOutcome, CharmProgressionReply,
        bestiary::BestiaryRaceProgress,
        charm::{CharmKind, CharmState, CharmView},
    };
    let id = |n| std::num::NonZeroU32::new(n).ok_or("id");
    let mut panel = Panel {
        selected: true,
        reply: Some(CharmProgressionReply {
            outcome: CharmProgressionOutcome::Query,
            fee_gold: 0,
            charms: CharmView {
                charm_points_available: 50,
                charms: vec![CharmState {
                    charm: id(7)?,
                    kind: CharmKind::Major,
                    unlocked_stage: 1,
                    assigned_race: None,
                    next_stage_cost: 250,
                    effect_active: false,
                }],
                ..Default::default()
            },
            bestiary: vec![
                BestiaryRaceProgress {
                    race: id(1)?,
                    kill_count: 5,
                    kill_thresholds: [5, 10, 20],
                },
                BestiaryRaceProgress {
                    race: id(9)?,
                    kill_count: 20,
                    kill_thresholds: [5, 10, 20],
                },
            ],
        }),
        ..Default::default()
    };
    panel.select_charm(false);
    panel.select_assignment_race(false);
    assert_eq!(panel.fields, ["7", "9"]);
    let text = charms_text(&panel);
    assert!(text.contains("250 Charm Points"));
    assert!(text.contains("Effect not yet active"));
    panel.pending = true;
    panel.select_charm(true);
    assert_eq!(panel.fields[0], "7");
    Ok(())
}

#[test]
fn selected_bestiary_card_shows_confirmed_discovery_goal() -> Result<(), &'static str> {
    use oteryn_client_runtime::charm_progression::{
        CharmProgressionOutcome, CharmProgressionReply, bestiary::BestiaryRaceProgress,
    };
    let mut panel = Panel {
        selected: true,
        reply: Some(CharmProgressionReply {
            outcome: CharmProgressionOutcome::Query,
            fee_gold: 0,
            charms: Default::default(),
            bestiary: vec![BestiaryRaceProgress {
                race: std::num::NonZeroU32::new(4).ok_or("race")?,
                kill_count: 7,
                kill_thresholds: [5, 10, 20],
            }],
        }),
        ..Default::default()
    };
    panel.select_race(false);
    let card = bestiary_text(&panel);
    assert!(card.contains("Selected creature — race 4"));
    assert!(card.contains("Current stage: 1/3"));
    assert!(card.contains("Next discovery: stage 2, 3 kills remaining"));
    panel.fields[1] = "999".into();
    assert!(bestiary_text(&panel).contains("Select a confirmed creature"));
    Ok(())
}

#[test]
fn t080_bestiary_illustrations_follow_the_selected_creature_category() {
    use oteryn_client_runtime::charm_progression::catalogue::Card;
    let card = |race, name: &str, category: &str| Card {
        race,
        name: name.into(),
        category: category.into(),
        ..Default::default()
    };
    let cards = vec![
        card(1, "Bones", "Undead"),
        card(2, "Wyrm", "Dragons"),
        card(3, "Crawler", "Insects"),
    ];
    let mut shapes = Vec::new();
    for selected in &cards {
        let mut panel = Panel {
            selected: true,
            ..Default::default()
        };
        panel.catalogue.page.cards = cards.clone();
        panel.fields[1] = selected.race.to_string();
        let text = bestiary_text(&panel);
        assert!(text.contains(&format!("{} — category illustration", selected.name)));
        assert!(text.contains('█'));
        shapes.push(
            text.split("— category illustration")
                .nth(1)
                .map(|t| t.lines().nth(1).unwrap_or("").to_owned()),
        );
    }
    shapes.sort();
    shapes.dedup();
    assert_eq!(shapes.len(), 3);
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    panel.catalogue.page.cards = cards;
    panel.fields[1] = "99".into();
    assert!(!bestiary_text(&panel).contains("category illustration"));
}

#[test]
fn t081_charm_selector_lists_names_without_indices_and_shows_full_description() {
    use oteryn_client_runtime::charm_progression::catalogue::Card;
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    panel.charm_browser.cards.insert(
        41,
        Card {
            race: 41,
            name: "Dodge".into(),
            category: "Major".into(),
            detail: "Avoid a qualifying creature attack.\r\nStage 1: 240 Charm Points".into(),
            ..Default::default()
        },
    );
    panel.charm_browser.cards.insert(
        42,
        Card {
            race: 42,
            name: "Gut".into(),
            category: "Minor".into(),
            detail: "Raises loot chance.".into(),
            ..Default::default()
        },
    );
    assert_eq!(
        super::charm_browser::resolve(&panel, 0, " dodge ").as_deref(),
        Some("41")
    );
    panel.fields[0] = "41".into();
    assert_eq!(super::charm_browser::named_fields(&panel)[0], "Dodge");
    let text = charms_text(&panel);
    assert!(text.contains("> Dodge — Major"));
    assert!(text.contains("Avoid a qualifying creature attack.\r\nStage 1: 240 Charm Points"));
    assert!(!text.contains("Raises loot chance."));
    assert!(!text.contains("41") && !text.contains("42"));
}
