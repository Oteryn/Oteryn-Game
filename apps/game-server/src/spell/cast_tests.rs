#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use oteryn_protocol_oteryn::actor_spell::{SpellTargetPosition, encode_actor_vitals};

use super::*;
use crate::ability::RevisionSet;

const CASTER: &str = "actor:caster";

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
        ..PlayerSpellState::new(facts).expect("state")
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
    let state = PlayerSpellState::new(druid(8)).expect("state");
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
        assert_eq!(PlayerSpellState::new(facts), None);
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
