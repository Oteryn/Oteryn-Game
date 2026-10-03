#![allow(clippy::expect_used)]

//! OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §4-§5: a monster admitted as executable
//! Reference records plus declarative authoring profiles.

use oteryn_game_server::content::*;
use serde_json::{Value, json};

const REVISION: &str = "definition-r1";

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 65_536,
        max_total_bytes: 262_144,
        max_json_depth: 24,
        max_decoded_fields: 4_096,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 64,
        max_import_records: 8,
        max_reimport_states: 8,
    }
}

fn identity(family: &str, key: &str) -> DefinitionIdentityDocument {
    DefinitionIdentityDocument {
        family: family.into(),
        key: key.into(),
        revision: REVISION.into(),
    }
}

fn document_ref(family: &str, key: &str) -> DefinitionReferenceDocument {
    DefinitionReferenceDocument {
        family: family.into(),
        key: key.into(),
        revision: REVISION.into(),
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: REVISION.into(),
    }
}

const CREATURE: &str = "oteryn:creature.dragon";
const PRESENTATION: &str = "oteryn:presentation.creature.dragon";
const BEHAVIOR: &str = "oteryn:behavior.creature.dragon";
const LOOT: &str = "oteryn:loot.creature.dragon";
const MELEE: &str = "oteryn:ability.creature.dragon.attack-1";
const WAVE: &str = "oteryn:ability.creature.dragon.attack-2";
const MELEE_EFFECT: &str = "oteryn:effect.creature.dragon.attack-1";
const MELEE_FORMULA: &str = "oteryn:formula.creature.dragon.attack-1-1";
const SPEED_FORMULA: &str = "oteryn:formula.creature.dragon.attack-2-1";
const CORPSE: &str = "oteryn:item.tibia.i6136";
const GOLD: &str = "oteryn:item.tibia.i3147";

fn item(key: &str) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Item {
        identity: identity("Item", key),
        client_projection: ProjectionDocument::ClientSafe,
        materializable: false,
        stack_class: ItemStackDocument::Unknown,
        semantics: ReferenceItemSemantics::default(),
    }
}

fn records() -> Vec<ProjectReferenceRecord> {
    vec![
        ProjectReferenceRecord::Generic {
            identity: identity("Presentation", PRESENTATION),
            client_projection: ProjectionDocument::ClientSafe,
        },
        ProjectReferenceRecord::Generic {
            identity: identity("Behavior", BEHAVIOR),
            client_projection: ProjectionDocument::ServerOnly,
        },
        item(CORPSE),
        item(GOLD),
        ProjectReferenceRecord::Loot {
            identity: identity("Loot", LOOT),
            algorithm: LootAlgorithmDocument::IndependentBernoulliPpm,
            entries: vec![
                LootEntryDocument {
                    item: document_ref("Item", GOLD),
                    min_count: 1,
                    max_count: 45,
                    probability_ppm: Some(1_000_000),
                },
                LootEntryDocument {
                    item: document_ref("Item", GOLD),
                    min_count: 1,
                    max_count: 43,
                    probability_ppm: Some(975_000),
                },
            ],
        },
        ProjectReferenceRecord::Formula {
            identity: identity("Formula", MELEE_FORMULA),
        },
        ProjectReferenceRecord::Formula {
            identity: identity("Formula", SPEED_FORMULA),
        },
        ProjectReferenceRecord::Effect {
            identity: identity("Effect", MELEE_EFFECT),
            client_projection: ProjectionDocument::ServerOnly,
            effect_family: EffectFamilyDocument::Damage,
            formula: document_ref("Formula", MELEE_FORMULA),
        },
        ProjectReferenceRecord::Ability {
            identity: identity("Ability", MELEE),
            effects: vec![document_ref("Effect", MELEE_EFFECT)],
        },
        // The only effect of this ability is a condition: no executable effect yet (§4).
        ProjectReferenceRecord::Ability {
            identity: identity("Ability", WAVE),
            effects: vec![],
        },
        ProjectReferenceRecord::Creature {
            identity: identity("Creature", CREATURE),
            client_projection: ProjectionDocument::ClientSafe,
            presentation: document_ref("Presentation", PRESENTATION),
            behavior: document_ref("Behavior", BEHAVIOR),
            loot: Some(document_ref("Loot", LOOT)),
        },
    ]
}

fn creature_profile() -> ProjectV2AuthoringProfile {
    ProjectV2AuthoringProfile {
        target: reference(ProjectV2Family::Creature, CREATURE),
        data: ProjectV2AuthoringProfileData::Creature(ProjectV2CreatureAuthoring {
            health: Some(1_000),
            experience: Some(700),
            speed: Some(86),
            armor: Some(25),
            mitigation: Some(ProjectV2ExactRatio {
                numerator: 39,
                denominator: 25,
            }),
            resistances: vec![ProjectV2Resistance {
                damage_type: "fire".into(),
                percent: ProjectV2ExactRatio {
                    numerator: 100,
                    denominator: 1,
                },
            }],
            immunities: vec!["fire".into()],
            abilities: vec![
                reference(ProjectV2Family::Ability, MELEE),
                reference(ProjectV2Family::Ability, WAVE),
            ],
            bestiary: Some(ProjectV2BestiaryProfile {
                difficulty: "medium".into(),
                occurrence: Some("common".into()),
                kill_thresholds: vec![50, 500, 1_000],
                charm_points: 25,
            }),
            details: Some(Box::new(ProjectV2CreatureDetails {
                display_name: "Dragon".into(),
                article: Some("a".into()),
                plural: Some("dragons".into()),
                inspection: "You see a dragon.".into(),
                defense: 30,
                critical_chance_ppm: 0,
                condition_immunities: vec!["invisible".into(), "paralyze".into()],
                flags: ProjectV2CreatureFlags {
                    attackable: true,
                    illusionable: true,
                    health_hidden: false,
                },
                summoning: ProjectV2CreatureSummoning {
                    summonable: false,
                    convinceable: false,
                    mana_cost: None,
                    is_familiar: false,
                },
                system_eligibility: ProjectV2SystemEligibility {
                    prey: true,
                    exclusive_prey: false,
                    forge: true,
                    reward_boss: false,
                },
                spawn_eligibility: ProjectV2SpawnEligibility {
                    period: ProjectV2SpawnPeriod::All,
                    ignore_period_underground: false,
                    blocked_by_nearby_players: false,
                },
                bestiary: Some(ProjectV2BestiaryDetails {
                    class: "Dragon".into(),
                    taxonomy: "dragon".into(),
                    stars: Some(3),
                    locations: Some("Darashia Dragon Lair.".into()),
                    notes: None,
                }),
                bosstiary: None,
                corpse_item: Some(reference(ProjectV2Family::Item, CORPSE)),
                soul_core_item: None,
                death_residue: Some(ProjectV2DeathResidue {
                    item: reference(ProjectV2Family::Item, CORPSE),
                    fluid_type: Some("blood".into()),
                }),
                encyclopedia_document: None,
                damage_reflection: vec![],
                healing_from_damage: vec![],
                reward_encounter: None,
            })),
            ..ProjectV2CreatureAuthoring::default()
        }),
    }
}

fn schedule(key: &str, chance_ppm: u32) -> ProjectV2AbilitySchedule {
    ProjectV2AbilitySchedule {
        ability: reference(ProjectV2Family::Ability, key),
        interval_ms: 2_000,
        chance_ppm,
        magnitude: None,
        range_tiles: None,
    }
}

fn profiles() -> Vec<ProjectV2AuthoringProfile> {
    vec![
        creature_profile(),
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Behavior, BEHAVIOR),
            data: ProjectV2AuthoringProfileData::Behavior(ProjectV2BehaviorAuthoring {
                movement: ProjectV2Movement {
                    can_walk: true,
                    pass_through: false,
                    pushable: false,
                    push_items: true,
                    push_creatures: true,
                    walks_on_energy: true,
                    walks_on_fire: true,
                    walks_on_poison: true,
                    wander: None,
                },
                targeting: ProjectV2Targeting {
                    hostile: true,
                    can_target: true,
                    sense_invisible: true,
                    target_distance_tiles: 1,
                    static_attack_chance_ppm: 800_000,
                    flee_health: 300,
                    change_target: Some(ProjectV2ChangeTarget {
                        interval_ms: 4_000,
                        chance_ppm: 100_000,
                    }),
                    strategy_weights: Some(ProjectV2StrategyWeights {
                        nearest: 70,
                        damage: 10,
                        health: 10,
                        random: 10,
                    }),
                },
                attacks: vec![schedule(MELEE, 1_000_000), schedule(WAVE, 150_000)],
                defenses: vec![],
                voices: Some(ProjectV2Voices {
                    interval_ms: 5_000,
                    chance_ppm: 100_000,
                    entries: vec![ProjectV2Voice {
                        text: "GROOAAARRR".into(),
                        mode: ProjectV2SpeechMode::Yell,
                    }],
                }),
                summons: None,
                periodic_audio: None,
                faction: None,
                event_bindings: vec![],
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Presentation, PRESENTATION),
            data: ProjectV2AuthoringProfileData::Presentation(ProjectV2PresentationAuthoring {
                asset_binding: Some("canary.appearance:outfit/34".into()),
                selection: None,
                palette_bindings: vec![],
                attachment_bindings: vec![],
                visual_effect_bindings: vec![],
                light_level: 0,
                light_color_binding: None,
                audio_bindings: vec![],
                variant_label: None,
                status_marker: None,
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Ability, MELEE),
            data: ProjectV2AuthoringProfileData::Ability(ProjectV2AbilityAuthoring {
                details: Some(Box::new(ProjectV2AbilityDetails {
                    kind: ProjectV2AbilityKind::Melee,
                    range_tiles: 1,
                    needs_target: true,
                    needs_direction: false,
                    area: None,
                    effects: vec![ProjectV2AbilityEffect::Executable(reference(
                        ProjectV2Family::Effect,
                        MELEE_EFFECT,
                    ))],
                    variants: vec![],
                    encounter: None,
                    path_requirement: None,
                    windup: None,
                    chain: None,
                    cast_cue: None,
                    impact_cue: None,
                })),
                ..ProjectV2AbilityAuthoring::default()
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Ability, WAVE),
            data: ProjectV2AuthoringProfileData::Ability(ProjectV2AbilityAuthoring {
                details: Some(Box::new(ProjectV2AbilityDetails {
                    kind: ProjectV2AbilityKind::Spell,
                    range_tiles: 0,
                    needs_target: false,
                    needs_direction: true,
                    area: Some(ProjectV2AbilityArea::Matrix {
                        north: vec!["xxx".into(), ".x.".into(), ".C.".into()],
                        diagonal: vec![],
                    }),
                    effects: vec![ProjectV2AbilityEffect::Inline(Box::new(
                        ProjectV2InlineEffect {
                            key: "oteryn:effect.creature.dragon.attack-2".into(),
                            operation: ProjectV2InlineEffectOperation::Condition {
                                duration_ms: Some(20_000),
                                condition: ProjectV2Condition {
                                    condition_type: "paralyze".into(),
                                    lifetime: ProjectV2ConditionLifetime::FixedDuration,
                                    damage_over_time: None,
                                    speed_formula: Some(reference(
                                        ProjectV2Family::Formula,
                                        SPEED_FORMULA,
                                    )),
                                    attribute_modifiers: vec![],
                                    light: None,
                                    regeneration: None,
                                    buff_spell: None,
                                },
                            },
                            presentation: Some(ProjectV2EffectPresentation {
                                impact_asset_binding: Some("canary.appearance:effect/red".into()),
                                projectile_asset_binding: None,
                                path_asset_binding: None,
                            }),
                        },
                    ))],
                    variants: vec![],
                    encounter: None,
                    path_requirement: None,
                    windup: None,
                    chain: None,
                    cast_cue: None,
                    impact_cue: None,
                })),
                ..ProjectV2AbilityAuthoring::default()
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Effect, MELEE_EFFECT),
            data: ProjectV2AuthoringProfileData::Effect(ProjectV2EffectAuthoring {
                damage_type: "physical".into(),
                mitigated_by: vec![ProjectV2Mitigation::Armor, ProjectV2Mitigation::Shield],
                affects: None,
                presentation: None,
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Formula, MELEE_FORMULA),
            data: ProjectV2AuthoringProfileData::Formula(ProjectV2FormulaAuthoring::Range {
                minimum: 0,
                maximum: 120,
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Formula, SPEED_FORMULA),
            data: ProjectV2AuthoringProfileData::Formula(
                ProjectV2FormulaAuthoring::SpeedModifier {
                    minimum_multiplier: ProjectV2ExactRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    minimum_offset: -600,
                    maximum_multiplier: ProjectV2ExactRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    maximum_offset: -400,
                },
            ),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Loot, LOOT),
            data: ProjectV2AuthoringProfileData::Loot(ProjectV2LootAuthoring {
                entries: vec![
                    ProjectV2LootEntryDetails {
                        skip_later_same_item_after_success: true,
                    },
                    ProjectV2LootEntryDetails::default(),
                ],
            }),
        },
    ]
}

fn draft() -> ProjectV2Draft {
    ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "project-r1".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "license:project-owned-v1".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: records(),
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            authoring_profiles: profiles(),
            ..ProjectV2State::default()
        },
    }
}

fn admit(draft: ProjectV2Draft) -> Result<WorldProject, String> {
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, limits())
        .map_err(|error| format!("{error:?}"))?;
    ProjectSnapshot::new(documents.documents().clone(), limits())
        .map_err(|error| format!("{error:?}"))?
        .parse(limits())
        .map_err(|error| format!("{error:?}"))
}

fn profile_mut<'a>(
    draft: &'a mut ProjectV2Draft,
    key: &str,
) -> &'a mut ProjectV2AuthoringProfileData {
    &mut draft
        .state
        .authoring_profiles
        .iter_mut()
        .find(|profile| profile.target.key == key)
        .expect("profile")
        .data
}

#[test]
fn admitted_monster_round_trips_and_lowers_only_its_reference_records() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(draft(), limits()).expect("monster documents");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit monster")
        .parse(limits())
        .expect("parse monster");
    let state = parsed.v2().expect("v2 state");
    assert_eq!(state.authoring_profiles.len(), profiles().len());
    let mut expected = profiles();
    expected.sort_by(|left, right| left.target.cmp(&right.target));
    assert_eq!(state.authoring_profiles, expected);
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        documents.documents()
    );

    let reference = parsed
        .lower_reference_source()
        .expect("reference projection stays executable-only");
    assert_eq!(reference.definitions.len(), records().len());
}

#[test]
fn initial_health_split_round_trips_without_changing_reference_records() {
    let baseline = admit(draft())
        .expect("legacy profile")
        .lower_reference_source()
        .expect("legacy reference");
    for initial in [1, 1_000] {
        let mut candidate = draft();
        let creature = match profile_mut(&mut candidate, CREATURE) {
            ProjectV2AuthoringProfileData::Creature(creature) => Some(creature),
            _ => None,
        }
        .expect("creature profile");
        creature.initial_health = Some(initial);
        let documents = CanonicalProjectDocuments::from_v2_draft(candidate, limits())
            .expect("initial health documents");
        let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
            .expect("initial health snapshot")
            .parse(limits())
            .expect("initial health admitted");
        let profile = parsed
            .v2()
            .expect("v2")
            .authoring_profiles
            .iter()
            .find(|profile| profile.target.key == CREATURE)
            .expect("creature profile");
        let creature = match &profile.data {
            ProjectV2AuthoringProfileData::Creature(creature) => Some(creature),
            _ => None,
        }
        .expect("creature profile");
        assert_eq!(creature.health, Some(1_000));
        assert_eq!(creature.initial_health, Some(initial));
        assert_eq!(
            parsed
                .canonical_documents(limits())
                .expect("rewrite")
                .documents(),
            documents.documents()
        );
        assert_eq!(
            parsed
                .lower_reference_source()
                .expect("reference projection")
                .definitions,
            baseline.definitions
        );
    }
    let legacy = serde_json::to_value(creature_profile()).expect("legacy serialization");
    assert!(legacy["data"]["profile"].get("initial_health").is_none());
}

#[test]
fn invalid_initial_health_bounds_and_missing_maximum_fail_closed() {
    for (maximum, initial) in [(Some(1_000), 0), (Some(1_000), 1_001), (None, 1)] {
        let mut candidate = draft();
        let creature = match profile_mut(&mut candidate, CREATURE) {
            ProjectV2AuthoringProfileData::Creature(creature) => Some(creature),
            _ => None,
        }
        .expect("creature profile");
        creature.health = maximum;
        creature.initial_health = Some(initial);
        assert!(
            admit(candidate)
                .expect_err("invalid initial health")
                .contains("initial health requires")
        );
    }
    for initial in [json!(-1), json!(true), json!(1.5), json!("1")] {
        let mut profile = serde_json::to_value(creature_profile()).expect("profile value");
        profile["data"]["profile"]["initial_health"] = initial;
        assert!(serde_json::from_value::<ProjectV2AuthoringProfile>(profile).is_err());
    }
}

#[test]
fn authored_sequences_keep_their_order_and_sets_are_canonicalized() {
    let mut changed = draft();
    if let ProjectV2AuthoringProfileData::Creature(creature) = profile_mut(&mut changed, CREATURE) {
        creature
            .details
            .as_mut()
            .expect("details")
            .condition_immunities
            .reverse();
    }
    if let ProjectV2AuthoringProfileData::Behavior(behavior) = profile_mut(&mut changed, BEHAVIOR) {
        behavior.attacks.reverse();
    }
    if let ProjectV2AuthoringProfileData::Presentation(presentation) =
        profile_mut(&mut changed, PRESENTATION)
    {
        presentation.attachment_bindings = ["addon-2", "addon-1"]
            .map(|addon| ProjectV2SlotBinding {
                slot: ProjectV2AttachmentSlot::Addon,
                asset_binding: format!("canary.appearance:outfit/34/{addon}"),
            })
            .to_vec();
    }
    let parsed = admit(changed).expect("admit reordered monster");
    let state = parsed.v2().expect("v2 state");
    let find = |key: &str| {
        state
            .authoring_profiles
            .iter()
            .find(|profile| profile.target.key == key)
            .expect("profile")
            .data
            .clone()
    };
    let immunities = match find(CREATURE) {
        ProjectV2AuthoringProfileData::Creature(creature) => creature
            .details
            .map(|details| details.condition_immunities)
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    assert_eq!(
        immunities,
        vec!["invisible".to_owned(), "paralyze".to_owned()]
    );
    let first_attack = match find(BEHAVIOR) {
        ProjectV2AuthoringProfileData::Behavior(behavior) => behavior
            .attacks
            .first()
            .map(|schedule| schedule.ability.key.clone()),
        _ => None,
    };
    assert_eq!(
        first_attack.as_deref(),
        Some(WAVE),
        "schedule order is authored order"
    );
    let addons = match find(PRESENTATION) {
        ProjectV2AuthoringProfileData::Presentation(presentation) => presentation
            .attachment_bindings
            .into_iter()
            .map(|binding| binding.asset_binding)
            .collect(),
        _ => Vec::new(),
    };
    assert_eq!(
        addons,
        vec![
            "canary.appearance:outfit/34/addon-1".to_owned(),
            "canary.appearance:outfit/34/addon-2".to_owned()
        ],
        "both addons share the addon slot and sort by binding"
    );
}

#[test]
fn each_broken_invariant_is_rejected() {
    type Mutation = fn(&mut ProjectV2Draft);
    let cases: [(&str, &str, Mutation); 13] = [
        (
            "two palette bindings for one slot",
            "v2 presentation slots are not sorted and unique",
            |draft| {
                if let ProjectV2AuthoringProfileData::Presentation(presentation) =
                    profile_mut(draft, PRESENTATION)
                {
                    presentation.palette_bindings = ["canary.palette:1", "canary.palette:2"]
                        .map(|binding| ProjectV2SlotBinding {
                            slot: ProjectV2PaletteSlot::Head,
                            asset_binding: binding.to_owned(),
                        })
                        .to_vec();
                }
            },
        ),
        (
            "chance above one million ppm",
            "v2 schedule chance exceeds 100%",
            |draft| {
                if let ProjectV2AuthoringProfileData::Behavior(behavior) =
                    profile_mut(draft, BEHAVIOR)
                {
                    behavior.attacks[1].chance_ppm = 1_000_001;
                }
            },
        ),
        (
            "schedule names an unknown Ability",
            "unresolved v2 typed definition reference",
            |draft| {
                if let ProjectV2AuthoringProfileData::Behavior(behavior) =
                    profile_mut(draft, BEHAVIOR)
                {
                    behavior.attacks[1].ability.key =
                        "oteryn:ability.creature.dragon.attack-9".into();
                }
            },
        ),
        (
            "zero schedule interval",
            "v2 schedule interval must be positive",
            |draft| {
                if let ProjectV2AuthoringProfileData::Behavior(behavior) =
                    profile_mut(draft, BEHAVIOR)
                {
                    behavior.attacks[0].interval_ms = 0;
                }
            },
        ),
        (
            "invisible appearance with an asset",
            "v2 invisible appearance forbids appearance bindings",
            |draft| {
                if let ProjectV2AuthoringProfileData::Presentation(presentation) =
                    profile_mut(draft, PRESENTATION)
                {
                    presentation.selection = Some(ProjectV2AppearanceSelection::Invisible);
                }
            },
        ),
        (
            "visible appearance without an asset",
            "v2 visible appearance requires an asset binding",
            |draft| {
                if let ProjectV2AuthoringProfileData::Presentation(presentation) =
                    profile_mut(draft, PRESENTATION)
                {
                    presentation.asset_binding = None;
                }
            },
        ),
        (
            "profile on the wrong family",
            "v2 authoring profile target family mismatch",
            |draft| {
                let profile = draft
                    .state
                    .authoring_profiles
                    .iter_mut()
                    .find(|profile| profile.target.key == BEHAVIOR)
                    .expect("behavior");
                profile.target = reference(ProjectV2Family::Presentation, PRESENTATION);
                draft.state.authoring_profiles.retain(|profile| {
                    !matches!(profile.data, ProjectV2AuthoringProfileData::Presentation(_))
                });
            },
        ),
        (
            "loot details misaligned with the Loot record",
            "v2 Loot details must align with the entries of their Loot record",
            |draft| {
                if let ProjectV2AuthoringProfileData::Loot(loot) = profile_mut(draft, LOOT) {
                    loot.entries.pop();
                }
            },
        ),
        (
            "fixed-duration condition without duration",
            "v2 fixed-duration condition needs a positive duration",
            |draft| {
                if let ProjectV2AuthoringProfileData::Ability(ability) = profile_mut(draft, WAVE)
                    && let Some(ProjectV2AbilityEffect::Inline(effect)) = ability
                        .details
                        .as_mut()
                        .and_then(|details| details.effects.first_mut())
                    && let ProjectV2InlineEffectOperation::Condition { duration_ms, .. } =
                        &mut effect.operation
                {
                    *duration_ms = None;
                }
            },
        ),
        (
            "area matrix without a centre",
            "v2 area matrix must be rectangular with exactly one centre",
            |draft| {
                if let ProjectV2AuthoringProfileData::Ability(ability) = profile_mut(draft, WAVE) {
                    ability.details.as_mut().expect("details").area =
                        Some(ProjectV2AbilityArea::Matrix {
                            north: vec!["xxx".into(), ".x.".into()],
                            diagonal: vec![],
                        });
                }
            },
        ),
        (
            "executable effect that is not an Effect",
            "v2 Ability executable effect family mismatch",
            |draft| {
                if let ProjectV2AuthoringProfileData::Ability(ability) = profile_mut(draft, MELEE) {
                    ability.details.as_mut().expect("details").effects =
                        vec![ProjectV2AbilityEffect::Executable(reference(
                            ProjectV2Family::Formula,
                            MELEE_FORMULA,
                        ))];
                }
            },
        ),
        (
            "summon mana cost without being summonable",
            "v2 Creature summon mana cost must be present exactly when",
            |draft| {
                if let ProjectV2AuthoringProfileData::Creature(creature) =
                    profile_mut(draft, CREATURE)
                {
                    creature
                        .details
                        .as_mut()
                        .expect("details")
                        .summoning
                        .mana_cost = Some(10);
                }
            },
        ),
        (
            "inverted formula range",
            "v2 formula range is inverted",
            |draft| {
                *profile_mut(draft, MELEE_FORMULA) =
                    ProjectV2AuthoringProfileData::Formula(ProjectV2FormulaAuthoring::Range {
                        minimum: 121,
                        maximum: 120,
                    });
            },
        ),
    ];
    for (name, message, mutation) in cases {
        let mut changed = draft();
        mutation(&mut changed);
        let error = admit(changed).expect_err(name);
        assert!(
            error.contains(message),
            "{name}: rejected for another reason: {error}"
        );
    }
}

#[test]
fn unknown_profile_fields_fail_closed() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(draft(), limits()).expect("monster documents");
    let declarations: Value =
        serde_json::from_slice(&documents.documents()["definitions/declarations.json"])
            .expect("declarations");
    let mut behavior = declarations["authoring_profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|profile| profile["target"]["key"] == BEHAVIOR)
        .expect("behavior")["data"]
        .clone();
    assert!(serde_json::from_value::<ProjectV2AuthoringProfileData>(behavior.clone()).is_ok());
    behavior["profile"]["movement"]["teleports"] = json!(true);
    assert!(
        serde_json::from_value::<ProjectV2AuthoringProfileData>(behavior).is_err(),
        "unknown movement field admitted"
    );
}

#[test]
fn players_only_chain_round_trips_and_other_filters_fail_closed() {
    let chain: ProjectV2Chain = serde_json::from_value(json!({
        "max_targets": 2,
        "range_tiles": 3,
        "backtracking": false,
        "target_filter": "players",
    }))
    .expect("players-only chain");
    assert_eq!(
        chain.target_filter,
        Some(ProjectV2ChainTargetFilter::Players)
    );
    assert_eq!(
        serde_json::to_value(&chain).expect("chain")["target_filter"],
        json!("players")
    );
    let unfiltered: ProjectV2Chain = serde_json::from_value(json!({
        "max_targets": 2,
        "range_tiles": 3,
        "backtracking": false,
    }))
    .expect("unfiltered chain");
    assert_eq!(unfiltered.target_filter, None);
    assert!(
        serde_json::to_value(&unfiltered)
            .expect("chain")
            .get("target_filter")
            .is_none()
    );
    assert!(
        serde_json::from_value::<ProjectV2Chain>(json!({
            "max_targets": 2,
            "range_tiles": 3,
            "backtracking": false,
            "target_filter": "monsters",
        }))
        .is_err(),
        "unknown chain target filter admitted"
    );
}

#[test]
fn players_only_affects_round_trips_and_needs_no_creatures() {
    let affects: ProjectV2EffectAffects = serde_json::from_value(json!({
        "kind": "Players",
        "top_creature_only": true,
        "excludes_caster_name": false,
        "includes_caster": false,
    }))
    .expect("players-only affects");
    assert_eq!(affects.kind, ProjectV2AffectsKind::Players);
    assert!(affects.creatures.is_empty());
    assert_eq!(
        serde_json::to_value(&affects).expect("affects")["kind"],
        json!("Players")
    );
}

#[test]
fn windup_admits_only_on_a_single_target_ability() {
    fn with_windup(key: &str, delay_ms: u32) -> Result<WorldProject, String> {
        let mut draft = draft();
        if let ProjectV2AuthoringProfileData::Ability(ability) = profile_mut(&mut draft, key) {
            ability.details.as_mut().expect("details").windup = Some(ProjectV2Windup {
                delay_ms,
                caster_asset_binding: "canary.appearance:effect/ghost_smoke".into(),
            });
        }
        admit(draft)
    }
    with_windup(MELEE, 2_000).expect("single-target windup admits");
    for (key, delay_ms) in [(MELEE, 0), (WAVE, 2_000)] {
        let error = with_windup(key, delay_ms).expect_err("windup must fail closed");
        assert!(error.contains("v2 Ability windup"), "{error}");
    }
}

fn with_condition(value: Value) -> ProjectV2Draft {
    let mut changed = draft();
    let mut found = false;
    if let ProjectV2AuthoringProfileData::Ability(ability) = profile_mut(&mut changed, WAVE) {
        for effect in &mut ability.details.as_mut().expect("details").effects {
            if let ProjectV2AbilityEffect::Inline(effect) = effect
                && let ProjectV2InlineEffectOperation::Condition {
                    condition,
                    duration_ms,
                } = &mut effect.operation
            {
                *condition = serde_json::from_value(value.clone()).expect("typed condition");
                *duration_ms = if condition.lifetime == ProjectV2ConditionLifetime::DamageSchedule {
                    None
                } else {
                    Some(120_000)
                };
                found = true;
                break;
            }
        }
    }
    assert!(found, "condition fixture missing");
    changed
}

#[test]
fn s17_condition_payloads_round_trip_through_validated_admission() {
    for value in [
        json!({"condition_type": "light", "lifetime": "FixedDuration",
               "light": {"level": 9, "color": 215}, "buff_spell": false}),
        json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
               "regeneration": {"health_gain": 20, "health_interval_ms": 2000}, "buff_spell": true}),
        json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
               "regeneration": {"mana_gain": 5, "mana_interval_ms": 3000}}),
        json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
               "regeneration": {"health_gain": 20, "health_interval_ms": 2000,
                                "mana_gain": 5, "mana_interval_ms": 3000}}),
        json!({"condition_type": "attributes", "lifetime": "FixedDuration", "buff_spell": true,
               "attribute_modifiers": [{"attribute": "shield", "mode": "Add", "value": 3}]}),
    ] {
        let changed = with_condition(value);
        let mut expected = changed.state.authoring_profiles.clone();
        expected.sort_by(|left, right| left.target.cmp(&right.target));
        let admitted = admit(changed).expect("S17 declaration admits");
        assert_eq!(admitted.v2().expect("v2").authoring_profiles, expected);
    }
}

#[test]
fn s17_invalid_type_pairing_bounds_and_damage_schedule_fail_closed() {
    for (value, message) in [
        (
            json!({"condition_type": "light", "lifetime": "FixedDuration"}),
            "v2 light condition",
        ),
        (
            json!({"condition_type": "light", "lifetime": "FixedDuration",
                "light": {"level": 0, "color": 215}}),
            "v2 light condition",
        ),
        (
            json!({"condition_type": "invisible", "lifetime": "FixedDuration",
                "light": {"level": 9, "color": 215}}),
            "v2 light condition",
        ),
        (
            json!({"condition_type": "regeneration", "lifetime": "FixedDuration"}),
            "v2 regeneration condition",
        ),
        (
            json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
                "regeneration": {}}),
            "complete positive gain/interval pair",
        ),
        (
            json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
                "regeneration": {"health_gain": 20}}),
            "complete positive gain/interval pair",
        ),
        (
            json!({"condition_type": "regeneration", "lifetime": "FixedDuration",
                "regeneration": {"mana_gain": 0, "mana_interval_ms": 3000}}),
            "complete positive gain/interval pair",
        ),
        (
            json!({"condition_type": "invisible", "lifetime": "FixedDuration",
                "regeneration": {"mana_gain": 5, "mana_interval_ms": 3000}}),
            "v2 regeneration condition",
        ),
        (
            json!({"condition_type": "fire", "lifetime": "DamageSchedule", "buff_spell": false,
                "damage_over_time": {"tick_profile": "Fixed", "first_tick": "AfterInterval",
                                     "ticks": [{"count": 1, "interval_ms": 1000, "amount": 10}]}}),
            "v2 damage-schedule condition",
        ),
    ] {
        let error = admit(with_condition(value)).expect_err("invalid S17 payload");
        assert!(error.contains(message), "wrong rejection: {error}");
    }
    for light in [
        json!({"level": 256, "color": 215}),
        json!({"level": 9, "color": -1}),
        json!({"level": 9, "color": 215, "unknown": 1}),
    ] {
        assert!(serde_json::from_value::<ProjectV2ConditionLight>(light).is_err());
    }
}

#[test]
fn authored_bestiary_notes_round_trip_and_invalid_source_text_is_rejected() {
    fn with_notes(notes: &str) -> ProjectV2Draft {
        let mut changed = draft();
        if let ProjectV2AuthoringProfileData::Creature(creature) =
            profile_mut(&mut changed, CREATURE)
        {
            creature
                .details
                .as_mut()
                .expect("details")
                .bestiary
                .as_mut()
                .expect("bestiary")
                .notes = Some(notes.into());
        }
        changed
    }
    let changed = with_notes("Original Oteryn narrative.\nSecond paragraph.");
    let mut expected = changed.state.authoring_profiles.clone();
    expected.sort_by(|left, right| left.target.cmp(&right.target));
    let admitted = admit(changed).expect("authored notes admit");
    assert_eq!(admitted.v2().expect("v2").authoring_profiles, expected);
    for notes in ["", " surrounding spaces "] {
        let error = admit(with_notes(notes)).expect_err("invalid notes");
        assert!(error.contains("v2 Bestiary notes"), "{error}");
    }
}

#[test]
fn accepted_creature_ai_content_limits_reject_first_excess_at_admission() {
    // CREATURE-AI-0 §10 RL07/RL08 apply at content admission, not only AI preparation.
    for (field, maximum) in [
        ("attacks", 16),
        ("defenses", 8),
        ("summon_entries", 8),
        ("max_summons", 16),
        ("summon_count", 16),
    ] {
        for value in [maximum, maximum + 1] {
            let mut candidate = draft();
            if let ProjectV2AuthoringProfileData::Behavior(behavior) =
                profile_mut(&mut candidate, BEHAVIOR)
            {
                match field {
                    "attacks" => behavior.attacks = vec![schedule(WAVE, 1_000_000); value],
                    "defenses" => behavior.defenses = vec![schedule(WAVE, 1_000_000); value],
                    _ => {
                        behavior.summons = Some(ProjectV2Summons {
                            max_summons: if field == "max_summons" {
                                u32::try_from(value).expect("bounded fixture")
                            } else {
                                16
                            },
                            entries: vec![
                                ProjectV2SummonEntry {
                                    creature: reference(ProjectV2Family::Creature, CREATURE),
                                    interval_ms: 1_000,
                                    chance_ppm: 1_000_000,
                                    count: if field == "summon_count" {
                                        u32::try_from(value).expect("bounded fixture")
                                    } else {
                                        16
                                    },
                                };
                                if field == "summon_entries" { value } else { 1 }
                            ],
                        });
                    }
                }
            }
            let result = admit(candidate);
            assert_eq!(
                result.is_ok(),
                value == maximum,
                "{field}={value}: {:?}",
                result.as_ref().err()
            );
        }
    }
}
