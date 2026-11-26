//! CONDITIONS-0 §3.5 (MAGIC-COND-1): the gate families refuse a spell cast first, before the
//! protection zone, cost and cooldown, with `REJECTED` on the wire.
#![allow(clippy::expect_used)]

use super::*;
use crate::spell::chain::TilePosition;
use crate::spell::{OperationalCastFacts, SpellDefinition};
use oteryn_gameplay_domain::domain::condition::ConditionGate;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_gameplay_domain::domain::condition::StatusKind;
use oteryn_gameplay_foundation::foundation::condition::ApplicationFacts;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
use oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind;
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;
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
fn gated(state: PlayerSpellState, gates: &[&str]) -> PlayerSpellState {
    let values = gates
        .iter()
        .map(|gate| ConditionValues::gate(gate, 10_000).expect("gate family"));
    conditioned(state, values)
}

fn conditioned(
    mut state: PlayerSpellState,
    values: impl IntoIterator<Item = ConditionValues>,
) -> PlayerSpellState {
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
    for values in values {
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
fn c04_protection_zone_returns_rejected_disposition() {
    use crate::spell::native_combat::{Direction, ElementalStance, NativeCombatFacts};

    let book = book();
    let spell = book.spoken("exevo vis lux").expect("energy beam").spell;
    let state = druid();
    let caster = state.caster(state.owned_harmony_multiplier(spell).expect("harmony"));
    let pz = operational(true);
    let facts = NativeCombatFacts {
        now_ms: 1,
        stage: 0,
        inputs: crate::spell::formula::FormulaInputs {
            level: caster.level,
            magic_level: caster.magic_level,
            base_power: spell.base_power,
            attack_skill: caster.attack_skill,
            attack_value: caster.attack_value,
            attack_factor: caster.attack_factor,
            shielding_skill: caster.shielding_skill,
            shield_defense: caster.shield_defense,
        },
        maximum_mana: caster.max_mana,
        caster: 1,
        caster_position: pz.caster_position,
        area_origin: pz.caster_position,
        area_sight_origin: pz.caster_position,
        direction: Direction::North,
        explicit_target: None,
        attacked_target: None,
        targets: Vec::new(),
        clear_sight: Default::default(),
        floor_change_tiles: Default::default(),
        elemental_stance: ElementalStance::None,
        incoming_hit: None,
    };
    assert_eq!(
        prepare_native_owner_cast_with_caster(
            &state,
            spell,
            &pz,
            crate::spell::native::Facts::Combat(&facts),
            at(1),
            &mut |_, maximum| maximum,
            &caster,
        )
        .map(|_| ()),
        Err(SpellCastDisposition::Rejected)
    );
}

#[test]
fn c04_light_healing_arms_the_shared_healing_cooldown() {
    use crate::ability::{AbilityOccurrence, RevisionSet};

    let book = book();
    let state = druid();
    let open = operational(false);
    let cast = |state: &PlayerSpellState, spell: &SpellDefinition, now| {
        let caster = state.caster(state.owned_harmony_multiplier(spell).expect("harmony"));
        let revisions = RevisionSet::new(
            "ruleset:r1",
            "content:r1",
            "world:r1",
            "formula:r1",
            "simulation:r1",
        )
        .expect("revisions");
        prepare_source_self_cast_with_caster(
            state,
            spell,
            &book,
            &open,
            &caster,
            CastContext {
                caster: "actor:c04",
                owner_scope: "channel-owner",
                occurrence: AbilityOccurrence::new("spell-cast:c04", revisions)
                    .expect("occurrence"),
                now,
                draw: &mut |_, maximum| maximum,
            },
        )
    };
    let light = book.spoken("exura").expect("light healing").spell;
    let healed = cast(&state, light, at(1)).expect("light healing").next;
    let intense = book.spoken("exura gran").expect("intense healing").spell;
    assert_eq!(healed.cooldowns.spell_ready_at(&intense.key), None);
    assert_eq!(
        cast(&healed, intense, at(2)).map(|_| ()),
        Err(SpellCastDisposition::CoolingDown)
    );
    let ready_at = healed
        .cooldowns
        .group_ready_at("healing")
        .expect("healing timer");
    assert!(cast(&healed, intense, ready_at).is_ok());
}

#[test]
fn a_protection_zone_refuses_an_aggressive_cast_before_its_cooldown_and_mana() {
    // Canary Spell::playerSpellCheck: the zone answers before the cooldowns, level and mana.
    let book = book();
    let wave = book
        .spells
        .iter()
        .find(|s| s.name == "Terra Wave")
        .expect("Terra Wave");
    let cooling = ordinary(&book, &druid(), "Terra Wave", &operational(false))
        .expect("first cast")
        .next;
    let resolve = |state: &PlayerSpellState, spell: &SpellDefinition, pz: bool| {
        let caster = state.caster(state.owned_harmony_multiplier(spell).expect("harmony"));
        crate::spell::resolve_cast_with_operational_facts(
            spell,
            &caster,
            &state.cooldowns,
            at(1),
            &operational(pz),
            &mut |_, maximum| maximum,
        )
        .map(|_| ())
    };
    assert!(matches!(
        resolve(&cooling, wave, false),
        Err(crate::spell::CastRejection::SpellCooling { .. }
            | crate::spell::CastRejection::GroupCooling { .. })
    ));
    assert_eq!(
        resolve(&cooling, wave, true),
        Err(crate::spell::CastRejection::ProtectionZoneRejected)
    );
    let broke = PlayerSpellState { mana: 0, ..druid() };
    assert!(matches!(
        resolve(&broke, wave, false),
        Err(crate::spell::CastRejection::NotEnoughMana { .. })
    ));
    assert_eq!(
        resolve(&broke, wave, true),
        Err(crate::spell::CastRejection::ProtectionZoneRejected)
    );
    // A non-aggressive spell in the zone still answers with its cooldown.
    let heal = book
        .spells
        .iter()
        .find(|s| s.name == "Light Healing")
        .expect("Light Healing");
    let healed = ordinary(&book, &druid(), "Light Healing", &operational(true))
        .expect("first heal")
        .next;
    assert!(matches!(
        resolve(&healed, heal, true),
        Err(crate::spell::CastRejection::SpellCooling { .. }
            | crate::spell::CastRejection::GroupCooling { .. })
    ));
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
fn a_feared_player_casts_no_spell_and_uses_no_rune_and_spends_nothing() {
    let book = book();
    let open = operational(false);
    let feared = conditioned(
        druid(),
        [ConditionValues::TimedStatus {
            kind: StatusKind::Feared,
            duration_ms: 10_000,
        }],
    );
    // Canary Spell::playerSpellCheck refuses a feared player first, aggressive or not.
    for name in ["Terra Wave", "Light Healing"] {
        assert_eq!(
            ordinary(&book, &feared, name, &open).map(|_| ()),
            Err(SpellCastDisposition::Rejected)
        );
        assert!(ordinary(&book, &druid(), name, &open).is_ok());
    }
    let broke = PlayerSpellState {
        mana: 0,
        ..feared.clone()
    };
    assert_eq!(
        ordinary(&book, &broke, "Terra Wave", &open).map(|_| ()),
        Err(SpellCastDisposition::Rejected)
    );
    let wave = book
        .spells
        .iter()
        .find(|s| s.name == "Terra Wave")
        .expect("Terra Wave");
    for aggressive in [true, false] {
        let rune = SpellDefinition {
            carrier: crate::spell::Carrier::Rune {
                item: 1,
                charges: 1,
                magic_level: 0,
            },
            aggressive,
            ..wave.clone()
        };
        assert_eq!(
            admit_condition_gates(&feared, &rune, at(1)),
            Err(SpellCastDisposition::Rejected)
        );
        assert_eq!(admit_condition_gates(&feared, &rune, at(10_000)), Ok(()));
    }
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
