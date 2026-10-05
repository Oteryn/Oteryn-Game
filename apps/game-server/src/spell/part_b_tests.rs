//! Engine tests of part B (`OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md`): the B.5 target rule
//! `not_self` (`targeting.allowed_targets`, `allow_on_self`) and the rune refusals, the B.3 spoken
//! matching (P1, P2) and location phrase (P7), and the rejection of the Part B keys that have no
//! implementation yet.
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]

use serde_json::{Value, json};

use super::authoring::spell_from_bundle;
use super::locate::{Compass, LocateDistance, LocateLevel, locate};
use super::part_d_tests::{
    CASTER, HEALING, OTHER_PLAYER, SUMMON, at_ms, caster, lowest, occurrence, target,
    ultimate_healing_rune,
};
use super::plan::effect_plan;
use super::target::AllowedTargets;
use super::*;
use crate::ability::AbilityEngine;

const LIGHT_HEALING: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-light_healing/spell.json"
);

/// Nature's Embrace as the B.5 authoring shape: a plain heal Ability aimed by player name or at the
/// attacked creature, `allowed_targets` `not_self` (N8833, F1190771, BR436996; the formula
/// belongs to the healing family, so the Light Healing formula stands in).
fn natures_embrace_bundle() -> (Value, Value) {
    let mut spell: Value = serde_json::from_str(LIGHT_HEALING).expect("light healing spell");
    let body = &mut spell["spell"];
    body["identity"]["key"] = json!("candidate:spell/natures_embrace");
    body["name"] = json!("Nature's Embrace");
    body["words"] = json!("exura gran sio");
    body["requirements"] = json!({
        "vocations": ["druid", "elder_druid"],
        "level": 275,
        "premium": true,
        "learning_required": false
    });
    body["costs"] = json!({ "mana": 400, "soul": 0 });
    body["cooldown_ms"] = json!(60_000);
    body["base_power"] = json!(2000);
    let targeting = &mut body["targeting"];
    targeting["self_target"] = json!(false);
    targeting["needs_target"] = json!(true);
    targeting["range_tiles"] = json!(7);
    targeting["parameter"] = json!("player_name");
    targeting["allowed_targets"] = json!("not_self");
    let mut dependencies: Value = serde_json::from_str(HEALING).expect("healing dependencies");
    dependencies["abilities"][0]["needs_target"] = json!(true);
    (spell, dependencies)
}

fn natures_embrace() -> SpellDefinition {
    let (spell, dependencies) = natures_embrace_bundle();
    spell_from_bundle(&spell, &dependencies).expect("Nature's Embrace admitted")
}

fn druid() -> CasterState {
    CasterState {
        level: 300,
        premium: true,
        ..caster(Vocation::Druid)
    }
}

/// B.5 test 1: Nature's Embrace on the caster is refused with mana and cooldown unchanged; on
/// another player it heals that player.
#[test]
fn natures_embrace_refuses_the_caster_and_heals_another_player() {
    let embrace = natures_embrace();
    assert_eq!(embrace.allowed_targets, AllowedTargets::NotSelf);
    assert!(matches!(
        embrace.carrier,
        Carrier::Instant {
            takes_parameter: true,
            ..
        }
    ));
    let cooldowns = Cooldowns::default();
    assert_eq!(
        resolve_targeted_cast(
            &embrace,
            &druid(),
            &cooldowns,
            at_ms(0),
            target(CASTER, None),
            &mut lowest,
        ),
        Err(CastRejection::TargetNotAllowed)
    );
    // The refusal returns no resolution: no mana is spent and the cooldowns are the caller's own.
    assert_eq!(cooldowns.spell_ready_at(&embrace.key), None);
    let resolution = resolve_targeted_cast(
        &embrace,
        &druid(),
        &cooldowns,
        at_ms(0),
        target(OTHER_PLAYER, None),
        &mut lowest,
    )
    .expect("cast on another player");
    assert_eq!(resolution.mana_spent, 400);
    assert_eq!(
        resolution.cooldowns.spell_ready_at(&embrace.key),
        Some(at_ms(60_000))
    );
    // The heal also dispels paralysis (Canary `COMBAT_PARAM_DISPEL`), as Light Healing does.
    assert!(
        matches!(
            resolution.effects.as_slice(),
            [
                ResolvedEffect::Heal { .. },
                ResolvedEffect::RemoveCondition { .. }
            ]
        ),
        "{:?}",
        resolution.effects
    );
    let magnitude = resolution
        .effects
        .iter()
        .find_map(|effect| match effect {
            ResolvedEffect::Heal { magnitude } => Some(*magnitude),
            _ => None,
        })
        .expect("heal");
    let plan = effect_plan(
        &embrace,
        &resolution,
        "actor:1",
        Some("actor:2"),
        occurrence("cast:natures-embrace"),
        "channel:test",
    )
    .expect("plan");
    let mut engine = AbilityEngine::new();
    assert!(
        engine
            .commit(plan.effects.expect("heal"))
            .expect("commit")
            .applied()
    );
    assert_eq!(engine.fixture_health("actor:2"), Some(magnitude));
    assert_eq!(engine.fixture_health("actor:1"), None);
}

/// B.5: a summon is not the caster, so `not_self` allows it; the rule comes after the cast checks.
#[test]
fn not_self_follows_the_cast_checks() {
    let embrace = natures_embrace();
    assert!(
        resolve_targeted_cast(
            &embrace,
            &druid(),
            &Cooldowns::default(),
            at_ms(0),
            target(SUMMON, Some(CASTER)),
            &mut lowest,
        )
        .is_ok()
    );
    let low = CasterState {
        mana: 10,
        ..druid()
    };
    assert_eq!(
        resolve_targeted_cast(
            &embrace,
            &low,
            &Cooldowns::default(),
            at_ms(0),
            target(CASTER, None),
            &mut lowest,
        ),
        Err(CastRejection::NotEnoughMana { required: 400 })
    );
    // Without a target the spell never falls back to the caster.
    let facts = Cooldowns::default();
    assert_eq!(
        resolve_cast(&embrace, &druid(), &facts, at_ms(0), false, &mut lowest),
        Err(CastRejection::TargetRequired)
    );
    assert_eq!(
        resolve_cast(&embrace, &druid(), &facts, at_ms(0), true, &mut lowest),
        Err(CastRejection::TargetFactsRequired)
    );
}

/// B.5 tests 2 and 4: both monk vocations are refused the Ultimate Healing Rune (plain rune
/// vocations, B-QC3), and a refused use starts neither the rune nor the healing group cooldown.
#[test]
fn a_refused_rune_use_spends_nothing() {
    let rune = ultimate_healing_rune();
    for vocation in [Vocation::Monk, Vocation::ExaltedMonk] {
        assert_eq!(
            resolve_targeted_cast(
                &rune,
                &caster(vocation),
                &Cooldowns::default(),
                at_ms(0),
                target(CASTER, None),
                &mut lowest,
            ),
            Err(CastRejection::VocationCannotUse)
        );
    }
    // B.5 test 3: another player is refused; the refusal leaves the healing group ready, so the
    // same druid may use the rune on itself at once.
    let cooldowns = Cooldowns::default();
    assert_eq!(
        resolve_targeted_cast(
            &rune,
            &caster(Vocation::Druid),
            &cooldowns,
            at_ms(0),
            target(OTHER_PLAYER, None),
            &mut lowest,
        ),
        Err(CastRejection::TargetNotAllowed)
    );
    assert!(
        resolve_targeted_cast(
            &rune,
            &caster(Vocation::Druid),
            &cooldowns,
            at_ms(0),
            target(CASTER, None),
            &mut lowest,
        )
        .is_ok()
    );
}

/// B.5: `not_self` and Canary's `allowOnSelf` false are the same rule; the reader refuses the
/// combinations that cannot hold.
#[test]
fn not_self_and_allow_on_self_are_read_and_checked() {
    let read = |change: &dyn Fn(&mut Value)| {
        let (mut spell, dependencies) = natures_embrace_bundle();
        change(&mut spell["spell"]["targeting"]);
        spell_from_bundle(&spell, &dependencies).map(|spell| spell.allowed_targets)
    };
    assert_eq!(read(&|_| {}), Ok(AllowedTargets::NotSelf));
    assert_eq!(
        read(&|targeting| {
            targeting
                .as_object_mut()
                .expect("targeting")
                .remove("allowed_targets");
            targeting["allow_on_self"] = json!(false);
        }),
        Ok(AllowedTargets::NotSelf)
    );
    assert_eq!(
        read(&|targeting| targeting["allowed_targets"] = json!("any")),
        Ok(AllowedTargets::Any)
    );
    assert_eq!(
        read(&|targeting| {
            targeting["allowed_targets"] = json!("any");
            targeting["allow_on_self"] = json!(false);
        }),
        Ok(AllowedTargets::NotSelf)
    );
    for rule in ["self_only", "self_or_own_summons"] {
        assert!(
            read(&|targeting| {
                targeting["allowed_targets"] = json!(rule);
                targeting["allow_on_self"] = json!(false);
            })
            .is_err(),
            "{rule}"
        );
    }
    assert!(read(&|targeting| targeting["needs_target"] = json!(false)).is_err());
    assert!(read(&|targeting| targeting["self_target"] = json!(true)).is_err());
    assert!(read(&|targeting| targeting["allow_on_self"] = json!("no")).is_err());
}

/// An instant spell with the given words, for the spoken-matching tests.
fn instant(key: &str, words: &str, takes_parameter: bool) -> SpellDefinition {
    let (spell, dependencies) = natures_embrace_bundle();
    let spell = spell_from_bundle(&spell, &dependencies).expect("base spell");
    SpellDefinition {
        key: format!("candidate:spell/{key}"),
        name: key.to_owned(),
        carrier: Carrier::Instant {
            words: words.to_owned(),
            takes_parameter,
        },
        ..spell
    }
}

fn parameter_book() -> SpellBook {
    SpellBook::new(vec![
        instant("light_healing", "exura", false),
        instant("heal_friend", "exura sio", true),
        instant("find_person", "exiva", true),
        instant("summon_creature", "utevo res", true),
        instant("creature_illusion", "utevo res ina", true),
    ])
    .expect("unique words")
}

fn heard(book: &SpellBook, message: &str) -> Option<(String, Option<String>)> {
    book.spoken(message)
        .map(|spoken| (spoken.spell.name.clone(), spoken.parameter))
}

fn cast(name: &str, parameter: Option<&str>) -> Option<(String, Option<String>)> {
    Some((name.to_owned(), parameter.map(str::to_owned)))
}

/// B.3 test 1 (P1): the longest words win; a parameter spell alone has an empty parameter.
#[test]
fn the_longest_spoken_words_win() {
    let book = parameter_book();
    assert_eq!(
        heard(&book, "utevo res ina \"Rat\""),
        cast("creature_illusion", Some("Rat"))
    );
    assert_eq!(
        heard(&book, "utevo res rat"),
        cast("summon_creature", Some("rat"))
    );
    assert_eq!(heard(&book, "utevo res"), cast("summon_creature", None));
    assert_eq!(
        heard(&book, "  UTEVO   res  "),
        cast("summon_creature", None)
    );
    // Canary picks the longest prefix first, so `ina` glued to a letter is chat.
    assert_eq!(heard(&book, "utevo res inax"), None);
    // A spell without a parameter must match exactly.
    assert_eq!(heard(&book, "exura"), cast("light_healing", None));
    assert_eq!(heard(&book, "exura x"), None);
    assert_eq!(heard(&book, "exurax"), None);
    assert_eq!(heard(&book, "hello exura"), None);
    assert_eq!(heard(&book, "exiva"), cast("find_person", None));
    assert_eq!(heard(&book, "exivax"), None);
}

/// B.3 test 2 (P2): quoted and unquoted parameters keep their case; anything after the closing
/// quote, or a second unquoted word, makes the message chat.
#[test]
fn the_spoken_parameter_follows_the_quote_rules() {
    let book = parameter_book();
    assert_eq!(heard(&book, "exiva \"Foo Bar\" x"), None);
    assert_eq!(heard(&book, "exiva Foo Bar"), None);
    assert_eq!(
        heard(&book, "exiva \"Foo Bar\""),
        cast("find_person", Some("Foo Bar"))
    );
    assert_eq!(
        heard(&book, "exiva \"Foo Bar\"   "),
        cast("find_person", Some("Foo Bar"))
    );
    assert_eq!(
        heard(&book, "exiva \"Foo Bar"),
        cast("find_person", Some("Foo Bar"))
    );
    assert_eq!(
        heard(&book, "EXIVA Foo~"),
        cast("find_person", Some("Foo~"))
    );
    assert_eq!(heard(&book, "exiva \"\""), cast("find_person", None));
    assert_eq!(
        heard(&book, "exura sio \"Bubble"),
        cast("heal_friend", Some("Bubble"))
    );
    // Quoting applies only when the parameter opens with a quote.
    assert_eq!(heard(&book, "exiva Foo \"Bar\""), None);
    assert_eq!(
        heard(&book, "exiva Foo\"Bar\""),
        cast("find_person", Some("Foo\"Bar\""))
    );
    assert_eq!(
        heard(&book, "exiva Foo\""),
        cast("find_person", Some("Foo\""))
    );
}

/// Spoken words match case-insensitively, so words that differ only in case are one spell.
#[test]
fn spoken_words_are_unique_ignoring_case() {
    let error = SpellBook::new(vec![
        instant("find_person", "exiva", true),
        instant("shouted_find_person", "EXIVA", true),
    ])
    .expect_err("duplicate words");
    assert_eq!(error, SpellBookError::Words("EXIVA".to_owned()));
    let book = SpellBook::new(vec![instant("find_person", "Exiva", true)]).expect("one spell");
    assert_eq!(heard(&book, "exiva Foo"), cast("find_person", Some("Foo")));
}

fn phrase(dx: i32, dy: i32, dz: i32) -> String {
    locate(dx, dy, dz).phrase()
}

/// B.3 test 3 (P7): offsets are caster minus target.
#[test]
fn locate_gives_the_band_level_and_direction() {
    assert_eq!(phrase(10, -10, 0), "is to the south-west");
    assert_eq!(phrase(0, 300, 0), "is very far to the north");
    assert_eq!(
        locate(0, 300, 0),
        super::locate::Located {
            distance: LocateDistance::VeryFar,
            level: LocateLevel::Same,
            direction: Some(Compass::North),
        }
    );
    // Beside (d < 5): the floor decides the phrase and there is no direction.
    assert_eq!(phrase(4, -4, 0), "is standing next to you");
    assert_eq!(phrase(0, 0, 1), "is above you");
    assert_eq!(phrase(1, 2, -1), "is below you");
    assert_eq!(locate(4, 0, 0).direction, None);
    // Band edges 5, 101 and 251 tiles (F/TibiaMaps, QP2).
    assert_eq!(phrase(5, 0, 0), "is to the west");
    assert_eq!(phrase(-100, 0, 0), "is to the east");
    assert_eq!(phrase(101, 0, 0), "is far to the west");
    assert_eq!(phrase(250, 0, 2), "is far to the west");
    assert_eq!(phrase(251, 0, -2), "is very far to the west");
    assert_eq!(phrase(0, 5, 3), "is on a higher level to the north");
    assert_eq!(phrase(0, -5, -3), "is on a lower level to the south");
    // The tangent sectors: |t| < 0.4142 is west/east, < 2.4142 a diagonal, else north/south.
    assert_eq!(phrase(10, 4, 0), "is to the west");
    assert_eq!(phrase(10, 5, 0), "is to the north-west");
    assert_eq!(phrase(-10, -5, 0), "is to the south-east");
    assert_eq!(phrase(-10, 24, 0), "is to the north-east");
    assert_eq!(phrase(-10, 25, 0), "is to the north");
    assert_eq!(phrase(10, -25, 0), "is to the south");
}

/// S7/D13: the Part B `native_behavior` keys without an implementation stay rejected, as do the
/// family names. B.5 is admitted as `targeting.allowed_targets` `not_self`, not as a key.
#[test]
fn part_b_keys_without_a_runtime_stay_rejected() {
    for key in [
        "world_query",
        "house",
        "player_parameter",
        "item_grant",
        "caster_restriction",
        "cast_restriction",
        "house_access",
        "locate_message",
        "vertical_move",
        "creature_appearance",
        "summon_named_creature",
        "random_item_grant",
        "tile_item_operation",
        "owned_field_buff",
        "monster_ai_override",
    ] {
        let (mut spell, dependencies) = natures_embrace_bundle();
        spell["spell"]["execution"] =
            json!({ "native_behavior": { "key": key, "parameters": {} } });
        let error = spell_from_bundle(&spell, &dependencies).expect_err(key);
        assert!(!error.to_string().is_empty(), "{key}: {error}");
    }
}

/// A candidate data completion cannot discard an unsupported executor precondition.
#[test]
fn unimplemented_ability_preconditions_stay_rejected() {
    for (field, value) in [
        ("target_selection", json!("caster_or_top_creature")),
        ("zero_damage_health_path", json!(true)),
    ] {
        let (spell, mut dependencies) = natures_embrace_bundle();
        dependencies["abilities"][0][field] = value;
        let admitted =
            spell_from_bundle(&spell, &dependencies).expect("typed retained precondition");
        assert_eq!(
            resolve_targeted_cast(
                &admitted,
                &druid(),
                &Cooldowns::default(),
                at_ms(0),
                target(OTHER_PLAYER, None),
                &mut |_, _| panic!("draw before owning precondition")
            ),
            Err(CastRejection::OperationalFactsRequired),
            "{field}"
        );
    }
}

/// Timing on the caster differs from impact presentation and cannot be silently ignored.
#[test]
fn unimplemented_caster_effect_semantics_stay_rejected() {
    for (field, value) in [
        (
            "caster_effect_asset_binding",
            json!("canary.appearance:effect/magic_blue"),
        ),
        ("caster_effect_timing", json!("before_combat")),
    ] {
        let (spell, mut dependencies) = natures_embrace_bundle();
        let effect = &mut dependencies["effects"][0];
        if !effect["presentation"].is_object() {
            effect["presentation"] = json!({});
        }
        effect["presentation"][field] = value;
        let admitted =
            spell_from_bundle(&spell, &dependencies).expect("typed retained presentation");
        assert_eq!(
            resolve_targeted_cast(
                &admitted,
                &druid(),
                &Cooldowns::default(),
                at_ms(0),
                target(OTHER_PLAYER, None),
                &mut |_, _| panic!("draw before owning presentation")
            ),
            Err(CastRejection::OperationalFactsRequired),
            "{field}"
        );
    }
}
