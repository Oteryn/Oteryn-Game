use super::*;
use std::time::{Duration, Instant};
fn entry(id: u32, completed: bool) -> QuestListEntry {
    QuestListEntry {
        quest_index: id,
        completed,
    }
}
fn line(id: u32, completed: bool) -> QuestLine {
    QuestLine {
        quest_index: id,
        completed,
        missions: vec![QuestMission {
            mission_index: 0,
            name: "Tower key".into(),
            text: "Find the key".into(),
            done: completed,
        }],
    }
}
#[test]
fn auto_tracking_baselines_old_quests_and_waits_for_server_confirmation() {
    let now = Instant::now();
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.tracker.auto_add = true;
    p.tracker.observe(&[entry(1, false)]);
    assert_eq!(tracker_query(&mut p, now), Some(QuestLogQuery::ListOnly));
    p.tracker
        .observe(&[entry(1, false), entry(2, false), entry(3, true)]);
    p.state.list = vec![entry(1, false), entry(2, false), entry(3, true)];
    let expected = QuestLogQuery::Track {
        quest_indexes: vec![2],
    };
    assert_eq!(tracker_query(&mut p, now), Some(expected));
    assert!(p.state.log.tracked.is_empty());
    // A refusal must not produce a tight retry loop.
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(1)), None);
    p.state.log.tracked = vec![line(2, false)];
    p.tracker.observe(&p.state.list);
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(2)), None);
    // Explicitly untracking an already observed quest never auto-adds it again.
    p.state.log.tracked.clear();
    p.tracker.observe(&p.state.list);
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(3)), None);
}
#[test]
fn completed_removal_and_tracker_filters_preserve_authoritative_lines() {
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.state.log.tracked = vec![line(1, true), line(2, false)];
    p.tracker.auto_remove = true;
    let now = Instant::now();
    assert_eq!(
        tracker_query(&mut p, now),
        Some(QuestLogQuery::Track {
            quest_indexes: vec![2]
        })
    );
    assert_eq!(p.state.log.tracked.len(), 2);
    assert_eq!(
        tracker_query(&mut p, now + Duration::from_secs(1)),
        Some(QuestLogQuery::ListOnly)
    );
    p.tracker.filter = TrackerFilter::Active;
    p.tracker.search = "KEY".into();
    assert_eq!(
        tracker_lines(&p)
            .iter()
            .map(|l| l.quest_index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    p.tracker.filter = TrackerFilter::Completed;
    assert_eq!(tracker_lines(&p)[0].quest_index, 1);
    p.tracker.search = "unmatched".into();
    assert!(tracker_lines(&p).is_empty());
}
#[test]
fn automation_never_exceeds_ten_slots() -> Result<(), String> {
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.tracker.auto_add = true;
    p.tracker.observe(&[]);
    p.state.log.tracked = (0..9).map(|id| line(id, false)).collect();
    p.tracker.observe(&[entry(10, false), entry(11, false)]);
    let Some(QuestLogQuery::Track { quest_indexes }) = tracker_query(&mut p, Instant::now()) else {
        return Err("Expected a tracking command".into());
    };
    assert_eq!(quest_indexes.len(), 10);
    assert_eq!(quest_indexes.last(), Some(&10));
    Ok(())
}

#[test]
fn queued_new_quests_wait_for_the_previous_tracking_delta() {
    let now = Instant::now();
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.tracker.auto_add = true;
    p.tracker.observe(&[]);
    p.tracker.observe(&[entry(1, false)]);
    assert_eq!(
        tracker_query(&mut p, now),
        Some(QuestLogQuery::Track {
            quest_indexes: vec![1]
        })
    );
    p.tracker.observe(&[entry(1, false), entry(2, false)]);
    // A command result can precede its domain delta: never replace [1] with [2].
    assert_eq!(
        tracker_query(&mut p, now + Duration::from_secs(2)),
        Some(QuestLogQuery::ListOnly)
    );
    p.state.log.tracked = vec![line(1, false)];
    assert_eq!(
        tracker_query(&mut p, now + Duration::from_secs(3)),
        Some(QuestLogQuery::Track {
            quest_indexes: vec![1, 2]
        })
    );
}

#[test]
fn rejected_new_quest_additions_retry_after_refresh_without_a_busy_loop() {
    let now = Instant::now();
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.tracker.auto_add = true;
    p.tracker.observe(&[]);
    p.tracker.observe(&[entry(1, false)]);
    let expected = QuestLogQuery::Track {
        quest_indexes: vec![1],
    };
    assert_eq!(tracker_query(&mut p, now), Some(expected.clone()));
    p.tracker.command_failed();
    assert_eq!(
        tracker_query(&mut p, now + Duration::from_secs(1)),
        Some(QuestLogQuery::ListOnly)
    );
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(2)), None);
    p.tracker.observe(&[entry(1, false)]);
    assert_eq!(
        tracker_query(&mut p, now + Duration::from_secs(3)),
        Some(expected)
    );
    assert!(p.state.log.tracked.is_empty());
    p.state.log.tracked = vec![line(1, false)];
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(4)), None);
    p.state.log.tracked.clear();
    p.tracker.observe(&[entry(1, false)]);
    assert_eq!(tracker_query(&mut p, now + Duration::from_secs(5)), None);
}

#[test]
fn independent_tracker_renders_only_matching_confirmed_quests() {
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.state.log.tracked = vec![line(1, false), line(2, true)];
    p.tracker.filter = TrackerFilter::Active;
    p.tracker.search = " TOWER ".into();
    let text = tracker_text(&p);
    assert!(text.contains("Quest 1"));
    assert!(text.contains("Tower key"));
    assert!(!text.contains("Quest 2"));
    assert_eq!(p.state.log.tracked.len(), 2);
}

#[test]
fn tracker_windows_do_not_overwrite_each_others_search() {
    let mut p = QuestPanel::default();
    p.tracker.search = "saved".into();
    let mut previous = None;
    assert_eq!(
        sync_tracker_search(&mut p, &mut previous, "".into()),
        Some("saved".into())
    );
    p.tracker.search = "other window".into();
    assert_eq!(
        sync_tracker_search(&mut p, &mut previous, "saved".into()),
        Some("other window".into())
    );
    assert_eq!(
        sync_tracker_search(&mut p, &mut previous, "typed".into()),
        None
    );
    assert_eq!(p.tracker.search, "typed");
}
