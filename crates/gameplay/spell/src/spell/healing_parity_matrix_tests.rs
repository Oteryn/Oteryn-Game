//! Healing parity matrix, spell adapter: drives the real owners for every row of
//! `docs/reference/parity/healing.json` whose case key starts with `spell.`.
//!
//! The path is the one a cast takes: the production spell catalogue compiled to a `SpellBook`,
//! `prepare_ordinary_owner_cast_with_caster` (formula bounds, mana, cooldown and group cooldown,
//! vocation and level gates), and, for rows marked `final`, the `PreparedMagnitudeOwner` that
//! applies the Wheel flat healing, item healing, named-spell Sustain and Serene stages. Nothing
//! is re-implemented here. The potion adapter lives in the dispatch crate
//! (`item_use_healing_parity_tests.rs`) because the item-use gate is private to it.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
use super::cast::{CharacterCastFacts, PlayerSpellState, prepare_ordinary_owner_cast_with_caster};
use super::combat_batch::LiveActorBinding;
use super::magnitude_owner::{
    ExactRatio, MagnitudeOwnedAttributes, MagnitudeOwnerBinding, MagnitudePolicy,
    PreparedMagnitudeOwner,
};
use super::native_combat::{CombatPlan, Element, MagnitudePlan, NativeCombatPlan};
use super::*;
use crate::foundation::{ChannelId, ChannelRuntimeV1, CharacterId, GameSessionId, NodeId, WorldId};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_content::content::item_spell_augments::{
    ItemSpellAugment, ItemSpellAugmentKind,
};
use oteryn_gameplay_spell_rules::native_actor_states::StandardStance;
use oteryn_simulation_determinism::SemanticTimeMicros;
use sha2::{Digest, Sha256};

#[path = "parity_rows.rs"]
mod parity_rows;
use parity_rows::{Observed, Row, Status};

const CATALOG: &[u8] =
    include_bytes!("../../../../../content/abilities/definitions/player-spells.json");
const SELECTION: &[u8] =
    include_bytes!("../../../../../content/abilities/definitions/player-spell-selection.json");
const HEALING: &str = include_str!("../../../../../docs/reference/parity/healing.json");

/// The matrix may grow; it must never shrink. Raise this with the data, never lower it.
const MIN_ROWS: usize = 721;
const NOW: u64 = 1_000;

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn book() -> SpellBook {
    let catalog =
        executable_catalog::compile(CATALOG, &hex(CATALOG)).expect("production catalogue compiles");
    let selections =
        executable_catalog::compile_source_selection(SELECTION, &hex(SELECTION), &catalog)
            .expect("selection");
    catalog
        .into_spell_book_with_selections(&selections)
        .expect("book")
}

fn vocation(name: &str) -> Vocation {
    match name {
        "elite_knight" => Vocation::EliteKnight,
        "royal_paladin" => Vocation::RoyalPaladin,
        "master_sorcerer" => Vocation::MasterSorcerer,
        "elder_druid" => Vocation::ElderDruid,
        "exalted_monk" => Vocation::ExaltedMonk,
        other => panic!("unknown vocation {other}"),
    }
}

fn find<'a>(book: &'a SpellBook, key: &str) -> &'a SpellDefinition {
    (1..=book.source_len())
        .filter_map(|i| std::num::NonZeroU32::new(u32::try_from(i).ok()?))
        .filter_map(|index| book.source_indexed(index))
        .map(|(spell, _)| spell)
        .find(|spell| spell.key == key)
        .unwrap_or_else(|| panic!("spell {key} is in the production catalogue"))
}

fn owner(row: &Row, vocation: Vocation) -> (PlayerSpellState, CasterState) {
    let (level, magic_level) = (
        u32::try_from(row.int("level")).expect("level"),
        u32::try_from(row.int("ml") + row.int("item_ml")).expect("ml"),
    );
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation,
            level,
            magic_level,
            max_health: 20_000,
            max_mana: 50_000,
            max_soul: 200,
        },
        u8::try_from(row.int("harmony")).expect("harmony"),
        0,
    )
    .expect("state");
    state
        .apply_owner_premium_transition(true, Some(u64::MAX / 2))
        .expect("premium");
    state
        .make_playable(SemanticTimeMicros::from_micros(0))
        .expect("playable");
    if row.str_or("stance") == "sustain" {
        state.stance = Some(StandardStance::Sustain);
    }
    let vitals = state.vitals();
    let caster = CasterState {
        harmony_multiplier: harmony::HarmonyMultiplier::ONE,
        vocation,
        level,
        magic_level,
        premium: CasterPremium::Current,
        mana: vitals.mana,
        max_mana: 50_000,
        soul: vitals.soul,
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

fn at(x: i32) -> chain::TilePosition {
    chain::TilePosition { x, y: 10, floor: 7 }
}

fn operational(spell: &SpellDefinition) -> OperationalCastFacts {
    let targeted = spell.needs_target;
    // A heal that only reaches the caster (or own summons) is aimed at the caster itself.
    let own = matches!(
        spell.allowed_targets,
        AllowedTargets::SelfOnly | AllowedTargets::SelfOrOwnSummons
    );
    let creature = if own { 1 } else { 2 };
    OperationalCastFacts {
        caster_position: at(10),
        target_position: targeted.then(|| at(11)),
        target: targeted.then(|| target::CastTarget {
            caster: 1,
            creature,
            actor: format!("actor:{creature}"),
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

fn magnitude_attributes(row: &Row, binding: MagnitudeOwnerBinding) -> MagnitudeOwnedAttributes {
    let mut item_augments = Vec::new();
    if row.has("item_base_healing_permyriad") {
        item_augments.push(ItemSpellAugment {
            spell: row.str("spell_name").to_lowercase(),
            kind: ItemSpellAugmentKind::BaseHealing,
            value: u32::try_from(row.int("item_base_healing_permyriad")).expect("augment"),
        });
    }
    MagnitudeOwnedAttributes {
        item_augments,
        policy: MagnitudePolicy::Strict,
        owner_revisions: None,
        binding,
        base_critical_chance_permyriad: Some(0),
        base_critical_extra_permyriad: Some(0),
        equipment_critical_chance_permyriad: Some(0),
        equipment_critical_extra_permyriad: Some(0),
        fatal_chance_permyriad: Some(ExactRatio {
            numerator: 0,
            denominator: 1,
        }),
        armor_penetration_permyriad: Some(0),
        elemental_pierce_permyriad: Some(vec![]),
        wheel_effects: None,
        wheel_flat_damage: Some(0),
        wheel_flat_healing: Some(i32::try_from(row.int("wheel_flat_healing")).expect("flat")),
        source_damage_multiplier_percent: Some(0),
        source_healing_multiplier_percent: Some(
            u32::try_from(row.int("item_healing_percent")).expect("item healing"),
        ),
        healing_dealt_percent: Some(100),
        monster_armor_disabled: Some(false),
        inactive_extra_stages: Some(vec![]),
    }
}

/// The base heal bounds pushed through the magnitude owner, once at each bound.
fn finish(
    row: &Row,
    spell: &SpellDefinition,
    state: &PlayerSpellState,
    base: (i64, i64),
) -> (i64, i64) {
    let uuid = |tag: u8| [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = WorldId::decode(&uuid(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(0x61)).expect("channel"),
        NodeId::decode(&uuid(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .expect("runtime");
    let session = GameSessionId::decode(&uuid(32)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let caster = runtime.commit_fresh_session(reservation).expect("actor");
    runtime
        .initialize_first_entry_position(caster)
        .expect("position");
    let attributes = magnitude_attributes(
        row,
        MagnitudeOwnerBinding {
            actor: caster,
            session,
            character: CharacterId::decode(&uuid(42)).expect("character"),
            lease_generation: 1,
            player_revision: state.revision(),
            content_digest: runtime.content_pin().server_artifact_digest(),
        },
    );
    let bindings = vec![LiveActorBinding {
        source_id: 1,
        actor: caster,
        target_atom: combat_execution::actor_atom(caster),
    }];
    let run = |magnitude: i64| -> i64 {
        let hit = MagnitudePlan {
            target: 1,
            magnitude,
            bonus_percent: 0,
            side_percent: None,
            delay_ms: 0,
        };
        let plan = NativeCombatPlan::Combat(CombatPlan {
            element: Element::Healing,
            area: None,
            hits: vec![hit.clone()],
            cooldown_ms: None,
            shared_cooldown: None,
            reduce_all_spell_cooldowns_ms: 0,
            block_armor: false,
            use_weapon_charges: false,
            weapon_missile: false,
            dispel_paralysis_targets: vec![],
            resolve_critical_and_fatal_once: false,
        });
        let mut prepared = PreparedMagnitudeOwner::qualify(
            &runtime,
            state,
            caster,
            session,
            Some(&attributes),
            &plan,
            &bindings,
            &[],
            100,
        )
        .expect("magnitude owner qualifies");
        prepared.select_item_spell(spell).expect("named spell");
        prepared
            .finish(&plan, &hit, &bindings[0], &mut |_, _| {
                panic!("healing draws no critical")
            })
            .expect("finished heal")
    };
    (run(base.0), run(base.1))
}

fn deadline(state: &PlayerSpellState, family: &str) -> Option<i64> {
    state
        .cooldowns
        .canonical_deadlines()
        .into_iter()
        .find(|(key, _)| key.starts_with(family))
        .map(|(_, micros)| i64::try_from(micros).expect("micros"))
}

fn observe(book: &SpellBook, row: &Row) -> Observed {
    let spell = find(book, &format!("candidate:spell/{}", row.str("spell")));
    let mut seen = Observed::new();
    let ManaCost::Fixed(mana) = spell.mana else {
        panic!("{} has a percent mana cost", spell.key);
    };
    if let Execution::NativeProfile(_) = spell.execution {
        // A native spell has its own dispatch owner (mass_spirit_mend: native_combat.rs); this
        // adapter reads the definition the cast gate pays from.
        let allowed = spell.vocations.contains(&vocation(row.str("vocation")))
            && u32::try_from(row.int("level")).expect("level") >= spell.level;
        seen.insert("castable", i64::from(allowed));
        seen.insert("mana", i64::from(mana));
        seen.insert(
            "harmony_spender",
            i64::from(spell.harmony_role == Some(HarmonyRole::Spender)),
        );
        seen.insert(
            "cooldown_ms",
            i64::try_from(spell.cooldown_micros / 1000).expect("ms"),
        );
        if let Some(group) = spell.groups.first() {
            seen.insert(
                "group_cooldown_ms",
                i64::try_from(group.cooldown_micros / 1000).expect("ms"),
            );
        }
        return seen;
    }
    let vocation = vocation(row.str("vocation"));
    let (state, mut caster) = owner(row, vocation);
    caster.harmony_multiplier = state.owned_harmony_multiplier(spell).expect("harmony");
    let mut bounds = None;
    let paid = prepare_ordinary_owner_cast_with_caster(
        book,
        &state,
        spell,
        &operational(spell),
        SemanticTimeMicros::from_micros(NOW),
        &caster,
        None,
        None,
        &mut |low, high| {
            bounds = Some((low, high));
            low
        },
    );
    let paid = match paid {
        Ok(paid) => paid,
        Err(refusal) => {
            assert!(
                !spell.vocations.contains(&vocation)
                    || spell.level > u32::try_from(row.int("level")).expect("level"),
                "{}: a legal cast was refused: {refusal:?}",
                row.id
            );
            seen.insert("castable", 0);
            return seen;
        }
    };
    seen.insert("castable", 1);
    seen.insert("mana", i64::from(paid.anchor.paid_mana));
    seen.insert(
        "harmony_spender",
        i64::from(spell.harmony_role == Some(HarmonyRole::Spender)),
    );
    if mana == 0 {
        seen.insert("free_cast", 1);
    }
    let now = i64::try_from(NOW).expect("now");
    if let Some(ready) = deadline(&paid.next, "spell:") {
        seen.insert("cooldown_ms", (ready - now) / 1000);
    }
    if let Some(ready) = deadline(&paid.next, "group:") {
        seen.insert("group_cooldown_ms", (ready - now) / 1000);
    }
    let (low, high) = bounds.expect("a heal draws between its bounds");
    seen.insert("base_min", low);
    seen.insert("base_max", high);
    let (low, high) = if row.has("final") {
        finish(row, spell, &state, (low, high))
    } else {
        (low, high)
    };
    seen.insert("min", low);
    seen.insert("max", high);
    seen
}

fn rows() -> Vec<Row> {
    parity_rows::load(HEALING)
}

#[test]
fn spell_rows_follow_their_status_on_the_real_cast_owner() {
    let book = book();
    let mut ran = 0;
    for row in rows().iter().filter(|row| row.case.starts_with("spell.")) {
        assert_eq!(row.case, "spell.cast", "{}: unknown spell case", row.id);
        row.judge(&observe(&book, row));
        ran += 1;
    }
    assert!(ran > 0);
}

#[test]
fn healing_matrix_never_shrinks_and_every_case_has_an_adapter() {
    let rows = rows();
    let count = |status| rows.iter().filter(|row| row.status == status).count();
    let (evidence, gap, pending) = (
        count(Status::Evidence),
        count(Status::KnownGap),
        count(Status::Pending),
    );
    eprintln!("HEALING_MATRIX evidence={evidence} known_gap={gap} pending={pending}");
    assert_eq!(evidence + gap + pending, rows.len());
    assert!(rows.len() >= MIN_ROWS, "the matrix lost rows");
    for row in &rows {
        assert!(
            matches!(row.case.as_str(), "spell.cast" | "potion.use"),
            "{}: no adapter for case {}",
            row.id,
            row.case
        );
    }
}
