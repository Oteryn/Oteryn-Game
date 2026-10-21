//! CONDITIONS-0 §3.5 (MAGIC-COND-1): the gate families refuse a spell cast first, before the
//! protection zone, cost and cooldown, with `REJECTED` on the wire.
#![allow(clippy::expect_used)]

use super::*;
use crate::spell::OperationalCastFacts;
use crate::spell::chain::TilePosition;
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionGate, ConditionSourceKind, ConditionValues,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

fn at(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

fn book() -> SpellBook {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|root| root.join("workspace-boundaries.toml").is_file())
        .expect("workspace root")
        .join("content/spells.manifest.json");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&manifest)
        .expect("canonical manifest");
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)
            .expect("catalog");
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )
    .expect("selection");
    catalog
        .into_spell_book_with_selections(&selections)
        .expect("native book")
}

fn druid() -> PlayerSpellState {
    let facts = CharacterCastFacts {
        vocation: Vocation::Druid,
        level: 100,
        magic_level: 50,
        max_health: 1000,
        max_mana: 5000,
        max_soul: 100,
    };
    let mut state = PlayerSpellState::new(facts, 0, 0).expect("state");
    state
        .apply_owner_premium_transition(true, Some(u64::MAX / 2))
        .expect("premium");
    state.make_playable(at(0)).expect("playable");
    state
}

/// `state` with each named gate for 10 s from time 0.
fn gated(mut state: PlayerSpellState, gates: &[&str]) -> PlayerSpellState {
    let root = GameplayDecisionRoot::from_bytes([5; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([6; 16]),
    };
    for gate in gates {
        let values = ConditionValues::gate(gate, 10_000).expect("gate family");
        let definition = ConditionDefinition::new("cond.gate", 1, values).expect("definition");
        state
            .conditions
            .apply(
                &definition,
                Some("creature:test".to_owned()),
                ConditionSourceKind::Creature,
                &[],
                &facts,
            )
            .expect("applied");
    }
    state
}

fn operational(in_protection_zone: bool) -> OperationalCastFacts {
    OperationalCastFacts {
        caster_position: TilePosition {
            x: 2,
            y: 0,
            floor: 7,
        },
        target_position: None,
        target: None,
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: None,
        in_protection_zone,
        target_tile_solid: None,
        target_tile_creature: None,
    }
}

fn ordinary(
    book: &SpellBook,
    state: &PlayerSpellState,
    name: &str,
    operational: &OperationalCastFacts,
) -> Result<PaidOrdinaryCast, SpellCastDisposition> {
    let spell = book.spells.iter().find(|s| s.name == name).expect(name);
    let caster = state.caster(state.owned_harmony_multiplier(spell).expect(name));
    prepare_ordinary_owner_cast_with_caster(
        book,
        state,
        spell,
        operational,
        at(1),
        &caster,
        None,
        None,
        &mut |_, maximum| maximum,
    )
}

#[test]
fn a_gated_cast_in_a_protection_zone_is_refused_by_the_gate_not_the_zone() {
    let book = book();
    let wave = book
        .spells
        .iter()
        .find(|s| s.name == "Terra Wave")
        .expect("Terra Wave");
    assert!(wave.aggressive);
    let pz = operational(true);
    for (gate, reason) in [
        ("muted", ConditionGate::Muted),
        ("pacified", ConditionGate::Pacified),
        ("powerless", ConditionGate::Powerless),
    ] {
        let state = gated(druid(), &[gate]);
        // Both refuse; the gate answers first and alone.
        assert_eq!(condition_gate(&state, wave, at(1)), Err(reason));
        assert_eq!(
            crate::spell::check_operational_rules(wave, &pz, false),
            Err(crate::spell::CastRejection::ProtectionZoneRejected)
        );
        assert_eq!(
            ordinary(&book, &state, "Terra Wave", &pz).map(|_| ()),
            Err(SpellCastDisposition::Rejected)
        );
    }
    // A non-aggressive spell in the zone: only `muted` refuses it.
    assert!(
        ordinary(
            &book,
            &gated(druid(), &["pacified", "powerless"]),
            "Light Healing",
            &pz
        )
        .is_ok()
    );
    assert_eq!(
        ordinary(&book, &gated(druid(), &["muted"]), "Light Healing", &pz).map(|_| ()),
        Err(SpellCastDisposition::Rejected)
    );
}

#[test]
fn the_gate_answers_before_cost_and_spends_nothing() {
    let book = book();
    let open = operational(false);
    let broke = PlayerSpellState { mana: 0, ..druid() };
    assert_eq!(
        ordinary(&book, &broke, "Terra Wave", &open).map(|_| ()),
        Err(SpellCastDisposition::NotEnoughMana)
    );
    assert_eq!(
        ordinary(&book, &gated(broke, &["pacified"]), "Terra Wave", &open).map(|_| ()),
        Err(SpellCastDisposition::Rejected)
    );
    // Off the gate's duration the same cast is admitted and paid.
    let state = gated(druid(), &["muted", "pacified", "powerless"]);
    let spell = book
        .spells
        .iter()
        .find(|s| s.name == "Terra Wave")
        .expect("Terra Wave");
    assert_eq!(condition_gate(&state, spell, at(10_000)), Ok(()));
    assert!(ordinary(&book, &druid(), "Terra Wave", &open).is_ok());
}

#[test]
fn gates_refuse_in_their_order_and_runes_take_the_rune_gate() {
    let book = book();
    let wave = book
        .spells
        .iter()
        .find(|s| s.name == "Terra Wave")
        .expect("Terra Wave")
        .clone();
    let all = gated(druid(), &["muted", "pacified", "powerless"]);
    assert_eq!(
        condition_gate(&all, &wave, at(1)),
        Err(ConditionGate::Muted)
    );
    let two = gated(druid(), &["pacified", "powerless"]);
    assert_eq!(
        condition_gate(&two, &wave, at(1)),
        Err(ConditionGate::Pacified)
    );

    let rune = SpellDefinition {
        carrier: crate::spell::Carrier::Rune {
            item: 1,
            charges: 1,
            magic_level: 0,
        },
        ..wave.clone()
    };
    let muted = gated(druid(), &["muted"]);
    assert_eq!(condition_gate(&muted, &rune, at(1)), Ok(()));
    assert_eq!(
        condition_gate(&two, &rune, at(1)),
        Err(ConditionGate::Pacified)
    );
    let powerless = gated(druid(), &["powerless"]);
    assert_eq!(
        condition_gate(&powerless, &rune, at(1)),
        Err(ConditionGate::Powerless)
    );
    let healing_rune = SpellDefinition {
        aggressive: false,
        ..rune
    };
    assert_eq!(condition_gate(&all, &healing_rune, at(1)), Ok(()));
}
