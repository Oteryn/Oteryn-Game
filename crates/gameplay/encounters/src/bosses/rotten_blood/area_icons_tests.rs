#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

/// Explicit canonical-player fixture: applies published icons and persisted KV changes.
/// The callback under test never retains these facts itself.
#[derive(Debug, Default)]
struct CanonicalFixture {
    owner: RottenAreaIconOwner,
    current: BTreeMap<u64, RottenAreaCurrentFacts>,
}
impl CanonicalFixture {
    fn think(
        &mut self,
        minion: RoomPosition,
        now: u64,
        spectators: &[RottenAreaPlayer<'_>],
        draw: impl FnMut() -> u32,
    ) -> Result<Vec<RottenAreaEffect>, RottenAreaRefusal> {
        let mut current = self.current.clone();
        for player in spectators {
            current
                .entry(player.actor)
                .or_insert(RottenAreaCurrentFacts {
                    actor: player.actor,
                    ..RottenAreaCurrentFacts::default()
                });
        }
        let snapshot = current.values().copied().collect::<Vec<_>>();
        let effects = self
            .owner
            .think_current(minion, now, spectators, &snapshot, draw)?;
        for effect in &effects {
            let (actor, icon, count) = match *effect {
                RottenAreaEffect::SetIcon { actor, icon, count } => (actor, icon, Some(count)),
                RottenAreaEffect::RemoveIcon { actor, icon } => (actor, icon, None),
                RottenAreaEffect::EnergyDamage { .. } => continue,
            };
            let facts = current
                .get_mut(&actor)
                .expect("current canonical fixture Player");
            match icon {
                RottenAreaIcon::Hazard => facts.last_hazard = now,
                RottenAreaIcon::AreaArrowUp => {
                    facts.area_count = count;
                    facts.last_area = now;
                }
                RottenAreaIcon::WhiteCross => {
                    facts.cross_count = count;
                    facts.last_cross = now;
                }
            }
        }
        self.current = current;
        Ok(effects)
    }
    fn login(&mut self, actor: u64) -> Vec<RottenAreaEffect> {
        let effects = self.owner.login(actor);
        if let Some(current) = self.current.get_mut(&actor) {
            current.area_count = None;
            current.cross_count = None;
        }
        effects
    }
}

fn position(x: i32, y: i32, floor: i16) -> RoomPosition {
    RoomPosition { x, y, floor }
}

fn player(position: RoomPosition) -> RottenAreaPlayer<'static> {
    RottenAreaPlayer {
        actor: 1,
        position,
        taint_level: 3,
        party: None,
        alive: true,
    }
}

fn party_player<'a>(position: RoomPosition, levels: &'a [u8]) -> RottenAreaPlayer<'a> {
    RottenAreaPlayer {
        actor: 1,
        position,
        taint_level: 3,
        party: Some(RottenParty {
            leader: 1,
            taint_levels: levels,
        }),
        alive: true,
    }
}

fn icon_counts(effects: &[RottenAreaEffect], icon: RottenAreaIcon) -> Vec<u8> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            RottenAreaEffect::SetIcon {
                icon: actual,
                count,
                ..
            } if *actual == icon => Some(*count),
            _ => None,
        })
        .collect()
}

#[test]
fn rotten_area_minions_are_the_fifteen_crystal_callers_and_taint_maps_to_the_donor_icon_table() {
    assert!(ROTTEN_AREA_MINIONS.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(is_rotten_area_minion("oteryn:creature.converter"));
    assert!(!is_rotten_area_minion("oteryn:creature.bakragore"));
    assert_eq!(hazard_icon_level(0), None);
    assert_eq!(hazard_icon_level(1), Some(1));
    assert_eq!(hazard_icon_level(4), Some(4));
    assert_eq!(hazard_icon_level(5), None);
    assert_eq!(hazard_icon_level(6), Some(1));
    assert_eq!(hazard_icon_level(9), Some(4));
    assert_eq!(hazard_icon_level(10), None);

    let final_taint = RottenBloodState::from_stored(4, 3, 0, true, false, None)
        .expect("the stored Rotten Blood state should be valid");
    assert_eq!(donor_taint_level(&final_taint), 8);
    assert_eq!(donor_taint_level(&RottenBloodState::default()), 0);
}

#[test]
fn rotten_area_boxes_are_inclusive_on_floors_14_and_15() {
    assert!(in_upper_left_area(position(33_800, 31_653, 14)));
    assert!(in_upper_left_area(position(33_941, 31_771, 15)));
    assert!(!in_upper_left_area(position(33_942, 31_700, 14)));
    assert!(!in_upper_left_area(position(33_850, 31_700, 13)));
    assert!(in_lower_left_area(position(33_937, 31_931, 14)));
    assert!(!in_lower_left_area(position(33_938, 31_900, 14)));
    assert!(!in_lower_left_area(position(33_850, 31_808, 14)));
}

#[test]
fn hazard_icon_uses_party_minimum_every_eight_seconds_with_ten_second_leader_cache() {
    let mut owner = CanonicalFixture::default();
    let minion = position(33_850, 31_700, 14);
    let initial_levels = [3, 1];
    let changed_levels = [3, 2];
    let effects = owner
        .think(
            minion,
            1_000,
            &[party_player(minion, &initial_levels)],
            || 1_400,
        )
        .expect("initial think should succeed");
    assert!(effects.contains(&RottenAreaEffect::SetIcon {
        actor: 1,
        icon: RottenAreaIcon::Hazard,
        count: 1,
    }));
    let effects = owner
        .think(
            minion,
            1_007,
            &[party_player(minion, &changed_levels)],
            || 1_400,
        )
        .expect("think before refresh should succeed");
    assert!(icon_counts(&effects, RottenAreaIcon::Hazard).is_empty());
    let effects = owner
        .think(
            minion,
            1_008,
            &[party_player(minion, &changed_levels)],
            || 1_400,
        )
        .expect("cached refresh should succeed");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::Hazard), [1]);
    let effects = owner
        .think(
            minion,
            1_016,
            &[party_player(minion, &changed_levels)],
            || 1_400,
        )
        .expect("expired cache refresh should succeed");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::Hazard), [2]);

    let mut solo = player(minion);
    solo.actor = 2;
    solo.taint_level = 5;
    let effects = owner
        .think(minion, 1_017, &[solo], || 1_400)
        .expect("solo refresh should succeed");
    assert!(effects.contains(&RottenAreaEffect::RemoveIcon {
        actor: 2,
        icon: RottenAreaIcon::Hazard,
    }));
}

#[test]
fn upper_left_arrow_counts_every_thirty_seconds_and_caps_at_one_hundred() {
    let mut owner = CanonicalFixture::default();
    let minion = position(33_850, 31_700, 14);
    let spectator = player(minion);

    for step in 0_u64..105 {
        let now = 1_000 + AREA_STEP_SECONDS * step;
        let effects = owner
            .think(minion, now, &[spectator], || 1_400)
            .expect("scheduled area think should succeed");
        assert_eq!(
            icon_counts(&effects, RottenAreaIcon::AreaArrowUp),
            [u8::try_from(step + 1)
                .unwrap_or(u8::MAX)
                .min(AREA_MAX_COUNT)]
        );
        let effects = owner
            .think(minion, now + 29, &[spectator], || 1_400)
            .expect("early area think should succeed");
        assert!(icon_counts(&effects, RottenAreaIcon::AreaArrowUp).is_empty());
    }
}

#[test]
fn lower_left_white_cross_hits_living_players_at_one_hundred_eleven_for_crystal_energy_damage() {
    let minion = position(33_850, 31_850, 14);
    for alive in [true, false] {
        let mut owner = CanonicalFixture::default();
        let mut spectator = player(minion);
        spectator.alive = alive;
        for offset in 0_u64..111 {
            let effects = owner
                .think(minion, 1_000 + offset, &[spectator], || 1_400)
                .expect("cross think should succeed");
            if offset < 110 {
                assert_eq!(
                    icon_counts(&effects, RottenAreaIcon::WhiteCross),
                    [u8::try_from(offset + 1).expect("count fits in u8")]
                );
            } else {
                let mut expected = vec![
                    RottenAreaEffect::SetIcon {
                        actor: 1,
                        icon: RottenAreaIcon::WhiteCross,
                        count: 111,
                    },
                    RottenAreaEffect::SetIcon {
                        actor: 1,
                        icon: RottenAreaIcon::WhiteCross,
                        count: 0,
                    },
                ];
                if alive {
                    expected.push(RottenAreaEffect::EnergyDamage {
                        actor: 1,
                        amount: 1_400,
                    });
                }
                assert!(effects.ends_with(&expected));
            }
        }
        let effects = owner
            .think(minion, 1_111, &[spectator], || 1_400)
            .expect("post-hit cross think should succeed");
        assert_eq!(icon_counts(&effects, RottenAreaIcon::WhiteCross), [1]);
    }
}

#[test]
fn a_player_is_processed_once_per_second_across_minions() {
    let mut owner = CanonicalFixture::default();
    let spectator = player(position(33_850, 31_850, 14));
    let first = owner
        .think(position(33_849, 31_850, 14), 1_000, &[spectator], || 1_400)
        .expect("first minion think should succeed");
    assert!(!first.is_empty());
    let second = owner
        .think(position(33_851, 31_850, 14), 1_000, &[spectator], || 1_400)
        .expect("second minion think should succeed");
    assert!(second.is_empty());
    let next_second = owner
        .think(position(33_851, 31_850, 14), 1_001, &[spectator], || 1_400)
        .expect("next-second think should succeed");
    assert!(!next_second.is_empty());
}

#[test]
fn spectators_beyond_fifteen_tiles_or_on_another_floor_are_ignored() {
    let minion = position(33_850, 31_700, 14);
    for outside in [
        position(33_866, 31_700, 14),
        position(33_850, 31_684, 14),
        position(33_850, 31_700, 15),
    ] {
        let mut owner = CanonicalFixture::default();
        let effects = owner
            .think(minion, 1_000, &[player(outside)], || 1_400)
            .expect("out-of-range think should succeed");
        assert!(effects.is_empty());
    }
    let mut owner = CanonicalFixture::default();
    let effects = owner
        .think(
            minion,
            1_000,
            &[player(position(33_865, 31_685, 14))],
            || 1_400,
        )
        .expect("edge-of-range think should succeed");
    assert!(!effects.is_empty());
}

#[test]
fn an_out_of_range_damage_draw_refuses_without_changing_the_owner() {
    let mut owner = CanonicalFixture::default();
    let minion = position(33_850, 31_850, 14);
    let spectator = player(minion);
    for now in 1_000..=1_109 {
        owner
            .think(minion, now, &[spectator], || 1_400)
            .expect("cross setup should succeed");
    }
    let before = format!("{owner:?}");
    assert_eq!(
        owner.think(minion, 1_110, &[spectator], || 1_501),
        Err(RottenAreaRefusal::InvalidDamageDraw)
    );
    assert_eq!(format!("{owner:?}"), before);
    let effects = owner
        .think(minion, 1_110, &[spectator], || 1_300)
        .expect("valid damage draw should succeed");
    assert!(effects.ends_with(&[
        RottenAreaEffect::SetIcon {
            actor: 1,
            icon: RottenAreaIcon::WhiteCross,
            count: 111,
        },
        RottenAreaEffect::SetIcon {
            actor: 1,
            icon: RottenAreaIcon::WhiteCross,
            count: 0,
        },
        RottenAreaEffect::EnergyDamage {
            actor: 1,
            amount: 1_300,
        },
    ]));
}

#[test]
fn login_removes_the_three_icons_but_keeps_the_timers() {
    let mut owner = CanonicalFixture::default();
    let minion = position(33_850, 31_850, 14);
    let spectator = player(minion);
    owner
        .think(minion, 1_000, &[spectator], || 1_400)
        .expect("initial think should succeed");
    assert_eq!(
        owner.login(1),
        vec![
            RottenAreaEffect::RemoveIcon {
                actor: 1,
                icon: RottenAreaIcon::Hazard,
            },
            RottenAreaEffect::RemoveIcon {
                actor: 1,
                icon: RottenAreaIcon::AreaArrowUp,
            },
            RottenAreaEffect::RemoveIcon {
                actor: 1,
                icon: RottenAreaIcon::WhiteCross,
            },
        ]
    );
    let effects = owner
        .think(minion, 1_001, &[spectator], || 1_400)
        .expect("post-login think should succeed");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::WhiteCross), [1]);
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        RottenAreaEffect::SetIcon {
            icon: RottenAreaIcon::Hazard,
            ..
        } | RottenAreaEffect::RemoveIcon {
            icon: RottenAreaIcon::Hazard,
            ..
        }
    )));
}

#[test]
fn srx3_rotten_area_current_owner_icon_count_and_durable_clock_survive_handoff() {
    let minion = position(33_850, 31_700, 14);
    let spectator = player(minion);
    let current = RottenAreaCurrentFacts {
        actor: spectator.actor,
        last_hazard: 995,
        last_area: 970,
        last_cross: 0,
        area_count: Some(17),
        cross_count: None,
    };
    let effects = RottenAreaIconOwner::default()
        .think_current(minion, 1_000, &[spectator], &[current], || 1_400)
        .expect("canonical existing icon and KV should be consumed");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::AreaArrowUp), [18]);
    assert!(icon_counts(&effects, RottenAreaIcon::Hazard).is_empty());

    let after_commit = RottenAreaCurrentFacts {
        last_area: 1_000,
        area_count: Some(18),
        ..current
    };
    let effects = RottenAreaIconOwner::default()
        .think_current(minion, 1_029, &[spectator], &[after_commit], || 1_400)
        .expect("fresh callback owner after handoff must retain durable clock");
    assert!(icon_counts(&effects, RottenAreaIcon::AreaArrowUp).is_empty());
}

#[test]
fn srx3_rotten_cross_reads_canonical_zero_and_existing_one_hundred_ten() {
    let minion = position(33_850, 31_850, 14);
    let current = RottenAreaCurrentFacts {
        actor: 1,
        last_hazard: 995,
        last_cross: 999,
        cross_count: Some(110),
        ..RottenAreaCurrentFacts::default()
    };
    let effects = RottenAreaIconOwner::default()
        .think_current(minion, 1_000, &[player(minion)], &[current], || 1_400)
        .expect("canonical cross should reach source trigger");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::WhiteCross), [111, 0]);
    assert!(effects.contains(&RottenAreaEffect::EnergyDamage {
        actor: 1,
        amount: 1_400
    }));
    let reset = RottenAreaCurrentFacts {
        cross_count: Some(0),
        last_cross: 1_000,
        ..current
    };
    let effects = RottenAreaIconOwner::default()
        .think_current(minion, 1_001, &[player(minion)], &[reset], || 1_400)
        .expect("source count zero remains a present icon");
    assert_eq!(icon_counts(&effects, RottenAreaIcon::WhiteCross), [1]);
}
