use super::*;

#[test]
fn reaudit_arena_lifecycle_scoring_rotation_and_rewards() {
    let mut state = Progress::default();
    assert_eq!(state.rotate(1), 0);
    state.wave(1);
    state.wave(2);
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
    state.wave(26);
    state.finish(100);
    assert_eq!(state.start, 21);
    state.wave(22);
    assert_eq!(state.level, 26);
    assert_eq!(state.rotate(3), 260);
    assert_eq!(state.start, 7, "three elapsed rotations each subtract five");
}
