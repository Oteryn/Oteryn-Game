use super::*;
use std::collections::BTreeMap;

fn catalogue() -> Result<QuestStateCatalogue, String> {
    extend(QuestStateCatalogue::empty("spike-native-test").ok_or("empty catalogue")?)
}

#[test]
fn separate_source_blocks_can_start_two_pure_tasks_in_one_transaction() -> Result<(), String> {
    let book = dialogue::load()?;
    let tracks = BTreeMap::new();
    let selected = select(
        &book,
        "oteryn:npc.gnombold",
        None,
        "nests kill yes",
        &Facts {
            tracks: &tracks,
            level: 50,
            now: 1,
        },
    )
    .ok_or("compound")?;
    assert_eq!(selected.plan.starts, vec!["nest", "kill"]);
    let catalogue = catalogue()?;
    let transition = catalogue
        .transition(selected.transition.as_deref().ok_or("transition")?)
        .ok_or("catalogue transition")?;
    assert_eq!(transition.effects.len(), 2);
    assert_eq!(
        catalogue
            .evaluate(transition, &tracks, 1)
            .map_err(|e| format!("{e:?}"))?
            .len(),
        2
    );
    Ok(())
}

#[test]
fn inventory_dependent_compound_utterance_is_not_partially_activated() -> Result<(), String> {
    let book = dialogue::load()?;
    let tracks = BTreeMap::new();
    let facts = Facts {
        tracks: &tracks,
        level: 50,
        now: 1,
    };
    let complete = book
        .respond("oteryn:npc.gnombold", None, "charges nests yes", &facts)
        .ok_or("source plan")?;
    assert_eq!(complete.starts, vec!["charge", "nest"]);
    assert!(complete.needs_inventory);
    assert!(
        select(
            &book,
            "oteryn:npc.gnombold",
            None,
            "charges nests yes",
            &facts
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn held_yes_does_not_repeat_offer_time_main_or_level_guards() -> Result<(), String> {
    let book = dialogue::load()?;
    let task = book.task("upper", "kill").ok_or("task")?;
    let tracks = BTreeMap::from([(main_track(task), 7), (report_track(task), 100)]);
    let selected = select(
        &book,
        "oteryn:npc.gnomilly",
        Some("kill"),
        "yes",
        &Facts {
            tracks: &tracks,
            level: 1,
            now: 101,
        },
    )
    .ok_or("held yes")?;
    let catalogue = catalogue()?;
    let transition = catalogue
        .transition(selected.transition.as_deref().ok_or("transition")?)
        .ok_or("catalogue transition")?;
    let changes = catalogue
        .evaluate(transition, &tracks, 101)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].after, 0);
    Ok(())
}

#[test]
fn report_has_no_level_gate_and_atomically_writes_xp_fame_reset_and_clock() -> Result<(), String> {
    let book = dialogue::load()?;
    let task = book.task("lower", "kill").ok_or("task")?;
    let tracks = BTreeMap::from([(main_track(task), 7)]);
    let selected = select(
        &book,
        "oteryn:npc.gnomargery",
        Some("report"),
        "kill",
        &Facts {
            tracks: &tracks,
            level: 1,
            now: 100,
        },
    )
    .ok_or("report")?;
    let catalogue = catalogue()?;
    let transition = catalogue
        .transition(selected.transition.as_deref().ok_or("transition")?)
        .ok_or("catalogue transition")?;
    assert_eq!(transition.experience, Some(3500));
    let changes = catalogue
        .evaluate(transition, &tracks, 100)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 3);
    assert!(
        changes.iter().any(|change| change.track == dialogue::FAME
            && change.before == -1
            && change.after == 1)
    );
    assert!(
        changes
            .iter()
            .any(|change| change.track == report_track(task) && change.after == 100)
    );
    assert!(
        changes
            .iter()
            .any(|change| change.track == main_track(task) && change.after == -1)
    );
    let mut after = tracks;
    after.extend(
        changes
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    assert_eq!(dialogue::source_daily(task, &after), Some(72100));
    assert!(catalogue.evaluate(transition, &after, 101).is_err());
    Ok(())
}

#[test]
fn donor_bug_fix_fame_uses_the_explicit_storage_and_never_overflows_partial_reward()
-> Result<(), String> {
    let book = dialogue::load()?;
    let task = book.task("upper", "kill").ok_or("task")?;
    let catalogue = catalogue()?;
    let transition = catalogue
        .transition(&report_key("upper", "kill", false))
        .ok_or("report")?;
    let tracks = BTreeMap::from([(main_track(task), 7), (dialogue::FAME.into(), i64::MAX)]);
    assert!(catalogue.evaluate(transition, &tracks, 100).is_err());
    assert_eq!(
        catalogue.track(dialogue::FAME).ok_or("fame track")?.initial,
        -1
    );
    assert!(
        transition
            .effects
            .iter()
            .all(|effect| !effect.track.contains("constants"))
    );
    Ok(())
}

#[test]
fn donor_bug_fix_full_report_clamps_negative_fame_and_never_loses_xp_or_reset() -> Result<(), String>
{
    let book = dialogue::load()?;
    let task = book.task("upper", "kill").ok_or("task")?;
    let catalogue = catalogue()?;
    for before in [i64::MIN, -2, -1, 0, 8] {
        let tracks = BTreeMap::from([(main_track(task), 7), (dialogue::FAME.into(), before)]);
        let selected = select(
            &book,
            "oteryn:npc.gnomilly",
            Some("report"),
            "killing",
            &Facts {
                tracks: &tracks,
                level: 1,
                now: 100,
            },
        )
        .ok_or("report selection")?;
        let transition = catalogue
            .transition(selected.transition.as_deref().ok_or("report key")?)
            .ok_or("report transition")?;
        assert_eq!(transition.experience, Some(1000));
        let changes = catalogue
            .evaluate(transition, &tracks, 100)
            .map_err(|error| format!("{error:?}"))?;
        let mut after = tracks;
        after.extend(
            changes
                .into_iter()
                .map(|change| (change.track, change.after)),
        );
        assert_eq!(after.get(dialogue::FAME), Some(&(before.max(0) + 1)));
        assert_eq!(after.get(&main_track(task)), Some(&-1));
        assert_eq!(after.get(&report_track(task)), Some(&100));
        assert_eq!(dialogue::source_daily(task, &after), Some(72100));
        assert!(
            catalogue.evaluate(transition, &after, 101).is_err(),
            "no duplicate reward"
        );
    }
    Ok(())
}

#[test]
fn existing_lowered_counter_keeps_its_wider_bounds_owner_and_saved_zero() -> Result<(), String> {
    let book = dialogue::load()?;
    let task = book.task("upper", "kill").ok_or("task")?;
    // Exact baseline lowering: QuestState uses source_storage_width for ADD.
    // content/quests/missions/quest-state.json, Spike Upper Kill Main.
    let track = QuestTrack {
        key: main_track(task),
        quest: QUEST.into(),
        initial: -1,
        min: -1,
        max: i64::from(i32::MAX),
    };
    let baseline = QuestStateCatalogue::new("source-range", vec![track.clone()], Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    let merged = extend(baseline)?;
    assert_eq!(merged.track(&track.key), Some(&track));
    let saved = BTreeMap::from([(track.key.clone(), 0)]);
    let selected = select(
        &book,
        "oteryn:npc.gnomilly",
        Some("report"),
        "killing",
        &Facts {
            tracks: &saved,
            level: 1,
            now: 100,
        },
    )
    .ok_or("incomplete report")?;
    assert!(selected.transition.is_none());
    assert_eq!(saved.get(&track.key), Some(&0));
    let foreign = QuestTrack {
        quest: "oteryn:quest.other".into(),
        ..track
    };
    let baseline = QuestStateCatalogue::new("foreign-owner", vec![foreign], Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    assert!(extend(baseline).is_err());
    Ok(())
}
