#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use oteryn_protocol_oteryn::actor_spell::{SpellTargetPosition, encode_actor_vitals};
use serde_json::Value;

use super::*;
use crate::ability::RevisionSet;
use crate::spell::OperationalCastFacts;

const CASTER: &str = "actor:caster";

#[test]
fn actual_named_healing_failure_starts_cooldowns_without_resources_or_harmony() {
    use sha2::{Digest, Sha256};
    let bytes = include_bytes!(
        "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    );
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let catalog =
        crate::spell::executable_catalog::compile(bytes, &digest).expect("actual source catalog");
    for name in ["Heal Friend", "Nature's Embrace", "Restore Balance"] {
        let spell = catalog
            .entries
            .iter()
            .find(|e| e.header.name == name)
            .expect(name)
            .definition
            .clone()
            .expect("qualified source spell");
        let vocation = if name == "Restore Balance" {
            Vocation::Monk
        } else {
            Vocation::Druid
        };
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation,
                level: 400,
                magic_level: 100,
                max_mana: 1000,
                ..druid(400)
            },
            if vocation == Vocation::Monk { 3 } else { 0 },
            0,
        )
        .expect("owner state");
        state
            .apply_owner_premium_transition(true, Some(200_000_000))
            .expect("current premium");
        state.make_playable(at(0)).expect("initialized owner");
        let caster = state.caster(
            state
                .owned_harmony_multiplier(&spell)
                .expect("qualified multiplier"),
        );
        for _source_failure in ["offline", "ambiguous", "staff_hidden"] {
            let paid =
                prepare_named_player_failure_owner_cast_with_caster(&state, &spell, at(1), &caster)
                    .expect("source failed name");
            assert_eq!(
                (paid.next.health, paid.next.mana, paid.next.soul),
                (state.health, state.mana, state.soul)
            );
            assert_eq!(
                paid.next.monk_save_values(at(1)),
                state.monk_save_values(at(1))
            );
            assert_eq!((paid.anchor.paid_mana, paid.anchor.paid_soul), (0, 0));
            assert_eq!(paid.next.revision, state.revision + 1);
            let current_caster = paid.next.caster(
                paid.next
                    .owned_harmony_multiplier(&spell)
                    .expect("current multiplier"),
            );
            assert_eq!(
                prepare_named_player_failure_owner_cast_with_caster(
                    &paid.next,
                    &spell,
                    at(2),
                    &current_caster
                )
                .err(),
                Some(SpellCastDisposition::CoolingDown)
            );
            let operational = OperationalCastFacts {
                caster_position: TilePosition {
                    x: 0,
                    y: 0,
                    floor: 0,
                },
                target_position: Some(TilePosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                }),
                target: Some(super::super::target::CastTarget {
                    caster: 1,
                    creature: 2,
                    actor: "test:other-player".into(),
                    master: None,
                }),
                line_of_sight_clear: Some(true),
                direction_available: true,
                wheel_unlocked: None,
                in_protection_zone: false,
                target_tile_solid: Some(false),
                target_tile_creature: Some(true),
            };
            let mut draws = 0;
            let successful_target = prepare_ordinary_owner_cast_with_caster(
                &paid.next,
                &spell,
                &operational,
                at(2),
                &current_caster,
                None,
                None,
                &mut |minimum, _| {
                    draws += 1;
                    minimum
                },
            );
            assert_eq!(
                successful_target.err(),
                Some(SpellCastDisposition::CoolingDown)
            );
            assert_eq!(draws, 0);
            assert!(
                prepare_named_player_failure_owner_cast_with_caster(
                    &paid.next,
                    &spell,
                    at(61_001),
                    &current_caster
                )
                .is_ok()
            );
        }
        let mut substituted = caster.clone();
        substituted.magic_level += 1;
        assert!(
            prepare_named_player_failure_owner_cast_with_caster(
                &state,
                &spell,
                at(1),
                &substituted
            )
            .is_err()
        );
        let mut unavailable = state.clone();
        unavailable.mana = 0;
        let caster = unavailable.caster(
            unavailable
                .owned_harmony_multiplier(&spell)
                .expect("multiplier"),
        );
        assert_eq!(
            prepare_named_player_failure_owner_cast_with_caster(
                &unavailable,
                &spell,
                at(1),
                &caster
            )
            .err(),
            Some(SpellCastDisposition::NotEnoughMana)
        );
        assert_eq!(unavailable.mana, 0);
    }
}

#[test]
fn premium_grant_renewal_and_expiry_keep_soul_without_refilling() {
    let mut state = PlayerSpellState::new(druid(8), 0, 0).expect("state");
    state.soul = 75;
    assert!(
        state
            .apply_owner_premium_transition(true, Some(100))
            .expect("grant")
    );
    assert_eq!(
        (state.soul, state.facts.max_soul, state.revision()),
        (75, 200, 2)
    );
    assert!(
        !state
            .apply_owner_premium_transition(true, Some(150))
            .expect("renewal")
    );
    assert!(!state.expire_owner_premium(149).expect("before expiry"));
    state.soul = 180;
    assert!(state.expire_owner_premium(150).expect("expiry"));
    assert_eq!(
        (state.soul, state.facts.max_soul, state.revision()),
        (100, 100, 3)
    );
    assert!(!state.expire_owner_premium(151).expect("already expired"));
    assert!(
        state
            .apply_owner_premium_transition(true, Some(200))
            .expect("new grant")
    );
    assert_eq!(state.soul, 100);
}

// This qualifies the pure actor-owned transition, not a commercial entitlement issuer.
#[test]
fn premium_revocation_expiry_and_regrant_obey_the_same_soul_limits() {
    for initial in [0, 75, 100] {
        for revoke_explicitly in [false, true] {
            let mut state = PlayerSpellState::new(druid(8), 0, 0).expect("state");
            state.soul = initial;
            state
                .apply_owner_premium_transition(true, Some(100))
                .expect("grant");
            assert_eq!((state.soul, state.facts.max_soul), (initial, 200));
            // The actor has accumulated soul while Premium was genuinely current.
            state.soul = 180;
            let changed = if revoke_explicitly {
                state
                    .apply_owner_premium_transition(false, None)
                    .expect("revoke")
            } else {
                assert!(!state.expire_owner_premium(99).expect("before expiry"));
                state.expire_owner_premium(100).expect("expiry")
            };
            assert!(changed);
            assert_eq!((state.soul, state.facts.max_soul), (100, 100));
            let revision = state.revision();
            assert!(
                !state
                    .apply_owner_premium_transition(false, None)
                    .expect("repeat revoke")
            );
            assert_eq!(state.revision(), revision);
            state
                .apply_owner_premium_transition(true, Some(200))
                .expect("regrant");
            assert_eq!((state.soul, state.facts.max_soul), (100, 200));
            assert!(!state.expire_owner_premium(199).expect("current"));
            assert!(state.expire_owner_premium(200).expect("second expiry"));
            assert_eq!((state.soul, state.facts.max_soul), (100, 100));
        }
    }
}

#[test]
fn premium_transition_refuses_incomplete_evidence_and_revision_overflow_atomically() {
    let mut state = PlayerSpellState::new(druid(8), 0, 0).expect("state");
    let before = state.clone();
    assert!(state.apply_owner_premium_transition(true, None).is_err());
    assert_eq!(state, before);
    state.revision = u64::MAX;
    let before = state.clone();
    assert!(
        state
            .apply_owner_premium_transition(true, Some(100))
            .is_err()
    );
    assert_eq!(state, before);
}

fn druid(level: u32) -> CharacterCastFacts {
    CharacterCastFacts {
        vocation: Vocation::Druid,
        level,
        magic_level: 0,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    }
}

fn index(spell: u32) -> NonZeroU32 {
    NonZeroU32::new(spell).expect("1-based")
}

fn intent(spell: u32, target: SpellTarget) -> SpellCastIntent {
    SpellCastIntent {
        spell: index(spell),
        target,
        aim_at_target: false,
    }
}

fn at(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

/// Cast with the highest draw of each formula; `draws` records every `(minimum, maximum)`.
fn run(
    book: &SpellBook,
    state: &PlayerSpellState,
    intent: &SpellCastIntent,
    now: SemanticTimeMicros,
    draws: &mut Vec<(i64, i64)>,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    let revisions = RevisionSet::new(
        "ruleset:r1",
        "content:r1",
        "world:r1",
        "formula:r1",
        "simulation:r1",
    )
    .expect("revisions");
    let mut draw = |minimum: i64, maximum: i64| {
        draws.push((minimum, maximum));
        maximum
    };
    cast(
        book,
        state,
        intent,
        CastContext {
            caster: CASTER,
            owner_scope: "channel-owner",
            occurrence: AbilityOccurrence::new("spell-cast:1", revisions).expect("occurrence"),
            now,
            draw: &mut draw,
        },
    )
}

fn wounded(facts: CharacterCastFacts, health: u32) -> PlayerSpellState {
    PlayerSpellState {
        health,
        ..PlayerSpellState::new(facts, 0, 0).expect("state")
    }
}

#[test]
fn v1_book_holds_the_self_casts_in_canonical_key_order() {
    let book = v1_spell_book().expect("V1 book");
    let keys: Vec<&str> = (1..=3)
        .map(|spell| book.indexed(index(spell)).expect("indexed").key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "candidate:spell/cure_poison",
            "candidate:spell/intense_healing",
            "candidate:spell/light_healing",
        ]
    );
    assert!(book.indexed(index(4)).is_none());
    assert!(book.indexed(index(u32::MAX)).is_none());
}

#[test]
fn a_new_actor_starts_at_its_maxima_within_the_wire_bounds() {
    let state = PlayerSpellState::new(druid(8), 0, 0).expect("state");
    assert_eq!(state.revision(), 1);
    let vitals = state.vitals();
    assert_eq!(
        (vitals.health, vitals.mana, vitals.soul),
        (185, 90, 100),
        "a new runtime actor starts at its maxima"
    );
    assert_eq!((vitals.harmony, vitals.serene), (0, false));
    assert!(encode_actor_vitals(&vitals).is_ok());
    for facts in [
        CharacterCastFacts {
            max_health: MAX_VITAL_POOL + 1,
            ..druid(8)
        },
        CharacterCastFacts {
            max_mana: MAX_VITAL_POOL + 1,
            ..druid(8)
        },
        CharacterCastFacts {
            max_soul: MAX_SOUL + 1,
            ..druid(8)
        },
    ] {
        assert_eq!(PlayerSpellState::new(facts, 0, 0), None);
    }
}

#[test]
fn a_cast_heals_and_pays_mana_and_cooldowns_in_one_successor_state() {
    let book = v1_spell_book().expect("V1 book");
    let state = wounded(druid(8), 100);
    let mut draws = Vec::new();
    let next = run(
        &book,
        &state,
        &intent(3, SpellTarget::None),
        at(0),
        &mut draws,
    )
    .expect("exura");
    let (_, maximum) = draws[0];
    assert!(maximum > 0);
    assert_eq!(next.health, 100 + u32::try_from(maximum).expect("heal"));
    assert_eq!(next.mana, 70, "light healing costs 20 mana");
    assert_eq!(next.soul, 100);
    assert_eq!(next.revision(), 2);
    assert_eq!(
        next.cooldowns
            .spell_ready_at("candidate:spell/light_healing"),
        Some(at(1000))
    );
    assert_eq!(next.cooldowns.group_ready_at("healing"), Some(at(1000)));
    // The source state is untouched: the owner commits `next` or nothing.
    assert_eq!(state.mana, 90);
    assert_eq!(state.revision(), 1);
}

#[test]
fn a_heal_never_rises_above_the_maximum_and_a_full_caster_still_pays() {
    let book = v1_spell_book().expect("V1 book");
    let state = wounded(druid(8), 184);
    let next = run(
        &book,
        &state,
        &intent(3, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("exura");
    assert_eq!(next.health, 185);
    assert_eq!(next.mana, 70);
}

#[test]
fn cooldowns_reject_until_ready_and_a_rejection_changes_nothing() {
    let book = v1_spell_book().expect("V1 book");
    let state = wounded(druid(20), 50);
    let first = run(
        &book,
        &state,
        &intent(3, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("exura");
    // The shared healing group blocks another heal too.
    for spell in [2, 3] {
        assert_eq!(
            run(
                &book,
                &first,
                &intent(spell, SpellTarget::None),
                at(999),
                &mut Vec::new()
            ),
            Err(SpellCastDisposition::CoolingDown)
        );
    }
    let second = run(
        &book,
        &first,
        &intent(3, SpellTarget::None),
        at(1000),
        &mut Vec::new(),
    )
    .expect("ready again");
    assert_eq!(second.mana, first.mana - 20);
    assert_eq!(second.revision(), 3);
}

#[test]
fn caster_checks_map_to_their_dispositions() {
    let book = v1_spell_book().expect("V1 book");
    let none = SpellTarget::None;
    let cases = [
        // Level 8 is below exura gran's 20.
        (wounded(druid(8), 50), 2, SpellCastDisposition::LevelTooLow),
        (
            PlayerSpellState {
                mana: 19,
                ..wounded(druid(8), 50)
            },
            3,
            SpellCastDisposition::NotEnoughMana,
        ),
        (
            wounded(
                CharacterCastFacts {
                    vocation: Vocation::Knight,
                    ..druid(8)
                },
                50,
            ),
            3,
            SpellCastDisposition::NotAvailable,
        ),
    ];
    for (state, spell, expected) in cases {
        assert_eq!(
            run(&book, &state, &intent(spell, none), at(0), &mut Vec::new()),
            Err(expected)
        );
    }
}

#[test]
fn unknown_index_and_position_intent_are_rejected_and_attack_target_casts_on_self() {
    let book = v1_spell_book().expect("V1 book");
    let state = wounded(druid(8), 50);
    assert_eq!(
        run(
            &book,
            &state,
            &intent(4, SpellTarget::None),
            at(0),
            &mut Vec::new()
        ),
        Err(SpellCastDisposition::Rejected)
    );
    let position = SpellTarget::Position(SpellTargetPosition {
        x: 0,
        y: 0,
        floor: 0,
    });
    assert_eq!(
        run(&book, &state, &intent(3, position), at(0), &mut Vec::new()),
        Err(SpellCastDisposition::Rejected)
    );
    // No attack target is held, so a self heal cast "at target" heals the caster.
    let aimed = SpellCastIntent {
        aim_at_target: true,
        ..intent(3, SpellTarget::AttackTarget)
    };
    let next = run(&book, &state, &aimed, at(0), &mut Vec::new()).expect("exura");
    assert!(next.health > 50);
}

#[test]
fn condition_removal_is_satisfied_for_a_caster_without_conditions() {
    let book = v1_spell_book().expect("V1 book");
    let state = wounded(druid(10), 50);
    let next = run(
        &book,
        &state,
        &intent(1, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("exana pox");
    assert_eq!(next.health, 50);
    assert_eq!(next.mana, 60, "cure poison costs 30 mana");
    assert_eq!(
        next.cooldowns.spell_ready_at("candidate:spell/cure_poison"),
        Some(at(6000))
    );
}

#[test]
fn every_core_rejection_maps_to_the_section_3_disposition() {
    use SpellCastDisposition as D;
    let ready_at = at(1);
    let cases = [
        (
            CastRejection::GroupCooling {
                group: "healing".into(),
                ready_at,
            },
            D::CoolingDown,
        ),
        (CastRejection::SpellCooling { ready_at }, D::CoolingDown),
        (CastRejection::LevelTooLow { required: 1 }, D::LevelTooLow),
        (
            CastRejection::MagicLevelTooLow { required: 1 },
            D::MagicLevelTooLow,
        ),
        (
            CastRejection::NotEnoughMana { required: 1 },
            D::NotEnoughMana,
        ),
        (
            CastRejection::NotEnoughSoul { required: 1 },
            D::NotEnoughSoul,
        ),
        (CastRejection::NotLearned, D::NotAvailable),
        (CastRejection::VocationCannotUse, D::NotAvailable),
        (CastRejection::PremiumRequired, D::NotAvailable),
        (CastRejection::TargetRequired, D::TargetRequired),
        (CastRejection::TargetNotAllowed, D::TargetIllegal),
        (CastRejection::TimeOverflow, D::Rejected),
        (
            CastRejection::Formula(super::super::FormulaError::DivisionByZero),
            D::Rejected,
        ),
        (CastRejection::WeaponRequired, D::Rejected),
        (CastRejection::NoChainTarget, D::Rejected),
    ];
    for (rejection, expected) in cases {
        assert_eq!(disposition(&rejection), expected, "{rejection:?}");
    }
}

#[test]
fn a_book_that_does_not_load_fails_closed_and_nothing_is_skipped() {
    let good = V1_BUNDLES[0];
    let book = book_from_bundles(&V1_BUNDLES).expect("book");
    assert!(book.indexed(index(7)).is_some() && book.indexed(index(8)).is_none());
    // Unparseable spell, unparseable dependencies, and a bundle that parses but is no spell.
    assert!(book_from_bundles(&[("not json", good.1)]).is_err());
    assert!(book_from_bundles(&[(good.0, "not json")]).is_err());
    assert!(book_from_bundles(&[("{}", "{}")]).is_err());
    // One bad bundle among good ones refuses the whole book: no index shifts silently.
    assert!(book_from_bundles(&[good, ("{}", "{}"), V1_BUNDLES[1]]).is_err());
    // A duplicated spell (two bundles, one key) is refused by the book itself.
    assert!(book_from_bundles(&[good, good]).is_err());
}

fn monk(level: u32) -> CharacterCastFacts {
    CharacterCastFacts {
        vocation: Vocation::Monk,
        level,
        magic_level: 0,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    }
}

/// `exura` authored for monks with the Harmony `role`. It is the one self cast V1 commits, so it
/// shows the §8.2 PRIMARY COMMIT without a damage owner.
fn monk_book(role: &str) -> SpellBook {
    let (spell, dependencies) = V1_BUNDLES[2];
    let mut spell: Value = serde_json::from_str(spell).expect("spell");
    spell["spell"]["harmony_role"] = Value::String(role.into());
    spell["spell"]["requirements"]["vocations"] = serde_json::json!(["monk", "exalted_monk"]);
    let dependencies: Value = serde_json::from_str(dependencies).expect("dependencies");
    SpellBook::canonical(vec![
        spell_from_bundle(&spell, &dependencies).expect("monk spell"),
    ])
    .expect("book")
}

fn playable_monk(harmony: u8) -> PlayerSpellState {
    let mut state = PlayerSpellState::new(monk(20), harmony, 0).expect("monk");
    state.make_playable(at(0)).expect("initialized");
    state
}

#[test]
fn a_monk_accepts_no_cast_before_its_initialization_evaluation() {
    let book = monk_book("builder");
    let mut state = PlayerSpellState::new(monk(20), 0, 0).expect("monk");
    // Loaded but not yet evaluated: not Serene, and no command is accepted.
    assert_eq!((state.vitals().harmony, state.vitals().serene), (0, false));
    assert_eq!(
        run(
            &book,
            &state,
            &intent(1, SpellTarget::None),
            at(0),
            &mut Vec::new()
        ),
        Err(SpellCastDisposition::Rejected)
    );
    // A solo monk is Serene from the initialization evaluation, with no periodic step run.
    state.make_playable(at(0)).expect("initialized");
    assert_eq!((state.vitals().harmony, state.vitals().serene), (0, true));
    let next = run(
        &book,
        &state,
        &intent(1, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("builder cast");
    assert_eq!((next.vitals().harmony, next.revision()), (1, 2));
    // Detached (the session left the actor): commands wait for the next initialization.
    let mut detached = next.clone();
    detached.detach();
    assert_eq!(
        run(
            &book,
            &detached,
            &intent(1, SpellTarget::None),
            at(2000),
            &mut Vec::new()
        ),
        Err(SpellCastDisposition::Rejected)
    );
    detached.make_playable(at(2000)).expect("reinitialized");
    assert_eq!(
        detached.vitals().harmony,
        1,
        "Harmony is kept across a detach"
    );
    assert!(
        run(
            &book,
            &detached,
            &intent(1, SpellTarget::None),
            at(2000),
            &mut Vec::new()
        )
        .is_ok()
    );
}

#[test]
fn a_builder_adds_one_harmony_up_to_five_and_a_failed_cast_keeps_it() {
    let book = monk_book("builder");
    let state = playable_monk(4);
    let full = run(
        &book,
        &state,
        &intent(1, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("builder cast");
    assert_eq!(full.vitals().harmony, 5);
    assert_eq!(full.mana, 70, "mana is paid in the same value");
    // A rejected cast (cooling down) changes nothing, Harmony included.
    assert_eq!(
        run(
            &book,
            &full,
            &intent(1, SpellTarget::None),
            at(500),
            &mut Vec::new()
        ),
        Err(SpellCastDisposition::CoolingDown)
    );
    let capped = run(
        &book,
        &full,
        &intent(1, SpellTarget::None),
        at(1000),
        &mut Vec::new(),
    )
    .expect("builder cast");
    assert_eq!(capped.vitals().harmony, 5, "nothing is gained at 5");
    assert_eq!(state.vitals().harmony, 4, "the source state is untouched");
}

#[test]
fn a_spender_empties_harmony_at_primary_commit() {
    let book = monk_book("spender");
    let state = playable_monk(3);
    let next = run(
        &book,
        &state,
        &intent(1, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("spender cast");
    assert_eq!(next.vitals().harmony, 0);
    assert_eq!(next.revision(), 2);
    assert!(encode_actor_vitals(&next.vitals()).is_ok());
    // A monk's cast of a spell without a role (plain `exura`) leaves Harmony alone.
    let plain = run(
        &v1_spell_book().expect("V1 book"),
        &state,
        &intent(3, SpellTarget::None),
        at(0),
        &mut Vec::new(),
    )
    .expect("exura");
    assert_eq!(plain.vitals().harmony, 3);
}

#[test]
fn corrupt_durable_monk_values_fail_the_actor_closed() {
    assert_eq!(PlayerSpellState::new(monk(20), 6, 0), None);
    assert_eq!(PlayerSpellState::new(monk(20), 0, 7_000_001), None);
    assert_eq!(PlayerSpellState::new(druid(20), 1, 0), None);
    assert_eq!(PlayerSpellState::new(druid(20), 0, 1), None);
    let druid = PlayerSpellState::new(druid(20), 0, 0).expect("druid");
    assert_eq!(druid.monk_save_values(at(0)), None);
    assert_eq!((druid.vitals().harmony, druid.vitals().serene), (0, false));
}

#[test]
fn the_forced_serene_time_runs_from_initialization_and_the_periodic_evaluation_keeps_solo_serene() {
    let mut state = PlayerSpellState::new(monk(20), 2, 3_000_000).expect("monk");
    // Before the initialization the loaded remaining time is kept whole, and no tick runs.
    assert_eq!(state.monk_save_values(at(5000)), Some((2, 3_000_000)));
    assert!(state.tick(at(0)).is_err());
    assert_eq!(state.revision(), 1);
    state.make_playable(at(1000)).expect("initialized");
    assert!(state.vitals().serene);
    assert_eq!(state.monk_save_values(at(2000)), Some((2, 2_000_000)));
    // Not due yet, then due: a solo monk stays Serene, so nothing is published.
    assert_eq!(state.tick(at(1999)), Ok(false));
    assert_eq!(state.tick(at(2000)), Ok(false));
    assert_eq!(state.tick(at(9000)), Ok(false));
    assert!(state.vitals().serene);
    assert_eq!(state.revision(), 1);
    // The forced time has run out: the actor-end save stores zero for it.
    assert_eq!(state.monk_save_values(at(9000)), Some((2, 0)));
}

#[test]
fn actual_enchant_condition_changes_legacy_heal_draw_until_expiry_without_training_write() {
    let book = v1_spell_book().expect("actual starter book");
    let mut state = wounded(druid(8), 100);
    let build = crate::durability::character_build::DurableBuildState::new(
        "druid",
        (0, 123),
        [(10, 456); 7],
    )
    .expect("valid durable base progression");
    state.training = Some(super::super::mana_training::LiveManaTraining::from_owned_build(&build));
    let base = state.character_facts();
    let training = state.training.clone();
    let profile: super::super::executable_catalog::EffectProfile = serde_json::from_value(serde_json::json!({
        "identity":{"key":"candidate:spell/enchant_party/effect-attributes","revision":"spell-p2-r20"},
        "operation":"condition","duration_ms":120000,
        "condition":{"type":"attributes","lifetime":"fixed_duration","buff_spell":true,
            "attribute_modifiers":[{"attribute":"stat_magicpoints","mode":"add","value":1}]}
    })).expect("qualified source Enchant shape");
    super::super::actor_conditions::apply_effect(&mut state, &profile, 0)
        .expect("actual store application");
    let mut before_draws = Vec::new();
    let before = run(
        &book,
        &state,
        &intent(3, SpellTarget::None),
        at(119999),
        &mut before_draws,
    )
    .expect("buffed actual heal");
    let mut expired_draws = Vec::new();
    let expired = run(
        &book,
        &state,
        &intent(3, SpellTarget::None),
        at(120000),
        &mut expired_draws,
    )
    .expect("expired actual heal");
    assert_eq!(before_draws.len(), 1);
    assert_eq!(expired_draws.len(), 1);
    assert!(before_draws[0].0 > expired_draws[0].0);
    assert!(before_draws[0].1 > expired_draws[0].1);
    for successor in [before, expired] {
        assert_eq!(successor.character_facts(), base);
        assert_eq!(successor.training, training);
        assert_eq!(successor.mana, state.mana - 20);
    }
    assert_eq!(state.character_facts(), base);
    assert_eq!(state.training, training);
}

#[test]
fn resolved_health_credit_is_capped_and_preserves_unrelated_state() {
    let state = wounded(druid(8), 100);
    for (requested, expected) in [(0, 0), (7, 7), (85, 85), (u64::MAX, 85)] {
        let (next, gained) = state.after_health_gain(requested).expect("credit");
        let mut expected_state = state.clone();
        expected_state.health += expected;
        if expected != 0 {
            expected_state.revision += 1;
        }
        assert_eq!(gained, expected);
        assert_eq!(next, expected_state);
        assert_eq!((state.health, state.revision), (100, 1));
    }
    let full = PlayerSpellState::new(druid(8), 0, 0).expect("full");
    assert_eq!(full.after_health_gain(u64::MAX), Some((full.clone(), 0)));
}

#[test]
fn resolved_mana_credit_is_capped_and_preserves_unrelated_state() {
    let state = PlayerSpellState {
        mana: 70,
        ..PlayerSpellState::new(druid(8), 0, 0).expect("state")
    };
    for (requested, expected) in [(0, 0), (7, 7), (20, 20), (u64::MAX, 20)] {
        let (next, gained) = state.after_mana_gain(requested).expect("credit");
        let mut expected_state = state.clone();
        expected_state.mana += expected;
        if expected != 0 {
            expected_state.revision += 1;
        }
        assert_eq!(gained, expected);
        assert_eq!(next, expected_state);
        assert_eq!((state.mana, state.revision), (70, 1));
    }
    let full = PlayerSpellState::new(druid(8), 0, 0).expect("full");
    assert_eq!(full.after_mana_gain(u64::MAX), Some((full.clone(), 0)));
}

#[test]
fn positive_vitals_credit_refuses_revision_exhaustion_without_mutation() {
    let exhausted = PlayerSpellState {
        health: 100,
        mana: 70,
        revision: u64::MAX,
        ..PlayerSpellState::new(druid(8), 0, 0).expect("state")
    };
    let before = exhausted.clone();
    assert!(exhausted.after_health_gain(1).is_none());
    assert!(exhausted.after_mana_gain(1).is_none());
    assert_eq!(exhausted, before);
    assert_eq!(exhausted.after_health_gain(0), Some((exhausted.clone(), 0)));
    assert_eq!(exhausted.after_mana_gain(0), Some((exhausted.clone(), 0)));
}

#[test]
fn vitals_credit_refuses_a_pool_above_its_current_maximum() {
    let invalid_health = PlayerSpellState {
        health: 186,
        ..PlayerSpellState::new(druid(8), 0, 0).expect("state")
    };
    assert!(invalid_health.after_health_gain(1).is_none());
    assert_eq!((invalid_health.health, invalid_health.revision), (186, 1));
    let invalid_mana = PlayerSpellState {
        mana: 91,
        ..PlayerSpellState::new(druid(8), 0, 0).expect("state")
    };
    assert!(invalid_mana.after_mana_gain(1).is_none());
    assert_eq!((invalid_mana.mana, invalid_mana.revision), (91, 1));
}
