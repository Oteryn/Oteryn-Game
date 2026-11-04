// Test-only: `expect` names the scripted precondition that failed.
#![allow(clippy::expect_used)]

use std::{cell::Cell, collections::VecDeque};

use super::*;

const CENTRE: MovementLocalPosition = MovementLocalPosition {
    x: 33_600,
    y: 32_800,
    floor: 15,
};

fn facts(now_ms: u64) -> PrimalMenaceThinkFacts<'static> {
    PrimalMenaceThinkFacts {
        position: CENTRE,
        now_ms,
        health: 1_000_000,
        max_health: 1_000_000,
        damage_map_levels: &[],
    }
}

fn draw_queue(draws: Vec<u32>) -> impl FnMut() -> u32 {
    let mut draws = VecDeque::from(draws);
    move || draws.pop_front().expect("test draw queue exhausted")
}

fn initialise(owner: &mut PrimalMenaceThinkOwner) {
    let effects = owner
        .think(&facts(0), || unreachable!(), |_| true, |_| None)
        .expect("initial think should succeed");
    assert!(effects.is_empty());
}

#[test]
fn wave_formulas_match_the_donor() {
    assert_eq!(hazard_points(&[]), 1);
    assert_eq!(hazard_points(&[3, 1, 2]), 1);
    assert_eq!(hazard_points(&[4, 5]), 4);

    let five_tenths = (30_000.0 * 0.98_f64.powi(5)).floor() as u64;
    let hazard_ten = (30_000.0 * 0.985_f64.powi(10)).floor() as u64;
    assert_eq!(five_tenths, 27_117);
    assert_eq!(hazard_ten, 25_791);
    assert_eq!(
        wave_interval_ms(&BEAST_WAVE, 1_000_000, 1_000_000, 0),
        30_000
    );
    assert_eq!(
        wave_interval_ms(&BEAST_WAVE, 450_000, 1_000_000, 0),
        five_tenths
    );
    assert_eq!(
        wave_interval_ms(&BEAST_WAVE, 1_000_000, 1_000_000, 10),
        hazard_ten
    );

    assert_eq!(wave_count(&BEAST_WAVE, 0, 0, 999_999), 4);
    assert_eq!(wave_count(&BEAST_WAVE, 0, 999_999, 0), 3);
    assert_eq!(wave_count(&BEAST_WAVE, 10, 999_999, 999_999), 6);
    assert_eq!(wave_count(&POD_WAVE, 0, 0, 0), 2);
    assert_eq!(wave_count(&POD_WAVE, 10, 0, 999_999), 4);
    assert_eq!(pick_beast(0), Some(PRIMAL_BEAST_POOL[0]));
    assert_eq!(pick_beast(999_999), Some(PRIMAL_BEAST_POOL[10]));
    assert_eq!(pick_beast(1_000_000), None);
    assert_eq!(PRIMAL_MENACE_KEY, "oteryn:creature.the_primal_menace");
    assert_eq!(FUNGOSAURUS_KEY, "oteryn:creature.fungosaurus");
}

#[test]
fn first_think_initialises_and_the_first_beast_wave_fires_after_ten_seconds() {
    let mut owner = PrimalMenaceThinkOwner::default();
    initialise(&mut owner);
    assert_eq!(owner.centre(), Some(CENTRE));
    assert_eq!(owner.next_waves_ms(), (10_000, 20_000));
    assert!(
        owner
            .think(&facts(10_000), || unreachable!(), |_| true, |_| None)
            .expect("boundary think should succeed")
            .is_empty()
    );

    let mut draws = vec![0, 999_999];
    draws.extend([200_000, 999_999, 200_000, 0].repeat(4));
    let effects = owner
        .think(&facts(10_001), draw_queue(draws), |_| true, |_| None)
        .expect("beast wave should succeed");
    let position = MovementLocalPosition {
        x: 33_601,
        y: 32_799,
        floor: 15,
    };
    let expected_spawn = [
        PrimalMenaceEffect::SpawnSparkle {
            position,
            at_ms: 11_001,
        },
        PrimalMenaceEffect::SpawnSparkle {
            position,
            at_ms: 12_001,
        },
        PrimalMenaceEffect::SpawnSparkle {
            position,
            at_ms: 13_001,
        },
        PrimalMenaceEffect::ScheduledSpawn {
            kind: WaveKind::Beast,
            position,
            at_ms: 13_001,
        },
    ];
    assert_eq!(effects, expected_spawn.repeat(4));
    assert_eq!(owner.next_waves_ms().0, 39_551);
}

#[test]
fn a_blocked_search_falls_back_to_the_centre_after_six_tries() {
    let mut owner = PrimalMenaceThinkOwner::default();
    initialise(&mut owner);
    let consumed = Cell::new(0);
    let draws = [0, 999_999]
        .into_iter()
        .chain([200_000, 999_999, 200_000, 0].repeat(24));
    let mut draws = VecDeque::from_iter(draws);
    let effects = owner
        .think(
            &facts(10_001),
            || {
                consumed.set(consumed.get() + 1);
                draws.pop_front().expect("test draw queue exhausted")
            },
            |_| false,
            |_| None,
        )
        .expect("blocked wave should succeed");
    assert_eq!(consumed.get(), 98);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, PrimalMenaceEffect::ScheduledSpawn { position, .. } if *position == CENTRE))
            .count(),
        4
    );
}

#[test]
fn the_pod_wave_uses_pod_counts_and_kind() {
    let mut owner = PrimalMenaceThinkOwner::default();
    initialise(&mut owner);
    let mut beast_draws = vec![0, 999_999];
    beast_draws.extend([200_000, 999_999, 200_000, 0].repeat(4));
    owner
        .think(&facts(10_001), draw_queue(beast_draws), |_| true, |_| None)
        .expect("beast wave should succeed");

    let mut pod_draws = vec![0, 999_999];
    pod_draws.extend([200_000, 999_999, 200_000, 0].repeat(2));
    let effects = owner
        .think(&facts(20_001), draw_queue(pod_draws), |_| true, |_| None)
        .expect("pod wave should succeed");
    let position = MovementLocalPosition {
        x: 33_601,
        y: 32_799,
        floor: 15,
    };
    assert_eq!(effects.len(), 8);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| {
                matches!(
                    effect,
                    PrimalMenaceEffect::ScheduledSpawn {
                        kind: WaveKind::Pod,
                        position: p,
                        at_ms: 23_001
                    } if *p == position
                )
            })
            .count(),
        2
    );
    assert_eq!(owner.next_waves_ms().1, 49_551);
}

#[test]
fn beasts_older_than_twenty_seconds_turn_into_fungosaurus_and_dead_ones_are_dropped() {
    let mut owner = PrimalMenaceThinkOwner::default();
    owner.record_beast(1, 13_000);
    owner.record_beast(2, 13_000);
    owner.record_beast(3, 14_000);
    owner
        .think(
            &facts(30_000),
            || unreachable!(),
            |_| true,
            |actor| {
                Some(MovementLocalPosition {
                    x: actor as i32,
                    y: 1,
                    floor: 15,
                })
            },
        )
        .expect("initial think should succeed");
    let p1 = MovementLocalPosition {
        x: 1,
        y: 1,
        floor: 15,
    };
    let p3 = MovementLocalPosition {
        x: 3,
        y: 1,
        floor: 15,
    };
    let effects = owner
        .think(
            &facts(33_001),
            || unreachable!(),
            |_| true,
            |actor| match actor {
                1 => Some(p1),
                2 => None,
                3 => Some(p3),
                _ => unreachable!(),
            },
        )
        .expect("decay think should succeed");
    assert_eq!(
        effects,
        vec![PrimalMenaceEffect::ReplaceWithFungosaurus {
            beast: 1,
            position: p1,
        }]
    );
    assert_eq!(owner.beasts(), &[(3, 14_000)]);
    assert_eq!(
        owner
            .think(&facts(34_001), || unreachable!(), |_| true, |_| Some(p3))
            .expect("second decay think should succeed"),
        vec![PrimalMenaceEffect::ReplaceWithFungosaurus {
            beast: 3,
            position: p3,
        }]
    );
}

#[test]
fn an_out_of_range_draw_refuses_without_changing_the_owner() {
    let mut owner = PrimalMenaceThinkOwner::default();
    initialise(&mut owner);
    let before = owner.clone();
    assert_eq!(
        owner.think(&facts(10_001), || DRAW_PPM, |_| true, |_| None),
        Err(PrimalMenaceRefusal::InvalidDraw)
    );
    assert_eq!(owner, before);
}
