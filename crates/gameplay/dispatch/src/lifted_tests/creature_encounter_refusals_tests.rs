//! W01c: these exact source attacks require their separately admitted Encounter owner.
//! Coordinator ANSWER 6543276153 on PR #2766 recorded the original eighteen refusals.
use crate::content::ProjectV2AuthoringProfileData as Data;
use crate::content::native_gameplay::NativeGameplayInput;
use crate::creature_auto_attack::AttackError;
use crate::creature_damage_spell::{SpellSource, composed::ComposedSource};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Route {
    CallbackCast,
    CallbackSpawn,
    SevenEncounter,
    SmellyCheese,
}

// Creature key, zero-based attack entry, Ability key, Encounter binding, production owner.
// Keys are kept explicit so adding, removing or retargeting one route changes this gate.
const ROUTED: &[(&str, usize, &str, &str, Route)] = &[
    (
        "oteryn:creature.the_destruction",
        8,
        "oteryn:ability.spell.destruction_summon",
        "oteryn:encounter.world_devourer",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.the_hunger",
        6,
        "oteryn:ability.spell.hunger_summon",
        "oteryn:encounter.world_devourer",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.the_rage",
        8,
        "oteryn:ability.spell.rage_summon",
        "oteryn:encounter.world_devourer",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.world_devourer",
        7,
        "oteryn:ability.spell.devourer_summon",
        "oteryn:encounter.world_devourer",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.cerebellum",
        1,
        "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head",
        "oteryn:encounter.field_fill_cerebellum",
        Route::SevenEncounter,
    ),
    (
        "oteryn:creature.charged_anomaly",
        5,
        "oteryn:ability.spell.charge_vortex",
        "oteryn:encounter.charged_anomaly",
        Route::SevenEncounter,
    ),
    (
        "oteryn:creature.charging_outburst",
        5,
        "oteryn:ability.spell.outburst_explode",
        "oteryn:encounter.charging_outburst",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.energy_pulse",
        0,
        "oteryn:ability.spell.energy_pulse_explosion",
        "oteryn:encounter.source_core_energy_pulse",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.eruption_of_destruction",
        1,
        "oteryn:ability.spell.eruption_of_destruction_explosion",
        "oteryn:encounter.eruption_of_destruction",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.foam_stalker",
        4,
        "oteryn:ability.creature.foam_stalker.field-fill-foamsplash",
        "oteryn:encounter.field_fill_foam_stalker",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.gaz_haragoth",
        10,
        "oteryn:ability.spell.gaz_haragoth_death",
        "oteryn:encounter.gaz_haragoth",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.gaz_haragoth",
        11,
        "oteryn:ability.spell.gaz_haragoth_summon",
        "oteryn:encounter.gaz_haragoth",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.lady_tenebris",
        3,
        "oteryn:ability.spell.tenebris_summon",
        "oteryn:encounter.lady_tenebris",
        Route::CallbackSpawn,
    ),
    (
        "oteryn:creature.lady_tenebris",
        4,
        "oteryn:ability.spell.tenebris_ultimate",
        "oteryn:encounter.lady_tenebris",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.plagirath",
        6,
        "oteryn:ability.spell.plagirath_bog",
        "oteryn:encounter.plagirath",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.rootthing_amber_shaper",
        4,
        "oteryn:ability.spell.rotthingshaper",
        "oteryn:encounter.source_core_rootthing_amber_shaper",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.rootthing_nutshell",
        3,
        "oteryn:ability.spell.rotthingwave",
        "oteryn:encounter.source_core_rootthing_nutshell",
        Route::CallbackCast,
    ),
    (
        "oteryn:creature.smelly_cheese",
        0,
        "oteryn:ability.creature.smelly_cheese.berserk_bridge",
        "oteryn:encounter.crystal_smelly_cheese",
        Route::SmellyCheese,
    ),
];

// Nothing remains refused: all eighteen have a fail-closed, source-qualified production owner.
const STILL_REFUSED: &[(&str, usize, &str, &str)] = &[];

#[test]
fn all_eighteen_encounter_attacks_have_a_production_route() -> Result<(), AttackError> {
    #[derive(serde::Deserialize)]
    struct CastWitness {
        cases: std::collections::BTreeMap<String, CastCase>,
    }
    #[derive(serde::Deserialize)]
    struct CastCase {
        caster: String,
        index: usize,
        encounter: String,
    }
    #[derive(serde::Deserialize)]
    struct SevenWitness {
        cases: Vec<SevenCase>,
    }
    #[derive(serde::Deserialize)]
    struct SpawnWitness {
        cases: Vec<SpawnCase>,
    }
    #[derive(serde::Deserialize)]
    struct SpawnCase {
        caster: String,
        parent: String,
        index: usize,
        encounter: String,
    }
    #[derive(serde::Deserialize)]
    struct SevenCase {
        creature: String,
        ability: String,
        list: String,
        index: usize,
        encounter: String,
    }

    let cast: CastWitness = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_cast_fixture.json"
    ))
    .map_err(|_| AttackError::InvalidSource)?;
    let spawn: SpawnWitness = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_spawn_fixture.json"
    ))
    .map_err(|_| AttackError::InvalidSource)?;
    let seven: SevenWitness = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_encounter_seven_fixture.json"
    ))
    .map_err(|_| AttackError::InvalidSource)?;

    let mut actual = BTreeSet::new();
    for (ability, case) in cast.cases {
        actual.insert((
            case.caster,
            case.index,
            ability,
            case.encounter,
            Route::CallbackCast,
        ));
    }
    for case in spawn.cases {
        if ROUTED.iter().any(|expected| {
            expected.0 == case.caster
                && expected.1 == case.index
                && expected.3 == case.encounter
                && expected.4 == Route::CallbackSpawn
        }) {
            actual.insert((
                case.caster,
                case.index,
                case.parent,
                case.encounter,
                Route::CallbackSpawn,
            ));
        }
    }
    for case in seven.cases {
        if case.list == "attacks"
            && ROUTED.iter().any(|expected| {
                expected.0 == case.creature
                    && expected.1 == case.index
                    && expected.3 == case.encounter
                    && expected.4 == Route::SevenEncounter
            })
        {
            actual.insert((
                case.creature,
                case.index,
                case.ability,
                case.encounter,
                Route::SevenEncounter,
            ));
        }
    }
    actual.insert((
        crate::smelly_encounter_dispatch::CREATURE.to_owned(),
        0,
        crate::smelly_encounter_dispatch::PARENT.to_owned(),
        "oteryn:encounter.crystal_smelly_cheese".to_owned(),
        Route::SmellyCheese,
    ));

    let expected = ROUTED
        .iter()
        .map(|&(creature, index, ability, encounter, route)| {
            (
                creature.to_owned(),
                index,
                ability.to_owned(),
                encounter.to_owned(),
                route,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(expected.len(), 18);
    assert!(STILL_REFUSED.is_empty());
    Ok(())
}

#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn monster_actual_encounter_attacks_are_owned_outside_generic_spell_admission()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("OTERYN_FULL_SPELL_TEST_MANIFEST")
        .ok_or("explicit actual source artifact manifest is required")?;
    let input = NativeGameplayInput::from_manifest(std::path::Path::new(&path))
        .map_err(|error| format!("{error:?}"))?;
    let world =
        crate::content::native_gameplay_test_world_id().map_err(|error| format!("{error:?}"))?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, &input)
        .map_err(|error| format!("{error:?}"))?;
    let native = crate::content::qualified_native_gameplay_test_state(room.compiled())
        .map_err(|error| format!("{error:?}"))?;
    let expected = ROUTED
        .iter()
        .map(|&(creature, index, ability, encounter, _)| (creature, index, ability, encounter))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        expected.len(),
        18,
        "each qualified source occurrence is named once"
    );
    let mut actual = BTreeSet::new();
    for record in &native.creature_profiles().records {
        let Some(definitions) = &record.attack_definitions else {
            continue;
        };
        let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|p| &p.data) else {
            continue;
        };
        for (index, attack) in behavior.attacks.iter().enumerate() {
            let Some(Data::Ability(ability)) = definitions
                .profiles
                .iter()
                .find(|p| p.target == attack.ability)
                .map(|p| &p.data)
            else {
                continue;
            };
            let Some(encounter) = ability
                .details
                .as_deref()
                .and_then(|d| d.encounter.as_ref())
            else {
                continue;
            };
            let case = (
                record.profile.target.key.as_str(),
                index,
                attack.ability.key.as_str(),
                encounter.key.as_str(),
            );
            assert!(
                expected.contains(&case),
                "unreviewed Encounter attack: {case:?}"
            );
            assert_eq!(encounter.family, crate::content::ProjectV2Family::Encounter);
            assert_eq!(encounter.revision, "definition-r1");
            assert!(
                matches!(
                    SpellSource::from_native(
                        &record.profile.target,
                        index,
                        &definitions.records,
                        &definitions.profiles,
                        native.source_digest(),
                    ),
                    Err(AttackError::UnsupportedShape)
                ),
                "generic spell admission must refuse this Encounter binding: {case:?}"
            );
            assert!(
                matches!(
                    ComposedSource::from_native_parts(
                        &record.profile.target,
                        index,
                        &definitions.records,
                        &definitions.profiles,
                        native.source_digest(),
                    ),
                    Err(AttackError::UnsupportedShape)
                ),
                "bounded composed admission must not replace an Encounter owner: {case:?}"
            );
            assert!(
                native
                    .source_item_ability(&record.profile.target, &attack.ability)
                    .is_none(),
                "the Item writer must not claim an Encounter callback: {case:?}"
            );
            actual.insert(case);
        }
    }
    assert_eq!(
        actual, expected,
        "all eighteen source-owned Encounter attacks remain outside generic admission"
    );
    Ok(())
}
