use super::*;

fn missed(kills_without_drop: u16) -> PityState {
    PityState {
        kills_without_drop,
        first_drop_received: false,
    }
}

#[test]
fn bonus_starts_at_base_doubles_after_ten_misses_and_caps_at_three_times() {
    assert_eq!(missed(0).bonus_permille(), 0);
    assert_eq!(
        boosted_probability_ppm(3_000, missed(0).bonus_permille()),
        3_000
    );
    assert_eq!(missed(1).bonus_permille(), 100);
    assert_eq!(
        boosted_probability_ppm(3_000, missed(1).bonus_permille()),
        3_300
    );
    assert_eq!(missed(10).bonus_permille(), 1_000);
    assert_eq!(
        boosted_probability_ppm(3_000, missed(10).bonus_permille()),
        6_000
    );
    for kills in [20, 21, 500, PITY_KILLS_MAX, u16::MAX] {
        assert_eq!(missed(kills).bonus_permille(), 2_000, "kills {kills}");
        assert_eq!(
            boosted_probability_ppm(3_000, missed(kills).bonus_permille()),
            9_000
        );
    }
    assert_eq!(boosted_probability_ppm(600_000, 2_000), 1_000_000);
}

#[test]
fn first_drop_turns_the_tracker_off_and_only_base_chance_remains() {
    let mut state = PityState::default();
    assert!(state.tracker_active());
    for expected in 1..=12 {
        state = state.after_victory(false);
        assert_eq!(state, missed(expected));
    }
    state = state.after_victory(true);
    assert_eq!(
        state,
        PityState {
            kills_without_drop: 12,
            first_drop_received: true,
        }
    );
    assert!(!state.tracker_active());
    assert_eq!(state.bonus_permille(), 0);
    assert_eq!(
        boosted_probability_ppm(3_000, state.bonus_permille()),
        3_000
    );
    assert_eq!(state.after_victory(false), state);
    assert_eq!(state.after_victory(true), state);
    assert_eq!(
        missed(PITY_KILLS_MAX).after_victory(false),
        missed(PITY_KILLS_MAX)
    );
}

#[test]
fn only_difficulty_bosses_and_moonsilver_equipment_take_part() {
    assert_eq!(
        pity_boss_key("oteryn:creature.mimar_haffar").as_deref(),
        Some("oteryn:boss.mimar_haffar")
    );
    assert_eq!(
        pity_boss_key("oteryn:boss.maior_domus").as_deref(),
        Some("oteryn:boss.maior_domus")
    );
    assert_eq!(
        pity_boss_key("oteryn:boss.phosphorus").as_deref(),
        Some("oteryn:boss.phosphorus")
    );
    assert_eq!(pity_boss_key("oteryn:boss.reward_fixture"), None);
    for id in [53_207, 53_229, 53_233] {
        assert!(is_pity_item(&format!("oteryn:item.tibia.i{id}")), "{id}");
    }
    for key in [
        "oteryn:item.tibia.i53206",
        "oteryn:item.tibia.i53234",
        "oteryn:item.tibia.i54267",
        "oteryn:item.tibia.i3031",
        "oteryn:item.other.i53229",
    ] {
        assert!(!is_pity_item(key), "{key}");
    }
}
