// Offline candidate qualification against the repository's actual Quest loader/evaluator.
// The external runner supplies `quest` and Q1_FIXTURE. No writer/composition pass is claimed.
use std::collections::BTreeMap;

const TRACK: &str = "oteryn:quest-progress/shards-candidate/";
const TRANSITION: &str = "oteryn:quest-transition/shards-candidate/";

fn catalogue() -> quest::QuestStateCatalogue {
    quest::loader::parse_quest_state(Q1_FIXTURE, "shards-candidate-r1")
        .expect("candidate must load through native loader")
        .catalogue()
        .clone()
}

fn apply(
    c: &quest::QuestStateCatalogue,
    state: &mut BTreeMap<String, i64>,
    name: &str,
) -> Result<(), quest::QuestRefusal> {
    let t = c
        .transition(&(TRANSITION.to_owned() + name))
        .expect("candidate transition");
    let changes = c.evaluate(t, state, 0)?;
    for change in changes {
        state.insert(change.track, change.after);
    }
    Ok(())
}

fn branch(c: &quest::QuestStateCatalogue, s: &mut BTreeMap<String, i64>, route: &str, name: &str) {
    let events: Vec<&str> = match name {
        "yukti" => vec![if route == "saraki" {
            "clue_yukti_sundara"
        } else {
            "clue_yukti_nipuna_alt"
        }],
        "tides" => vec![
            "tides_tarisu",
            "tide_marker_1",
            "tide_marker_2",
            "tide_marker_3",
            "tides_join",
        ],
        "plants" => vec![
            "plants_dhira",
            "plant_tide_veil",
            "plant_whisper_reed",
            "refine_tide_veil",
            "refine_whisper_reed",
            "plants_join",
        ],
        _ => panic!("unknown qualification branch"),
    };
    for event in events {
        apply(c, s, event).expect("qualified branch order");
    }
}

#[test]
fn native_loader_accepts_complete_exact_guards() {
    let loaded = quest::loader::parse_quest_state(Q1_FIXTURE, "shards-candidate-r1").unwrap();
    assert_eq!(loaded.counts().quests, 1);
    assert_eq!(loaded.counts().transitions, 18);
    assert_eq!(loaded.counts().not_supported, 0);
    assert_eq!(loaded.counts().inexact, 0);
    assert_eq!(loaded.counts().explicit_computed, 0);
}

#[test]
fn all_six_branch_orders_work_for_both_routes() {
    let c = catalogue();
    let orders = [
        ["yukti", "tides", "plants"],
        ["yukti", "plants", "tides"],
        ["tides", "yukti", "plants"],
        ["tides", "plants", "yukti"],
        ["plants", "yukti", "tides"],
        ["plants", "tides", "yukti"],
    ];
    for route in ["saraki", "nilavarna_alt"] {
        for order in orders {
            let mut s = BTreeMap::new();
            apply(&c, &mut s, &format!("start_{route}")).unwrap();
            for (index, name) in order.into_iter().enumerate() {
                branch(&c, &mut s, route, name);
                if index < 2 {
                    assert_eq!(
                        apply(&c, &mut s, "three_clues_join"),
                        Err(quest::QuestRefusal::StageMismatch)
                    );
                }
            }
            apply(&c, &mut s, "three_clues_join").unwrap();
            apply(&c, &mut s, &format!("report_{route}")).unwrap();
            assert_eq!(s.get(&(TRACK.to_owned() + "report_done")), Some(&1));
        }
    }
}

#[test]
fn every_incomplete_subset_refuses_join_without_state_changes() {
    let c = catalogue();
    for route in ["saraki", "nilavarna_alt"] {
        for subset in 0..7 {
            let mut s = BTreeMap::new();
            apply(&c, &mut s, &format!("start_{route}")).unwrap();
            for (index, name) in ["yukti", "tides", "plants"].into_iter().enumerate() {
                if subset & (1 << index) != 0 {
                    branch(&c, &mut s, route, name);
                }
            }
            let before = s.clone();
            assert_eq!(
                apply(&c, &mut s, "three_clues_join"),
                Err(quest::QuestRefusal::StageMismatch)
            );
            assert_eq!(
                apply(&c, &mut s, &format!("report_{route}")),
                Err(quest::QuestRefusal::StageMismatch)
            );
            assert_eq!(s, before);
        }
    }
}

#[test]
fn repeated_marker_does_not_replace_distinct_placements() {
    let c = catalogue();
    let mut s = BTreeMap::new();
    apply(&c, &mut s, "start_saraki").unwrap();
    apply(&c, &mut s, "tides_tarisu").unwrap();
    apply(&c, &mut s, "tide_marker_1").unwrap();
    let before = s.clone();
    for _ in 0..3 {
        assert_eq!(
            apply(&c, &mut s, "tide_marker_1"),
            Err(quest::QuestRefusal::StageMismatch)
        );
    }
    assert_eq!(
        apply(&c, &mut s, "tides_join"),
        Err(quest::QuestRefusal::StageMismatch)
    );
    assert_eq!(s, before);
}

#[test]
fn start_cannot_switch_route_and_wrong_report_refuses() {
    let c = catalogue();
    for (route, other) in [("saraki", "nilavarna_alt"), ("nilavarna_alt", "saraki")] {
        let mut s = BTreeMap::new();
        apply(&c, &mut s, &format!("start_{route}")).unwrap();
        let before = s.clone();
        assert_eq!(
            apply(&c, &mut s, &format!("start_{other}")),
            Err(quest::QuestRefusal::StageMismatch)
        );
        assert_eq!(s, before);
        for name in ["yukti", "tides", "plants"] {
            branch(&c, &mut s, route, name);
        }
        apply(&c, &mut s, "three_clues_join").unwrap();
        assert_eq!(
            apply(&c, &mut s, &format!("report_{other}")),
            Err(quest::QuestRefusal::StageMismatch)
        );
        apply(&c, &mut s, &format!("report_{route}")).unwrap();
    }
}

#[test]
fn stale_positive_session_copy_does_not_bypass_locked_join() {
    let c = catalogue();
    let mut s = BTreeMap::new();
    apply(&c, &mut s, "start_saraki").unwrap();
    for name in ["yukti", "tides", "plants"] {
        branch(&c, &mut s, "saraki", name);
    }
    // Simulate one prerequisite missing in writer input, with every other fact unchanged.
    s.insert(TRACK.to_owned() + "plants_done", 0);
    let before = s.clone();
    assert_eq!(
        apply(&c, &mut s, "three_clues_join"),
        Err(quest::QuestRefusal::StageMismatch)
    );
    assert_eq!(s, before);
}
