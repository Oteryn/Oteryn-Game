//! Engine tests of monk Harmony and Serene (SPELL-D8 §8.2 H-2 and
//! `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §A.2), and the admission gate that keeps the
//! Harmony spells out until the durable field (H-1) exists.

#![allow(clippy::expect_used)]

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::{Value, json};

use super::Vocation;
use super::authoring::spell_from_bundle;
use super::chain::{ChainCreature, TilePosition};
use super::harmony::{
    FOCUS_SERENITY_MICROS, HarmonyMultiplier, MULTIPLIER_DENOMINATOR, MonkState, MonkStateError,
    SereneWorld,
};
use super::party::SoloParty;

const SPELL: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-cure_poison/spell.json"
);
const DEPENDENCIES: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-cure_poison/dependencies.json"
);

const MONK: u64 = 1;

fn at(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

fn creature(id: u64, x: i32, y: i32, floor: i16) -> ChainCreature {
    ChainCreature {
        id,
        actor: format!("creature:{id}"),
        position: TilePosition { x, y, floor },
    }
}

fn solo() -> SoloParty {
    SoloParty(creature(MONK, 100, 100, 7))
}

/// A monk in a party, with `adjacent` creatures next to it.
struct Party {
    members: Vec<ChainCreature>,
    adjacent: u32,
}

impl Party {
    /// The monk at (100, 100, 7) and one more member at the offset `(dx, dy)` and `floor`.
    fn with_member(dx: i32, dy: i32, floor: i16, adjacent: u32) -> Self {
        Self {
            members: vec![
                creature(MONK, 100, 100, 7),
                creature(2, 100 + dx, 100 + dy, floor),
            ],
            adjacent,
        }
    }
}

impl SereneWorld for Party {
    fn monk(&self) -> &ChainCreature {
        &self.members[0]
    }

    fn party(&self) -> Option<&[ChainCreature]> {
        Some(&self.members)
    }

    fn adjacent_creatures(&self) -> u32 {
        self.adjacent
    }
}

/// A visible party member and 8 adjacent creatures: not Serene by the rule.
fn crowded() -> Party {
    Party::with_member(1, 0, 7, 8)
}

fn monk(harmony: u8) -> MonkState {
    MonkState::load(Vocation::Monk, harmony, 0)
        .expect("valid")
        .expect("monk")
}

fn ready(harmony: u8) -> MonkState {
    let mut state = monk(harmony);
    state.initialize(at(0), &solo()).expect("initialized");
    state
}

#[test]
fn harmony_loads_from_the_supplied_value_and_fails_closed_outside_its_range() {
    for value in 0..=5 {
        assert_eq!(monk(value).harmony(), value);
    }
    let exalted = MonkState::load(Vocation::ExaltedMonk, 4, 0).expect("valid");
    assert_eq!(exalted.map(|state| state.harmony()), Some(4));
    assert_eq!(
        MonkState::load(Vocation::Monk, 6, 0),
        Err(MonkStateError::CorruptHarmony {
            vocation: Vocation::Monk,
            value: 6
        })
    );
    // Another vocation has no Harmony state, and a stored value other than 0 is corrupt.
    assert_eq!(MonkState::load(Vocation::Knight, 0, 0), Ok(None));
    assert_eq!(
        MonkState::load(Vocation::Sorcerer, 1, 0),
        Err(MonkStateError::CorruptHarmony {
            vocation: Vocation::Sorcerer,
            value: 1
        })
    );
    assert_eq!(
        MonkState::load(Vocation::Monk, 0, FOCUS_SERENITY_MICROS + 1),
        Err(MonkStateError::CorruptSereneForced {
            vocation: Vocation::Monk,
            micros: FOCUS_SERENITY_MICROS + 1
        })
    );
    assert_eq!(
        MonkState::load(Vocation::Druid, 0, 1),
        Err(MonkStateError::CorruptSereneForced {
            vocation: Vocation::Druid,
            micros: 1
        })
    );
}

#[test]
fn no_command_is_accepted_before_the_initialization_evaluation() {
    let mut state = monk(3);
    assert!(!state.serene());
    assert_eq!(state.accept_command(), Err(MonkStateError::NotInitialized));
    assert_eq!(
        state.spender_multiplier(200, false),
        Err(MonkStateError::NotInitialized)
    );
    assert_eq!(state.commit_builder(), Err(MonkStateError::NotInitialized));
    assert_eq!(state.commit_spender(), Err(MonkStateError::NotInitialized));
    assert_eq!(state.commit_fill(), Err(MonkStateError::NotInitialized));
    assert_eq!(
        state.commit_focus_serenity(at(0)),
        Err(MonkStateError::NotInitialized)
    );
    assert_eq!(
        state.tick(at(5000), &solo()),
        Err(MonkStateError::NotInitialized)
    );
    assert_eq!(state, monk(3), "a refused command changes nothing");
    state.initialize(at(0), &solo()).expect("initialized");
    assert_eq!(state.accept_command(), Ok(()));
}

#[test]
fn a_solo_monk_is_serene_for_its_first_cast_with_no_periodic_step() {
    // Fresh admission: a new runtime actor from the supplied value.
    let mut state = monk(5);
    state.initialize(at(40), &solo()).expect("initialized");
    assert!(state.serene());
    // Virtue of Harmony while Serene doubles the base: level 200, 5 charges, 3.56.
    assert_eq!(
        state
            .spender_multiplier(200, true)
            .expect("ready")
            .numerator(),
        712_000
    );
    // A same-GameSession reconnect and FND-04B §21 recovery keep the actor and initialize it again.
    for reattach in [at(900), at(30_000)] {
        state.detach();
        assert_eq!(state.accept_command(), Err(MonkStateError::NotInitialized));
        state.initialize(reattach, &solo()).expect("initialized");
        assert!(state.serene());
        assert_eq!(state.harmony(), 5, "the actor's Harmony is kept exactly");
    }
}

#[test]
fn a_solo_monk_is_always_serene() {
    let mut state = ready(0);
    for second in 1..=5 {
        assert_eq!(state.tick(at(second * 1000), &solo()), Ok(false));
        assert!(state.serene());
    }
}

#[test]
fn serene_is_evaluated_every_1000_ms() {
    let mut state = ready(0);
    assert!(state.serene());
    // Not due yet: the crowd is not seen.
    assert_eq!(state.tick(at(999), &crowded()), Ok(false));
    assert!(state.serene());
    assert_eq!(state.tick(at(1000), &crowded()), Ok(true));
    assert!(!state.serene());
    // The next evaluation is 1000 ms after the last one.
    assert_eq!(state.tick(at(1999), &solo()), Ok(false));
    assert!(!state.serene());
    assert_eq!(state.tick(at(2000), &solo()), Ok(true));
    assert!(state.serene());
}

#[test]
fn a_visible_party_member_and_eight_adjacent_creatures_end_serene() {
    let serene_at_init = |world: &Party| {
        let mut state = monk(0);
        state.initialize(at(0), world).expect("initialized");
        state.serene()
    };
    assert!(serene_at_init(&Party::with_member(1, 0, 7, 7)));
    assert!(!serene_at_init(&Party::with_member(1, 0, 7, 8)));
    // The visible area: dx in [-8, 9], dy in [-6, 7], same floor.
    for (dx, dy) in [(-8, 0), (9, 0), (0, -6), (0, 7), (-8, -6), (9, 7)] {
        assert!(
            !serene_at_init(&Party::with_member(dx, dy, 7, 8)),
            "{dx},{dy}"
        );
    }
    for (dx, dy) in [(-9, 0), (10, 0), (0, -7), (0, 8)] {
        assert!(
            serene_at_init(&Party::with_member(dx, dy, 7, 8)),
            "{dx},{dy}"
        );
    }
    assert!(serene_at_init(&Party::with_member(1, 0, 6, 8)));
    // A party of the monk alone has no visible member.
    let alone = Party {
        members: vec![creature(MONK, 100, 100, 7)],
        adjacent: 20,
    };
    assert!(serene_at_init(&alone));
}

#[test]
fn focus_serenity_forces_serene_for_7000_ms() {
    let mut state = ready(0);
    state.tick(at(1000), &crowded()).expect("tick");
    assert!(!state.serene());
    state.commit_focus_serenity(at(1500)).expect("committed");
    assert!(state.serene());
    assert_eq!(state.serene_forced_until(), Some(at(8500)));
    for second in 2..=8 {
        state.tick(at(second * 1000), &crowded()).expect("tick");
        assert!(state.serene(), "forced at {second} s");
    }
    assert_eq!(state.tick(at(9000), &crowded()), Ok(true));
    assert!(!state.serene());
    assert_eq!(state.serene_forced_until(), None);
}

#[test]
fn a_forced_serene_set_before_a_reconnect_holds_until_its_time_runs_out() {
    let mut state = ready(2);
    state.commit_focus_serenity(at(500)).expect("committed");
    state.detach();
    state.initialize(at(3000), &crowded()).expect("initialized");
    assert!(state.serene());
    assert_eq!(state.serene_forced_until(), Some(at(7500)));
    assert_eq!(state.serene_forced_remaining(at(3000)), 4_500_000);
    state.tick(at(7000), &crowded()).expect("tick");
    assert!(state.serene());
    state.tick(at(8000), &crowded()).expect("tick");
    assert!(!state.serene());
    assert_eq!(state.serene_forced_remaining(at(8000)), 0);
}

#[test]
fn a_loaded_forced_serene_with_3_s_remaining_lasts_exactly_3_s() {
    let mut state = MonkState::load(Vocation::Monk, 1, 3_000_000)
        .expect("valid")
        .expect("monk");
    assert_eq!(state.serene_forced_remaining(at(0)), 3_000_000);
    // The loaded time starts at the initialization evaluation, not at the load.
    state
        .initialize(at(10_000), &crowded())
        .expect("initialized");
    assert!(state.serene());
    assert_eq!(state.serene_forced_until(), Some(at(13_000)));
    let mut probe = state.clone();
    probe
        .tick(SemanticTimeMicros::from_micros(12_999_999), &crowded())
        .expect("tick");
    assert!(probe.serene(), "still forced 1 us before the end");
    state.tick(at(11_000), &crowded()).expect("tick");
    state.tick(at(12_000), &crowded()).expect("tick");
    assert!(state.serene());
    assert_eq!(state.serene_forced_remaining(at(12_000)), 1_000_000);
    assert_eq!(state.tick(at(13_000), &crowded()), Ok(true));
    assert!(!state.serene());
    // A later reconnect does not start it again.
    state.detach();
    state
        .initialize(at(14_000), &crowded())
        .expect("initialized");
    assert!(!state.serene());
}

#[test]
fn a_new_runtime_actor_without_a_loaded_forced_time_is_not_forced() {
    let mut state = monk(5);
    state.initialize(at(0), &crowded()).expect("initialized");
    assert!(!state.serene());
    assert_eq!(state.serene_forced_until(), None);
    assert_eq!(state.serene_forced_remaining(at(0)), 0);
}

#[test]
fn focus_serenity_past_the_end_of_time_changes_nothing() {
    let mut state = ready(0);
    state.tick(at(1000), &crowded()).expect("tick");
    let before = state.clone();
    assert_eq!(
        state.commit_focus_serenity(SemanticTimeMicros::from_micros(u64::MAX)),
        Err(MonkStateError::TimeOverflow)
    );
    assert_eq!(state, before);
}

#[test]
fn the_harmony_multiplier_matches_the_engine_tests() {
    let ratio = |level, charges, virtue, serene| {
        HarmonyMultiplier::new(level, charges, virtue, serene)
            .expect("charges")
            .numerator()
    };
    let hundredths = |values: [u64; 5]| values.map(|value| value * MULTIPLIER_DENOMINATOR / 100);
    // Level 200 without a virtue (Serene does not matter): 1.08, 1.16, 1.32, 1.64, 2.28.
    for serene in [false, true] {
        assert_eq!(
            [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, false, serene)),
            hundredths([108, 116, 132, 164, 228])
        );
    }
    // Virtue of Harmony while Serene: 1.16, 1.32, 1.64, 2.28, 3.56.
    assert_eq!(
        [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, true, true)),
        hundredths([116, 132, 164, 228, 356])
    );
    // Virtue without Serene, 5 charges: 2.92; level 700 with virtue and Serene: 4.36.
    assert_eq!(
        ratio(200, 5, true, false),
        292 * MULTIPLIER_DENOMINATOR / 100
    );
    assert_eq!(
        ratio(700, 5, true, true),
        436 * MULTIPLIER_DENOMINATOR / 100
    );
    // No charges: 1; more than 5 cannot be held.
    assert_eq!(
        HarmonyMultiplier::new(200, 0, true, true),
        Some(HarmonyMultiplier::ONE)
    );
    assert_eq!(HarmonyMultiplier::new(200, 6, false, false), None);
}

#[test]
fn a_spender_bound_is_multiplied_and_then_truncated_toward_zero() {
    let multiplier = HarmonyMultiplier::new(200, 3, false, false).expect("charges");
    // x1.32: 37 -> 48.84 -> 48; 45 -> 59.4 -> 59; negative bounds truncate toward zero.
    assert_eq!(multiplier.apply(37), 48);
    assert_eq!(multiplier.apply(45), 59);
    assert_eq!(multiplier.apply(-37), -48);
    assert_eq!(multiplier.apply(0), 0);
    assert_eq!(HarmonyMultiplier::ONE.apply(37), 37);
}

#[test]
fn reading_the_spender_multiplier_keeps_harmony() {
    // A failed or rejected cast reads the multiplier but reaches no PRIMARY COMMIT.
    let state = ready(3);
    let before = state.clone();
    assert_eq!(
        state
            .spender_multiplier(200, false)
            .expect("ready")
            .numerator(),
        132 * MULTIPLIER_DENOMINATOR / 100
    );
    assert_eq!(state, before);
}

#[test]
fn a_builder_gains_one_charge_up_to_five() {
    let mut state = ready(0);
    for expected in 1..=5 {
        assert_eq!(state.commit_builder(), Ok(1));
        assert_eq!(state.harmony(), expected);
    }
    // At 5 nothing is gained (and so no Virtue Healing happens).
    assert_eq!(state.commit_builder(), Ok(0));
    assert_eq!(state.harmony(), 5);
}

#[test]
fn a_spender_spends_every_charge() {
    let mut state = ready(3);
    assert_eq!(state.commit_spender(), Ok(3));
    assert_eq!(state.harmony(), 0);
    assert_eq!(state.commit_spender(), Ok(0));
    assert_eq!(state.harmony(), 0);
}

#[test]
fn a_focus_fill_gains_the_missing_charges() {
    let mut state = ready(2);
    assert_eq!(state.commit_fill(), Ok(3));
    assert_eq!(state.harmony(), 5);
    assert_eq!(state.commit_fill(), Ok(0));
}

/// Admission (H-live): Harmony is durable (H-1) and the runtime actor builds and spends it, so a
/// monk builder or spender is admitted; `monk_focus` has no implementation and stays fail-closed.
#[test]
fn the_harmony_roles_are_admitted_and_monk_focus_stays_fail_closed() {
    let dependencies: Value = serde_json::from_str(DEPENDENCIES).expect("dependencies");
    for role in ["builder", "spender"] {
        let mut spell: Value = serde_json::from_str(SPELL).expect("spell");
        spell["spell"]["harmony_role"] = json!(role);
        spell["spell"]["requirements"]["vocations"] = json!(["monk", "exalted_monk"]);
        let admitted = spell_from_bundle(&spell, &dependencies).expect("harmony role admitted");
        assert!(admitted.harmony_role.is_some());
    }
    let mut spell: Value = serde_json::from_str(SPELL).expect("spell");
    spell["spell"]["execution"] = json!({ "native_behavior": {
        "key": "monk_focus",
        "parameters": { "fill_harmony": true, "serene_ms": 7000, "reset_spender_cooldowns": true }
    }});
    let error = spell_from_bundle(&spell, &dependencies).expect_err("monk_focus admitted");
    assert!(error.to_string().contains("monk_focus"), "{error}");
}
