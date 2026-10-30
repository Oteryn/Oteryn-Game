//! C11 parity: monk Harmony and Serene against Crystal `201189f8` (`combat.cpp`
//! `applyImproveMonkAttackSpender`, `Player::setHarmony`, `Spell::playerSpellCheck`/`postCastSpell`,
//! `Game::checkImbuementsAndSereneStatus`, `hasPartyMembersNearby`, `isPlayerNoBoxed`, and the
//! `focus_*`/builder spell scripts). The oracles below are test-only restatements of that source,
//! not production code. Where news 8944 (2026-09-01, newer than Crystal and TibiaWiki) changed a
//! value, Oteryn follows it (`OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §A.2) and the test
//! pins the exact difference.

#![allow(clippy::expect_used, clippy::panic)]

use oteryn_simulation_determinism::SemanticTimeMicros;

use super::Vocation;
use super::chain::{ChainCreature, TilePosition};
use super::harmony::{
    HarmonyMultiplier, MAX_HARMONY, MULTIPLIER_DENOMINATOR, MonkState, SereneWorld,
};
use super::party::SoloParty;

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

struct World {
    monk: ChainCreature,
    party: Option<Vec<ChainCreature>>,
    adjacent: u32,
}

impl SereneWorld for World {
    fn monk(&self) -> &ChainCreature {
        &self.monk
    }

    fn party(&self) -> Option<&[ChainCreature]> {
        self.party.as_deref()
    }

    fn adjacent_creatures(&self) -> u32 {
        self.adjacent
    }
}

/// The monk at (100, 100, 7); with `member`, a party of the monk and one member at that offset.
fn world(member: Option<(i32, i32, i16)>, adjacent: u32) -> World {
    let monk = creature(MONK, 100, 100, 7);
    World {
        party: member
            .map(|(dx, dy, floor)| vec![monk.clone(), creature(2, 100 + dx, 100 + dy, floor)]),
        monk,
        adjacent,
    }
}

fn ready(harmony: u8, world: &dyn SereneWorld) -> MonkState {
    let mut state = MonkState::load(Vocation::Monk, harmony, 0)
        .expect("valid")
        .expect("monk");
    state.initialize(at(0), world).expect("initialized");
    state
}

/// Crystal `applyImproveMonkAttackSpender` without Virtue of Harmony and Ascetic: base 7%, doubled
/// per extra point, an f32 multiplier and `static_cast<int32_t>` of the f32 product.
fn crystal_spender(value: i32, harmony: u8) -> i32 {
    if harmony == 0 || harmony > 5 {
        return value;
    }
    let total = 7_i32 * (1 << (harmony - 1));
    let multiplier = 1.0_f32 + (total as f32 / 100.0_f32);
    (value as f32 * multiplier) as i32
}

/// Crystal `checkImbuementsAndSereneStatus` without a forced state: Serene when there is no party
/// member within 3 tiles on the floor (`hasPartyMembersNearby`) AND fewer than 6 adjacent
/// monsters (`isPlayerNoBoxed`), party or not.
fn crystal_serene(member: Option<(i32, i32, i16)>, adjacent: u32) -> bool {
    let nearby = member.is_some_and(|(dx, dy, floor)| {
        floor == 7 && dx.abs() <= 3 && dy.abs() <= 3 && (dx, dy) != (0, 0)
    });
    !nearby && adjacent < 6
}

#[test]
fn c11_spender_bonus_without_virtue_equals_crystal_at_the_level_free_base() {
    // 8944 adds 0.005% per level to the 7% base; at level 0 the term is 0 and only the shared
    // arithmetic is left: 7, 14, 28, 56, 112% and truncation toward zero.
    let mut f32_differences = Vec::new();
    for harmony in 0..=MAX_HARMONY {
        let multiplier = ready(harmony, &SoloParty(creature(MONK, 100, 100, 7)))
            .spender_multiplier(0, false)
            .expect("ready");
        for value in -60_000..=60_000 {
            let oteryn = multiplier.apply(i64::from(value));
            let crystal = i64::from(crystal_spender(value, harmony));
            if oteryn != crystal {
                f32_differences.push((harmony, value, oteryn, crystal));
            }
        }
    }
    // Crystal multiplies by the f32 1.56f / 2.12f (just below 1.56 / 2.12), so some exact
    // products truncate one lower; Oteryn keeps the exact fraction. Every other roll matches.
    let mut stats = std::collections::BTreeMap::new();
    for (harmony, value, oteryn, crystal) in &f32_differences {
        assert_eq!(
            oteryn - crystal,
            i64::from(value.signum()),
            "{harmony} {value}"
        );
        *stats.entry(*harmony).or_insert(0) += 1;
    }
    assert_eq!(
        stats.into_iter().collect::<Vec<_>>(),
        [(4, 1_264), (5, 4_286)]
    );
    // Above level 0 the 8944 level term raises the bonus: level 200 is 8% (Crystal 7%).
    let level_200 = ready(1, &SoloParty(creature(MONK, 100, 100, 7)))
        .spender_multiplier(200, false)
        .expect("ready");
    assert_eq!(
        (level_200.apply(1_000), crystal_spender(1_000, 1)),
        (1_080, 1_070)
    );
}

#[test]
fn c11_virtue_of_harmony_scales_the_base_where_crystal_adds_points() {
    // Crystal: +3 points, +6 Serene. 8944: base x1.5, x2.0 Serene (§A.2 parameters). In tenths
    // of a percent at level 0 and one charge:
    for (serene, crystal, oteryn) in [(false, 100, 105), (true, 130, 140)] {
        let multiplier = HarmonyMultiplier::new(0, 1, true, serene).expect("bounded");
        let tenths =
            (multiplier.numerator() - MULTIPLIER_DENOMINATOR) * 1_000 / MULTIPLIER_DENOMINATOR;
        assert_eq!((tenths, crystal), (oteryn, crystal), "serene {serene}");
    }
    // Crystal also clamps Harmony to at least 1 under the virtue (`setHarmony`), the refund of
    // TibiaWiki's Virtue of Harmony; the cast owner applies that refund (§A.2 step 3c).
}

#[test]
fn c11_harmony_transitions_equal_crystal_without_virtue_of_harmony() {
    let solo = SoloParty(creature(MONK, 100, 100, 7));
    // Builders: `addHarmony(1)`, clamped to 5.
    let mut state = ready(0, &solo);
    for expected in [1, 2, 3, 4, 5, 5] {
        state.commit_builder().expect("ready");
        assert_eq!(state.harmony(), expected);
    }
    // Spender: `postCastSpell` `setHarmony(0)` (clamp minimum 0 without Virtue of Harmony).
    assert_eq!(state.commit_spender(), Ok(5));
    assert_eq!(state.harmony(), 0);
    // Focus Harmony / Focus Serenity: `setHarmony(5)`.
    for start in 0..=MAX_HARMONY {
        let mut state = ready(start, &solo);
        assert_eq!(state.commit_fill(), Ok(MAX_HARMONY - start));
        assert_eq!(state.harmony(), 5);
    }
    // Recorded difference: Crystal `playerSpellCheck` refuses a harmony spell at 0
    // (RETURNVALUE_NOTENOUGHHARMONY); Canary and Oteryn cast it with no bonus.
    let mut empty = ready(0, &solo);
    assert_eq!(empty.commit_spender(), Ok(0));
    assert_eq!(empty.harmony(), 0);
}

#[test]
fn c11_serene_equals_crystal_outside_the_recorded_8944_and_global_differences() {
    let mut cases = 0_u32;
    let (mut solo_boxed, mut crowd_six_seven, mut wider_party_view) = (0, 0, 0);
    let members =
        std::iter::once(None).chain((-10..=10).flat_map(|dx| {
            (-10..=10).flat_map(move |dy| [6, 7].map(|floor| Some((dx, dy, floor))))
        }));
    for member in members.filter(|m| !matches!(m, Some((0, 0, 7)))) {
        for adjacent in 0..=9 {
            let world = world(member, adjacent);
            let oteryn = ready(0, &world).serene();
            let crystal = crystal_serene(member, adjacent);
            cases += 1;
            if oteryn == crystal {
                continue;
            }
            let visible = member.is_some_and(|(dx, dy, floor)| {
                floor == 7 && (-8..=9).contains(&dx) && (-6..=7).contains(&dy)
            });
            let crystal_nearby =
                member.is_some_and(|(dx, dy, floor)| floor == 7 && dx.abs() <= 3 && dy.abs() <= 3);
            if !visible && adjacent >= 6 {
                // Crystal drops Serene for 6+ monsters even without a visible party member;
                // TibiaWiki: "monks who are not in a party will always be serene".
                assert!(oteryn && !crystal);
                solo_boxed += 1;
            } else if visible && (6..=7).contains(&adjacent) {
                // 8944: 8 or more adjacent creatures end Serene (Crystal and Canary: 6).
                assert!(oteryn && !crystal);
                crowd_six_seven += 1;
            } else if visible && !crystal_nearby {
                // Crystal looks 3 tiles around the monk; Oteryn uses the screen (§A.2 step 6).
                assert!(!oteryn && adjacent >= 8);
                wider_party_view += 1;
            } else if crystal_nearby && adjacent < 6 {
                // Crystal: a nearby member alone ends Serene; Oteryn needs the crowd too.
                assert!(oteryn && !crystal);
                wider_party_view += 1;
            } else {
                panic!(
                    "unrecorded difference: {member:?} adjacent {adjacent}: {oteryn} vs {crystal}"
                );
            }
        }
    }
    assert_eq!(cases, 8_820);
    assert!(solo_boxed > 0 && crowd_six_seven > 0 && wider_party_view > 0);
}

#[test]
fn c11_focus_serenity_forces_serene_for_crystal_7000_ms() {
    // `focus_serenity.lua` `setSereneCooldown(7000)`; the tick keeps Serene while the cooldown
    // is above 0 and applies the rule once it reaches 0.
    let crowded = world(Some((1, 0, 7)), 8);
    let mut state = ready(0, &crowded);
    assert!(!state.serene());
    state.commit_focus_serenity(at(500)).expect("forced");
    for now in [1_500, 6_500, 7_499] {
        state.tick(at(now), &crowded).expect("tick");
        assert!(state.serene(), "{now}");
    }
    state.tick(at(7_500), &crowded).expect("tick");
    assert!(!state.serene());
}
