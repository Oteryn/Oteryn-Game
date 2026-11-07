#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::*;
use crate::quest::{QuestRefusal, QuestTrackChange};

const NOW: i64 = 1_800_000_000;

#[test]
fn talisman_upgrade_paths_keep_the_accepted_canary_profile() {
    for path in 0..4 {
        assert_eq!(talisman_upgrade_cost_for_path(path, 0), Some(5));
        assert_eq!(talisman_upgrade_cost_for_path(path, 354), Some(4253));
        assert_eq!(talisman_upgrade_cost_for_path(path, 355), None);
        assert_eq!(talisman_upgrade_cost_for_path(path, -1), None);
        assert_eq!(
            talisman_bonus(path, 355),
            if path == 3 { 10_000 } else { 5_000 }
        );
    }
    assert_eq!(talisman_upgrade_cost_for_path(4, 0), None);
}

fn catalogue() -> QuestStateCatalogue {
    let base = QuestStateCatalogue::empty("content-1").expect("empty catalogue");
    extend(&base).expect("the Task Board quest is a valid catalogue")
}

/// Apply `key` at `now` to `stored` as the writer would, or return its refusal.
fn apply(
    catalogue: &QuestStateCatalogue,
    stored: &mut BTreeMap<String, i64>,
    key: &str,
    now: i64,
) -> Result<Vec<QuestTrackChange>, QuestRefusal> {
    let transition = catalogue.transition(key).expect("known transition");
    let changes = catalogue.evaluate(transition, stored, now)?;
    for change in &changes {
        stored.insert(change.track.clone(), change.after);
    }
    Ok(changes)
}

#[test]
fn the_board_is_five_valid_kill_tasks_with_three_transitions_each() {
    let catalogue = catalogue();
    assert_eq!(TASKS.len(), 5);
    for (number, task) in (1..=5).zip(TASKS) {
        assert_eq!(super::task(number), Some(&task));
        for key in [
            select_transition(number),
            kill_transition(number),
            claim_transition(number),
        ] {
            assert!(catalogue.transition(&key).is_some(), "{key}");
        }
        assert!(catalogue.track(&claimed_at_track(number)).is_some());
        assert!(task.race.starts_with("oteryn:creature."));
        assert!(task.required > 0);
    }
    assert_eq!(super::task(0), None);
    assert_eq!(super::task(6), None);
    assert_eq!(
        catalogue
            .transition(&claim_transition(1))
            .and_then(|claim| claim.experience),
        Some(EXPERIENCE)
    );
    assert!(catalogue.definition_hash(QUEST).is_some());
}

#[test]
fn select_kill_and_claim_complete_one_task() {
    let catalogue = catalogue();
    let mut stored = BTreeMap::new();
    let rat = TASKS[0];
    apply(&catalogue, &mut stored, &select_transition(1), NOW).expect("select");
    assert_eq!(state(&stored).selected, 1);
    assert_eq!(
        apply(&catalogue, &mut stored, &claim_transition(1), NOW),
        Err(QuestRefusal::StageMismatch),
        "no claim before the required kills"
    );
    for kill in 1..=rat.required {
        assert_eq!(
            kill_credit(&stored, rat.race),
            Some(kill_transition(1)),
            "kill {kill}"
        );
        apply(&catalogue, &mut stored, &kill_transition(1), NOW).expect("kill");
    }
    assert_eq!(state(&stored).kills, rat.required);
    assert_eq!(
        kill_credit(&stored, rat.race),
        None,
        "a done task asks no more"
    );
    assert_eq!(
        apply(&catalogue, &mut stored, &kill_transition(1), NOW),
        Err(QuestRefusal::StageMismatch),
        "the writer refuses a kill past the required count"
    );
    apply(&catalogue, &mut stored, &claim_transition(1), NOW).expect("claim");
    let board = state(&stored);
    assert_eq!(board.selected, 0);
    assert_eq!(board.kills, 0);
    assert_eq!(board.points, TASK_POINTS);
    assert_eq!(board.claimed_at[0], NOW);
    assert_eq!(
        apply(&catalogue, &mut stored, &claim_transition(1), NOW),
        Err(QuestRefusal::StageMismatch),
        "a claim never repeats"
    );
}

#[test]
fn talisman_upgrade_spends_the_sourced_cost_and_persists_each_path() {
    let catalogue = catalogue();
    let mut stored = BTreeMap::from([(BOUNTY_POINTS.to_owned(), 22)]);
    for path in 0..TALISMAN_PATHS.len() {
        let key = talisman_upgrade_transition(path, 0).expect("first upgrade");
        if path == 0 {
            apply(&catalogue, &mut stored, &key, NOW).expect("upgrade");
            assert_eq!(state(&stored).bounty_points, 17);
            assert_eq!(state(&stored).talisman_levels[path], 1);
            assert_eq!(talisman_bonus(path, 1), 300);
        } else {
            assert!(catalogue.transition(&key).is_some());
        }
    }
    let second = talisman_upgrade_transition(0, 1).expect("second upgrade");
    apply(&catalogue, &mut stored, &second, NOW).expect("second upgrade");
    assert_eq!(state(&stored).bounty_points, 0);
    assert_eq!(talisman_upgrade_cost(1), Some(17));
    assert_eq!(talisman_bonus(3, 1), 600);
    assert_eq!(
        apply(&catalogue, &mut stored, &second, NOW),
        Err(QuestRefusal::StageMismatch),
        "a stale level cannot spend twice"
    );
}

#[test]
fn talisman_curve_and_upgrade_boundaries_match_the_canary_profile() {
    for (level, ordinary) in [
        (0, 0),
        (1, 300),
        (15, 1_000),
        (16, 1_025),
        (55, 2_000),
        (56, 2_010),
        (355, 5_000),
    ] {
        for path in 0..4 {
            assert_eq!(
                talisman_bonus(path, level),
                if path == 3 { ordinary * 2 } else { ordinary }
            );
        }
    }
    assert_eq!(talisman_upgrade_cost(354), Some(4_253));
    for level in [-1, 355, i64::MAX] {
        assert_eq!(talisman_upgrade_cost(level), None);
        assert_eq!(talisman_upgrade_transition(0, level), None);
    }
    assert_eq!(talisman_upgrade_transition(4, 0), None);
}

#[test]
fn legacy_weekly_selection_does_not_activate_a_bounty_talisman() {
    let tracks = BTreeMap::from([(SELECTED.to_owned(), 1), (TALISMAN_LOOT.to_owned(), 1)]);
    assert_eq!(
        full::TaskBoardData::default().active_talisman_bonuses(&tracks, TASKS[0].race, true),
        [0; 4]
    );
}

#[test]
fn a_claimed_task_is_locked_for_seven_days() {
    let catalogue = catalogue();
    let mut stored = BTreeMap::from([(claimed_at_track(2), NOW)]);
    assert_eq!(
        apply(
            &catalogue,
            &mut stored,
            &select_transition(2),
            NOW + LOCKOUT_SECONDS - 1
        ),
        Err(QuestRefusal::StageMismatch)
    );
    apply(&catalogue, &mut stored, &select_transition(3), NOW).expect("another task is open");
    apply(
        &catalogue,
        &mut stored,
        &select_transition(2),
        NOW + LOCKOUT_SECONDS,
    )
    .expect_err("one active task at a time");
    stored.insert(SELECTED.to_owned(), 0);
    apply(
        &catalogue,
        &mut stored,
        &select_transition(2),
        NOW + LOCKOUT_SECONDS,
    )
    .expect("selectable after seven days");
}

#[test]
fn only_the_active_task_of_the_killed_race_is_credited() {
    let catalogue = catalogue();
    let mut stored = BTreeMap::new();
    assert_eq!(kill_credit(&stored, TASKS[0].race), None, "no active task");
    apply(&catalogue, &mut stored, &select_transition(1), NOW).expect("select");
    assert_eq!(kill_credit(&stored, TASKS[1].race), None);
    assert_eq!(kill_credit(&stored, "oteryn:creature.dragon"), None);
    assert_eq!(
        apply(&catalogue, &mut stored, &kill_transition(2), NOW),
        Err(QuestRefusal::StageMismatch),
        "the writer refuses another task's kill"
    );
}

#[test]
fn with_quest_refuses_a_quest_already_present_and_keeps_other_hashes() {
    let catalogue = catalogue();
    assert!(matches!(
        extend(&catalogue),
        Err(QuestCatalogueError::DuplicateKey(_))
    ));
    let other = QuestTrack {
        key: "oteryn:other/stage".to_owned(),
        quest: "oteryn:other".to_owned(),
        initial: 0,
        min: 0,
        max: 1,
    };
    let base = QuestStateCatalogue::new("content-1", vec![other], Vec::new()).expect("base");
    let extended = extend(&base).expect("extended");
    assert_eq!(
        extended.definition_hash("oteryn:other"),
        base.definition_hash("oteryn:other")
    );
    assert_eq!(extended.content_revision(), "content-1");
    assert_eq!(
        extended.definition_hash(QUEST),
        catalogue.definition_hash(QUEST)
    );
}
