//! SPELL-BOOK-ACTIVATE-1: the whole imported book is castable-or-refused.
//!
//! The sweep is DB-free: it drives the pure engine `prepare_*` steps over the real compiled
//! `content/spells.manifest.json` book. The native `prepare_from_owners` step needs Postgres and a
//! Platform-issued room, so native keys are covered only by the key-gate classification.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use crate::foundation::{CharacterId, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates, commit_owner_batch, stage_player_batch,
};
use crate::spell::cast::{
    CharacterCastFacts, PlayerSpellState, prepare_conjure_owner_cast_with_caster,
    prepare_ordinary_owner_cast_with_caster,
};
use crate::spell::chain::{ChainCreature, ChainStart, ChainWorld, TilePosition};
use crate::spell::combat_batch::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::spell::party::SoloParty;
use crate::spell::{
    CasterState, Execution, OperationalCastFacts, ResolvedEffect, SpellBook, SpellDefinition,
    Vocation,
};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::collections::BTreeMap;
use std::path::Path;

const VOCATIONS: [Vocation; 10] = [
    Vocation::Druid,
    Vocation::ElderDruid,
    Vocation::Sorcerer,
    Vocation::MasterSorcerer,
    Vocation::Knight,
    Vocation::EliteKnight,
    Vocation::Paladin,
    Vocation::RoyalPaladin,
    Vocation::Monk,
    Vocation::ExaltedMonk,
];

fn book() -> SpellBook {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/spells.manifest.json");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&manifest)
        .expect("canonical manifest decodes");
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
        .expect("book")
}

fn at(x: i32) -> TilePosition {
    TilePosition { x, y: 10, floor: 7 }
}

/// The caster at x=10 and one fixture creature at x=11 on the caster's floor.
struct Arena {
    caster: ChainCreature,
    creatures: Vec<ChainCreature>,
}
fn arena() -> Arena {
    Arena {
        caster: ChainCreature {
            id: 1,
            actor: "actor:caster".into(),
            position: at(10),
        },
        creatures: vec![ChainCreature {
            id: 2,
            actor: "actor:2".into(),
            position: at(11),
        }],
    }
}
impl ChainWorld for Arena {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }
    fn creatures(&self) -> &[ChainCreature] {
        &self.creatures
    }
    fn may_hit(&self, _: &ChainCreature) -> bool {
        true
    }
    fn sight_clear(&self, _: TilePosition, _: TilePosition) -> bool {
        true
    }
    fn path(&self, _: TilePosition, _: TilePosition) -> Option<Vec<TilePosition>> {
        Some(Vec::new())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Form {
    None,
    AttackTarget,
    Position,
}
const FORMS: [Form; 3] = [Form::None, Form::AttackTarget, Form::Position];

/// `AttackTarget` carries the held attack target (SPELL-TARGET-1): the arena creature, visible and
/// in range. A `Position` is a tile on the caster's floor and names no creature.
fn facts(form: Form) -> OperationalCastFacts {
    OperationalCastFacts {
        caster_position: at(10),
        target_position: (form != Form::None).then(|| at(11)),
        target: (form == Form::AttackTarget).then(|| crate::spell::target::CastTarget {
            caster: 1,
            creature: 2,
            actor: "actor:2".into(),
            master: None,
        }),
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: Some(true),
        in_protection_zone: false,
        target_tile_solid: Some(false),
        target_tile_creature: Some(false),
    }
}

fn owner(vocation: Vocation) -> (PlayerSpellState, CasterState) {
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation,
            level: 1000,
            magic_level: 300,
            max_health: 5000,
            max_mana: 50_000,
            max_soul: 200,
        },
        0,
        0,
    )
    .expect("state");
    state
        .apply_owner_premium_transition(true, Some(u64::MAX / 2))
        .expect("premium");
    state
        .make_playable(SemanticTimeMicros::from_micros(0))
        .expect("playable");
    let v = state.vitals();
    let caster = CasterState {
        harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::ONE,
        vocation,
        level: 1000,
        magic_level: 300,
        premium: true,
        mana: v.mana,
        max_mana: 50_000,
        soul: v.soul,
        learned: std::collections::BTreeSet::new(),
        attack_skill: 100,
        attack_value: 50,
        attack_factor: 1.0,
        shielding_skill: 100,
        melee_weapon: true,
        shield_defense: Some(20),
    };
    (state, caster)
}

/// What one spell, one vocation and one target form did.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Cast { wrote: bool },
    Refused(SpellCastDisposition),
}

fn ordinary(book: &SpellBook, spell: &SpellDefinition, vocation: Vocation, form: Form) -> Outcome {
    let (state, mut caster) = owner(vocation);
    match state.owned_harmony_multiplier(spell) {
        Ok(multiplier) => caster.harmony_multiplier = multiplier,
        Err(refusal) => return Outcome::Refused(refusal),
    }
    let world = arena();
    let party = SoloParty(world.caster.clone());
    let paid = prepare_ordinary_owner_cast_with_caster(
        book,
        &state,
        spell,
        &facts(form),
        SemanticTimeMicros::from_micros(1_000),
        &caster,
        Some(&party),
        Some((
            &world,
            ChainStart {
                target: None,
                attacked: Some(2),
            },
        )),
        &mut |_, maximum| maximum,
    );
    match paid {
        Ok(paid) => Outcome::Cast {
            wrote: paid.anchor.next_revision > paid.anchor.expected_revision
                && (!paid.resolution.effects.is_empty()
                    || !paid.resolution.chain.is_empty()
                    || !paid.resolution.party.is_empty()
                    || paid.anchor.paid_mana > 0),
        },
        Err(refusal) => Outcome::Refused(refusal),
    }
}

fn conjure(spell: &SpellDefinition, vocation: Vocation, form: Form) -> Outcome {
    let (state, mut caster) = owner(vocation);
    match state.owned_harmony_multiplier(spell) {
        Ok(multiplier) => caster.harmony_multiplier = multiplier,
        Err(refusal) => return Outcome::Refused(refusal),
    }
    match prepare_conjure_owner_cast_with_caster(
        &state,
        spell,
        &facts(form),
        SemanticTimeMicros::from_micros(1_000),
        &caster,
    ) {
        Ok(paid) => Outcome::Cast {
            wrote: paid.anchor.next_revision > paid.anchor.expected_revision
                && paid.anchor.paid_mana > 0,
        },
        Err(refusal) => Outcome::Refused(refusal),
    }
}

/// The dispatch route of a native key, mirroring the key gates of `cast_spell_completion`.
fn native_route(key: &str) -> &'static str {
    match key {
        "stance_toggle" => "stance_cast",
        "familiar_summon" => "familiar_cast",
        "wheel_combat"
        | "avatar_state"
        | "monster_ai_override"
        | "mass_spirit_mend"
        | "mana_shield_capacity" => "native_combat",
        "monk_focus" => "focus",
        _ => "other_native",
    }
}

fn family(spell: &SpellDefinition) -> String {
    match &spell.execution {
        Execution::Effects(_) => "Effects".into(),
        Execution::AbilityVariants(_) => "AbilityVariants".into(),
        Execution::Conjure { .. } => "Conjure".into(),
        Execution::PartyBuff(_) => "PartyBuff".into(),
        Execution::ActorFocus { .. } => "ActorFocus".into(),
        Execution::NativeProfile(p) => format!(
            "Native/{}",
            native_route(
                p.spell()["execution"]["native_behavior"]["key"]
                    .as_str()
                    .unwrap_or("?")
            )
        ),
    }
}

/// The best outcome over every vocation: a cast, else the first refusal other than
/// `NotAvailable`, else `NotAvailable`.
fn best(book: &SpellBook, spell: &SpellDefinition, form: Form) -> String {
    let run = |vocation| match &spell.execution {
        Execution::Effects(_) | Execution::AbilityVariants(_) | Execution::PartyBuff(_) => {
            Some(ordinary(book, spell, vocation, form))
        }
        Execution::Conjure { .. } => Some(conjure(spell, vocation, form)),
        _ => None,
    };
    let Some(first) = run(VOCATIONS[0]) else {
        return "Classified".into();
    };
    let all: Vec<Outcome> = std::iter::once(first)
        .chain(VOCATIONS[1..].iter().filter_map(|v| run(*v)))
        .collect();
    if let Some(Outcome::Cast { wrote }) = all.iter().find(|o| matches!(o, Outcome::Cast { .. })) {
        assert!(
            *wrote,
            "{} cast without an owner effect or vitals change",
            spell.key
        );
        return "Cast".into();
    }
    let refusal = all
        .iter()
        .find_map(|o| match o {
            Outcome::Refused(d) if *d != SpellCastDisposition::NotAvailable => Some(*d),
            _ => None,
        })
        .unwrap_or(SpellCastDisposition::NotAvailable);
    format!("{refusal:?}")
}

fn sweep(book: &SpellBook) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for i in 1..=book.source_len() {
        let index = std::num::NonZeroU32::new(u32::try_from(i).unwrap()).unwrap();
        let (spell, _active) = book.source_indexed(index).expect("indexed");
        for form in FORMS {
            let outcome = best(book, spell, form);
            // A spell that needs a target never casts without one. The engine refuses it with a
            // typed disposition; the wire mapping to `TargetRequired` is the dispatch layer's.
            if spell.needs_target
                && form != Form::AttackTarget
                && matches!(spell.execution, Execution::Effects(_))
            {
                assert_ne!(outcome, "Cast", "{} cast without its target", spell.key);
            }
            *counts
                .entry(format!(
                    "{}{} {:?} {outcome}",
                    family(spell),
                    if spell.needs_target { "+target" } else { "" },
                    form
                ))
                .or_default() += 1;
        }
    }
    counts
}

const GOLDEN: &[(&str, u32)] = &[
    ("Conjure AttackTarget Rejected", 48),
    ("Conjure None Cast", 48),
    ("Conjure Position Rejected", 48),
    ("Effects AttackTarget Cast", 90),
    ("Effects None Cast", 72),
    ("Effects None Rejected", 18),
    ("Effects Position Cast", 90),
    ("Effects+target AttackTarget Cast", 34),
    ("Effects+target AttackTarget TargetIllegal", 2),
    ("Effects+target None Rejected", 36),
    ("Effects+target Position Rejected", 36),
    ("Native/familiar_cast AttackTarget Classified", 9),
    ("Native/familiar_cast None Classified", 9),
    ("Native/familiar_cast Position Classified", 9),
    ("Native/focus AttackTarget Classified", 2),
    ("Native/focus None Classified", 2),
    ("Native/focus Position Classified", 2),
    ("Native/native_combat AttackTarget Classified", 19),
    ("Native/native_combat None Classified", 19),
    ("Native/native_combat Position Classified", 19),
    ("Native/native_combat+target AttackTarget Classified", 1),
    ("Native/native_combat+target None Classified", 1),
    ("Native/native_combat+target Position Classified", 1),
    ("Native/other_native AttackTarget Classified", 28),
    ("Native/other_native None Classified", 28),
    ("Native/other_native Position Classified", 28),
    ("Native/other_native+target AttackTarget Classified", 2),
    ("Native/other_native+target None Classified", 2),
    ("Native/other_native+target Position Classified", 2),
    ("Native/stance_cast AttackTarget Classified", 6),
    ("Native/stance_cast None Classified", 6),
    ("Native/stance_cast Position Classified", 6),
    ("PartyBuff AttackTarget Rejected", 5),
    ("PartyBuff None Rejected", 5),
    ("PartyBuff Position Rejected", 5),
];

#[test]
fn whole_book_is_castable_or_refused_with_golden_counts() {
    let book = book();
    assert_eq!(book.source_len(), 246);
    let counts = sweep(&book);
    assert_eq!(
        counts.values().sum::<u32>(),
        246 * 3,
        "246 book indices once per target form"
    );
    let golden: BTreeMap<String, u32> = GOLDEN.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect();
    assert_eq!(counts, golden);
}

fn spell_named<'a>(book: &'a SpellBook, name: &str) -> &'a SpellDefinition {
    (1..=book.source_len())
        .map(|i| std::num::NonZeroU32::new(u32::try_from(i).unwrap()).unwrap())
        .filter_map(|index| book.source_indexed(index))
        .map(|(spell, _)| spell)
        .find(|spell| spell.name == name)
        .unwrap_or_else(|| panic!("{name} in the book"))
}

/// The ordinary pipeline resolves the spell's damage; the real owner commit then reduces the
/// fixture creature next to the caster. Geometry containment is `ordinary_combat::lower`'s and
/// needs the Postgres-backed room; the creature is adjacent to the caster, inside both shapes.
fn assert_damages_fixture_creature(name: &str) {
    let book = book();
    let book = &book;
    let spell = spell_named(book, name);
    let (state, caster) = owner(Vocation::MasterSorcerer);
    let world = arena();
    let party = SoloParty(world.caster.clone());
    let paid = prepare_ordinary_owner_cast_with_caster(
        book,
        &state,
        spell,
        &facts(Form::None),
        SemanticTimeMicros::from_micros(1_000),
        &caster,
        Some(&party),
        None,
        &mut |_, maximum| maximum,
    )
    .unwrap_or_else(|d| panic!("{name}: {d:?}"));
    let magnitude = paid
        .resolution
        .effects
        .iter()
        .find_map(|effect| match effect {
            ResolvedEffect::Damage { magnitude, .. } => Some(*magnitude),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} resolves a damage effect"));
    assert!(magnitude > 0, "{name} damage magnitude");

    let (mut runtime, actor, session) = runtime_with_player(41);
    runtime
        .initialize_movement_test_position(
            actor,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .expect("caster position");
    let target = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 11,
            y: 10,
            floor: 7,
        })
        .expect("fixture creature");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            actor,
            session,
            CharacterCastFacts { level: 20, ..FACTS },
            (0, 0),
            SemanticTimeMicros::from_micros(100_000),
        )
        .expect("Character-owned actor state");
    let health = |runtime: &crate::foundation::ChannelRuntimeV1| {
        runtime
            .creature_combat_facts(target, 100)
            .expect("creature HP")
            .health
    };
    let before = health(&runtime);
    let occurrence = crate::ability::AbilityOccurrence::new(
        "spell-cast:sweep",
        crate::ability::RevisionSet::new("rules:1", "content:1", "world:1", "formula:1", "sim:1")
            .expect("revisions"),
    )
    .expect("occurrence");
    let batch = OwnerCombatBatch {
        caster: actor,
        attacker: CharacterId::decode(&[1, 0x90, 0, 0, 0, 41, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 41])
            .expect("character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding::from(occurrence),
        binding: b"qualified:spell-book-sweep".to_vec(),
        anchor: None,
        now_ms: 100,
        effects: vec![OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: "test:creature".into(),
                magnitude: magnitude.min(before - 1),
            },
        }],
        deferred: None,
    };
    let staged = runtime.stage_spell_batch(&batch).expect("physical stage");
    let proof = stage_player_batch(&runtime, &states, &batch, None).expect("player stage");
    assert!(
        commit_owner_batch(&mut runtime, &mut states, staged, Some(proof))
            .expect("one owner turn")
            .applied
    );
    assert!(health(&runtime) < before, "{name} reduced the creature");
}

#[test]
fn self_centred_area_damage_reduces_the_adjacent_creature() {
    assert_damages_fixture_creature("Rage of the Skies");
}

#[test]
fn directional_wave_damage_reduces_the_adjacent_creature() {
    assert_damages_fixture_creature("Fire Wave");
}
