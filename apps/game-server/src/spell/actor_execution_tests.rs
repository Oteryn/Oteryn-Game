#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use super::*;
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates, cast_in_channel, observe_vitals,
    tests::{runtime_with_player, wound},
};
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState, v1_spell_book};
use crate::spell::{
    Carrier, CooldownGroup, Cooldowns, Execution, HarmonyRole, ManaCost, SpellBook, SpellDefinition,
};
use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget};

fn at(ms: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(ms * 1000)
}

fn parameters(name: &str) -> Value {
    let export: Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .expect("canonical profiles");
    export["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|row| {
            row["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
        .expect("canonical focus")["execution"]["native_behavior"]["parameters"]
        .clone()
}

fn facts() -> CharacterCastFacts {
    CharacterCastFacts {
        vocation: Vocation::Monk,
        level: 200,
        magic_level: 0,
        max_health: 2000,
        max_mana: 2000,
        max_soul: 100,
    }
}

fn player(harmony: u8) -> PlayerSpellState {
    let mut state = PlayerSpellState::new(facts(), harmony, 0).expect("real monk actor state");
    state.make_playable(at(0)).expect("initialized monk");
    state.set_health_for_test(10);
    state
}

/// A supplied test book isolates PRIMARY COMMIT from the absent Premium and
/// Wheel admission services. Its canonical Focus parameters, 500 mana, group
/// and spell cooldowns are retained; test-only header gates are explicit.
/// The production V1 book and qualified production headers are never changed.
fn definition(name: &str) -> SpellDefinition {
    let mut spell = v1_spell_book()
        .expect("existing book")
        .indexed(NonZeroU32::new(1).expect("index"))
        .expect("template")
        .clone();
    spell.key = format!("test:spell/{}", name.replace(' ', "_"));
    spell.name = name.to_owned();
    spell.carrier = Carrier::Instant {
        words: format!("test {name}"),
        takes_parameter: false,
    };
    spell.vocations = BTreeSet::from([Vocation::Monk, Vocation::ExaltedMonk]);
    spell.level = 150;
    spell.premium = false;
    spell.learning_required = false;
    spell.mana = ManaCost::Fixed(500);
    spell.soul = 0;
    spell.cooldown_micros = if name == "focus harmony" {
        120_000_000
    } else {
        600_000_000
    };
    spell.groups = vec![CooldownGroup {
        key: "support".into(),
        cooldown_micros: 2_000_000,
    }];
    spell.needs_target = false;
    spell.target_or_direction = false;
    spell.self_target = false;
    spell.aggressive = false;
    spell.needs_weapon = false;
    spell.needs_shield = false;
    spell.chain = None;
    spell.harmony_role = None;
    spell.execution = Execution::ActorFocus {
        profile: parameters(name),
    };
    spell
}

fn intent() -> SpellCastIntent {
    SpellCastIntent {
        spell: NonZeroU32::new(1).expect("index"),
        target: SpellTarget::None,
        aim_at_target: false,
    }
}

#[test]
fn focus_applies_to_real_monk_and_vitals_without_a_second_anchor() {
    let mut state = player(3);
    let original = state.clone();
    let mut draws = Vec::new();
    let plan = apply_focus(
        &mut state,
        &parameters("focus serenity"),
        at(100),
        &mut |lo, hi| {
            draws.push((lo, hi));
            hi
        },
    )
    .expect("focus primary effect");
    assert_eq!(draws, [(88, 102)]);
    assert_eq!(state.vitals().health, 112);
    assert_eq!(state.monk_save_values(at(100)), Some((5, 7_000_000)));
    assert_eq!(plan.gained_charges, 2);
    assert_eq!(
        (state.vitals().mana, state.vitals().soul, state.revision()),
        (
            original.vitals().mana,
            original.vitals().soul,
            original.revision()
        )
    );
}

#[test]
fn full_focus_skips_heal_but_still_forces_serene_and_harmony_preserves_existing_force() {
    let mut state = player(5);
    let mut draws = 0;
    let plan = apply_focus(
        &mut state,
        &parameters("focus serenity"),
        at(100),
        &mut |_, _| {
            draws += 1;
            0
        },
    )
    .expect("full focus");
    assert_eq!(draws, 0);
    assert_eq!(state.vitals().health, 10);
    assert!(plan.healing.is_none());
    assert_eq!(state.monk_save_values(at(100)), Some((5, 7_000_000)));
    apply_focus(
        &mut state,
        &parameters("focus harmony"),
        at(200),
        &mut |_, _| {
            draws += 1;
            0
        },
    )
    .expect("keep force");
    assert_eq!(draws, 0);
    assert_eq!(state.monk_save_values(at(200)), Some((5, 6_900_000)));
}

#[test]
fn altered_canonical_parameters_and_invalid_draw_fail_without_partial_mutation() {
    let canonical = parameters("focus serenity");
    let mut invalid = Vec::new();
    let mut p = canonical.clone();
    p["harmony_max"] = serde_json::json!(6);
    invalid.push(p);
    let mut p = canonical.clone();
    p["serene_ms"] = serde_json::json!(8000);
    invalid.push(p);
    let mut p = canonical.clone();
    p["harmony_gain_healing"]["sustain_percent"] = serde_json::json!(0);
    invalid.push(p);
    let mut p = canonical.clone();
    p["hidden_operation"] = serde_json::json!(true);
    invalid.push(p);
    for p in invalid {
        let mut state = player(3);
        let before = state.clone();
        let mut draws = 0;
        assert_eq!(
            apply_focus(&mut state, &p, at(100), &mut |_, _| {
                draws += 1;
                100
            }),
            Err(SpellCastDisposition::Rejected)
        );
        assert_eq!(state, before);
        assert_eq!(draws, 0);
    }
    let mut state = player(3);
    let before = state.clone();
    assert_eq!(
        apply_focus(&mut state, &canonical, at(100), &mut |_, hi| hi + 1),
        Err(SpellCastDisposition::Rejected)
    );
    assert_eq!(
        state, before,
        "fill and forced Serene cannot escape a refused healing draw"
    );
}

#[test]
fn missing_uninitialized_or_vocation_mismatched_monk_state_is_atomic() {
    let mut missing = player(3);
    missing.monk = None;
    let uninitialized = PlayerSpellState::new(facts(), 3, 0).expect("not yet initialized");
    let mut wrong_vocation = player(3);
    wrong_vocation.facts.vocation = Vocation::Druid;
    let mut invalid_health = player(3);
    invalid_health.health = invalid_health.facts.max_health + 1;
    for mut state in [missing, uninitialized, wrong_vocation, invalid_health] {
        let before = state.clone();
        let mut draws = 0;
        assert_eq!(
            apply_focus(
                &mut state,
                &parameters("focus serenity"),
                at(100),
                &mut |_, _| {
                    draws += 1;
                    0
                }
            ),
            Err(SpellCastDisposition::Rejected)
        );
        assert_eq!(state, before);
        assert_eq!(draws, 0);
    }
}

#[test]
fn focus_resets_known_spenders_and_unresolved_groups_but_preserves_unknown_individuals() {
    let mut spender = definition("focus harmony");
    spender.key = "test:spender".into();
    spender.harmony_role = Some(HarmonyRole::Spender);
    let mut builder = definition("focus serenity");
    builder.key = "test:builder".into();
    builder.harmony_role = Some(HarmonyRole::Builder);
    let book = SpellBook::new(vec![spender, builder]).expect("classified book");
    let entries = BTreeMap::from([
        ("test:spender".into(), at(5000)),
        ("test:builder".into(), at(5000)),
        ("unknown".into(), at(5000)),
    ]);
    let mut cooldowns = Cooldowns {
        spells: entries.clone(),
        groups: entries,
    };
    let mut state = player(5);
    let harmony = apply_focus(
        &mut state,
        &parameters("focus harmony"),
        at(0),
        &mut |a, _| a,
    )
    .expect("harmony");
    let before = cooldowns.clone();
    cooldowns.reset_for_focus(&book, &harmony);
    assert_eq!(cooldowns, before);
    let serenity = apply_focus(
        &mut state,
        &parameters("focus serenity"),
        at(0),
        &mut |a, _| a,
    )
    .expect("serenity");
    cooldowns.reset_for_focus(&book, &serenity);
    assert_eq!(cooldowns.spell_ready_at("test:spender"), None);
    assert_eq!(cooldowns.spell_ready_at("test:builder"), Some(at(5000)));
    assert_eq!(cooldowns.spell_ready_at("unknown"), Some(at(5000)));
    assert_eq!(cooldowns.group_ready_at("test:spender"), None);
    assert_eq!(cooldowns.group_ready_at("test:builder"), Some(at(5000)));
    assert_eq!(cooldowns.group_ready_at("unknown"), None);
}

#[test]
fn channel_focus_serenity_commits_heal_fill_force_and_payment_in_one_revision_then_rearms() {
    let (runtime, actor, session) = runtime_with_player(0x52);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, facts(), (3, 0), at(0))
        .expect("owned real actor");
    wound(&mut states, actor, session, 10);
    let book = SpellBook::new(vec![definition("focus serenity")]).expect("supplied test book");
    let result = cast_in_channel(
        &runtime,
        &mut states,
        &book,
        actor,
        session,
        7,
        &intent(),
        at(100),
    );
    assert_eq!(result.disposition, SpellCastDisposition::Cast);
    let (revision, vitals) =
        observe_vitals(&runtime, &states, actor, session).expect("committed vitals");
    assert_eq!(
        (revision, vitals.harmony, vitals.mana, vitals.soul),
        (2, 5, 1500, 100)
    );
    assert!((98..=112).contains(&vitals.health));
    assert!(vitals.serene);
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, at(100)),
        Some((5, 7_000_000))
    );
    let before = (revision, vitals);
    for (command, now) in [(8, 101), (9, 2200)] {
        assert_eq!(
            cast_in_channel(
                &runtime,
                &mut states,
                &book,
                actor,
                session,
                command,
                &intent(),
                at(now)
            )
            .disposition,
            SpellCastDisposition::CoolingDown,
            "own cooldown must be rearmed after Focus reset"
        );
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some(before)
        );
    }
    states
        .initialize(&runtime, actor, session, facts(), (0, 0), at(3100))
        .expect("same actor reattach");
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some(before)
    );
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, at(3100)),
        Some((5, 4_000_000))
    );
}

#[test]
fn channel_focus_harmony_uses_existing_cast_checks_and_stale_session_is_refused() {
    let (runtime, actor, session) = runtime_with_player(0x53);
    let (_, _, stale_session) = runtime_with_player(0x54);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, facts(), (3, 0), at(0))
        .expect("actor");
    wound(&mut states, actor, session, 10);
    let original = observe_vitals(&runtime, &states, actor, session).expect("before");
    let book = SpellBook::new(vec![definition("focus harmony")]).expect("test book");
    assert_eq!(
        cast_in_channel(
            &runtime,
            &mut states,
            &book,
            actor,
            stale_session,
            1,
            &intent(),
            at(100)
        )
        .disposition,
        SpellCastDisposition::Rejected
    );
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some(original)
    );
    let mut canonical_level = definition("focus harmony");
    canonical_level.level = 275;
    let gated = SpellBook::new(vec![canonical_level]).expect("level gated book");
    assert_eq!(
        cast_in_channel(
            &runtime,
            &mut states,
            &gated,
            actor,
            session,
            2,
            &intent(),
            at(100)
        )
        .disposition,
        SpellCastDisposition::LevelTooLow
    );
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some(original)
    );
    assert_eq!(
        cast_in_channel(
            &runtime,
            &mut states,
            &book,
            actor,
            session,
            3,
            &intent(),
            at(100)
        )
        .disposition,
        SpellCastDisposition::Cast
    );
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, at(100)),
        Some((5, 0))
    );
    let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("after");
    assert_eq!((revision, vitals.harmony, vitals.mana), (2, 5, 1500));
    assert!((98..=112).contains(&vitals.health));
}

#[test]
fn channel_cost_or_parameter_refusal_leaves_harmony_vitals_and_revision_unchanged() {
    let (runtime, actor, session) = runtime_with_player(0x55);
    let mut states = ChannelSpellStates::default();
    let low_mana = CharacterCastFacts {
        max_mana: 400,
        ..facts()
    };
    states
        .initialize(&runtime, actor, session, low_mana, (3, 0), at(0))
        .expect("actor");
    wound(&mut states, actor, session, 10);
    let before = observe_vitals(&runtime, &states, actor, session).expect("before");
    let book = SpellBook::new(vec![definition("focus serenity")]).expect("test book");
    assert_eq!(
        cast_in_channel(
            &runtime,
            &mut states,
            &book,
            actor,
            session,
            1,
            &intent(),
            at(100)
        )
        .disposition,
        SpellCastDisposition::NotEnoughMana
    );
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some(before)
    );
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, at(100)),
        Some((3, 0))
    );

    let (runtime, actor, session) = runtime_with_player(0x56);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, facts(), (3, 0), at(0))
        .expect("actor");
    wound(&mut states, actor, session, 10);
    let before = observe_vitals(&runtime, &states, actor, session).expect("before");
    let mut invalid = definition("focus serenity");
    let Execution::ActorFocus { profile } = &mut invalid.execution else {
        panic!("focus fixture");
    };
    profile["serene_ms"] = serde_json::json!(8000);
    let book = SpellBook::new(vec![invalid]).expect("invalid profile test book");
    assert_eq!(
        cast_in_channel(
            &runtime,
            &mut states,
            &book,
            actor,
            session,
            2,
            &intent(),
            at(100)
        )
        .disposition,
        SpellCastDisposition::Rejected
    );
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some(before)
    );
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, at(100)),
        Some((3, 0))
    );
}
