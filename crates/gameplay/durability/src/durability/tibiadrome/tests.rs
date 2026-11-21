use super::*;

#[test]
fn srx3_drome_wave_zero_and_replay_below_best_do_not_invent_start_levels() {
    let mut progress = Progress::default();
    progress.wave(0);
    assert_eq!((progress.level, progress.start, progress.waves), (1, 1, 1));
    progress.wave(7);
    assert_eq!((progress.level, progress.start, progress.waves), (8, 8, 2));
    progress.wave(2);
    assert_eq!((progress.level, progress.start, progress.waves), (8, 8, 3));
    progress.wave(299);
    assert_eq!(
        (progress.level, progress.start, progress.waves),
        (300, 300, 4)
    );
    let before = progress.clone();
    progress.wave(300);
    assert_eq!(progress, before, "reaching wave 300 ends the battle");
    progress.wave(301);
    assert_eq!(progress, before, "ARENA1-RL-08 rejects waves above 300");
}

#[test]
fn reaudit_arena_lifecycle_scoring_rotation_and_rewards() {
    let mut state = Progress::default();
    assert_eq!(state.rotate(1), 0);
    state.wave(0);
    state.wave(1);
    assert_eq!(state.level, 2);
    assert_eq!(state.start, 2);
    state.finish(100);
    assert_eq!(state.points, 0, "points are paid only at rotation end");
    assert_eq!(state.start, 0);
    assert_eq!(state.cooldown_until, 3700);
    assert_eq!(state.rotate(2), 20);
    assert_eq!(state.rotate(2), 0, "a rotation pays once");
    assert_eq!(state.points, 20);
    assert_eq!(state.level, 0);
    assert_eq!(
        (
            rank_potions(1),
            rank_potions(5),
            rank_potions(6),
            rank_potions(10),
            rank_potions(11),
            rank_potions(20),
            rank_potions(21)
        ),
        (3, 3, 2, 2, 1, 1, 0)
    );
}

#[test]
fn reaudit_arena_lifecycle_access_and_group_limits() {
    for size in 1..=5 {
        assert_eq!(monster_count(size), Some([4, 8, 11, 14, 17][size - 1]));
    }
    assert_eq!(monster_count(0), None);
    assert_eq!(monster_count(6), None);
    assert!(!eligible(49, true));
    assert!(!eligible(50, false));
    assert!(eligible(50, true));
}

#[test]
fn reaudit_arena_lifecycle_wave_and_reset_never_lower_the_best_score() {
    let mut state = Progress {
        start: 25,
        ..Progress::default()
    };
    state.wave(25);
    state.finish(100);
    assert_eq!(state.start, 21);
    state.wave(21);
    assert_eq!(state.level, 26);
    assert_eq!(state.rotate(3), 260);
    assert_eq!(state.start, 7, "three elapsed rotations each subtract five");
}

#[test]
fn srx3_drome_lifetime_levels_add_only_new_best_delta_and_survive_rotation()
-> Result<(), serde_json::Error> {
    let mut state = Progress::default();
    assert_eq!(
        serde_json::to_value(&state)?["lifetime_levels"],
        serde_json::json!(0)
    );
    state.wave(0);
    state.wave(7);
    state.wave(2);
    assert_eq!(
        serde_json::to_value(&state)?["lifetime_levels"],
        serde_json::json!(8)
    );
    assert_eq!(state.waves, 3, "replays still count as participation");
    state.rotate(1);
    state.wave(4);
    assert_eq!(
        serde_json::to_value(&state)?["lifetime_levels"],
        serde_json::json!(13)
    );
    Ok(())
}
#[test]
fn srx3_drome_legacy_lifetime_baseline_remains_explicitly_unknown() -> Result<(), serde_json::Error>
{
    let mut legacy = serde_json::to_value(Progress {
        level: 20,
        start: 20,
        ..Progress::default()
    })?;
    if let Some(fields) = legacy.as_object_mut() {
        fields.remove("lifetime_levels");
    }
    let mut state: Progress = serde_json::from_value(legacy)?;
    state.wave(25);
    let encoded = serde_json::to_value(state)?;
    assert!(encoded.get("lifetime_levels").is_some());
    assert!(
        encoded["lifetime_levels"].is_null(),
        "historical totals need owner backfill policy"
    );
    Ok(())
}

#[test]
fn srx3_drome_lifetime_highscores_category_is_served_from_existing_world_job() {
    use crate::durability::highscore_snapshot::{
        LOYALTY_CATEGORY, TIBIADROME_LIFETIME_CATEGORY, TIBIADROME_ROTATION_CATEGORY,
        served_category,
    };
    // Main's ranking owner supersedes SRX-3's provisional category13; see
    // SRX-3-2026-11-05's shipping-merge reconciliation (categories14/17).
    assert_eq!(TIBIADROME_LIFETIME_CATEGORY, 14);
    assert_eq!(TIBIADROME_ROTATION_CATEGORY, 17);
    assert!(served_category(TIBIADROME_LIFETIME_CATEGORY));
    assert!(served_category(TIBIADROME_ROTATION_CATEGORY));
    assert_eq!(LOYALTY_CATEGORY, 13);
    assert!(served_category(LOYALTY_CATEGORY));
    assert_ne!(TIBIADROME_LIFETIME_CATEGORY, LOYALTY_CATEGORY);
    assert_ne!(TIBIADROME_ROTATION_CATEGORY, LOYALTY_CATEGORY);
    assert!(!served_category(15));
}
