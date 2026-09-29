#![allow(clippy::expect_used)]

//! OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 E1-E3: an encounter admitted as a typed
//! declarative profile, and the creature it covers bound to it.

use oteryn_game_server::content::*;
use serde_json::{Value, json};

const REVISION: &str = "definition-r1";
const ENCOUNTER: &str = "oteryn:encounter.vortex";
const BOSS: &str = "oteryn:creature.the_hunger";
const ADD: &str = "oteryn:creature.greed";
const PRESENTATION: &str = "oteryn:presentation.creature.shared";
const BEHAVIOR: &str = "oteryn:behavior.creature.shared";
const SUMMON: &str = "oteryn:ability.spell.hunger_summon";
const VORTEX: &str = "oteryn:item.registry.i00023469";
const OPEN_VORTEX: &str = "oteryn:item.registry.i00023470";

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

fn item(key: &str) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Item {
        identity: identity("Item", key),
        client_projection: ProjectionDocument::ClientSafe,
        materializable: false,
        stack_class: ItemStackDocument::Unknown,
        semantics: ReferenceItemSemantics::default(),
    }
}

fn creature(key: &str) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Creature {
        identity: identity("Creature", key),
        client_projection: ProjectionDocument::ClientSafe,
        presentation: document_ref("Presentation", PRESENTATION),
        behavior: document_ref("Behavior", BEHAVIOR),
        loot: None,
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
        item(VORTEX),
        item(OPEN_VORTEX),
        ProjectReferenceRecord::Ability {
            identity: identity("Ability", SUMMON),
            effects: vec![],
        },
        creature(BOSS),
        creature(ADD),
    ]
}

fn creature_ref(key: &str) -> Value {
    json!({"family": "Creature", "key": key, "revision": REVISION})
}

fn item_ref(key: &str) -> Value {
    json!({"family": "Item", "key": key, "revision": REVISION})
}

/// The Heart of Destruction vortex rules of the `world_devourer` encounter, in the v2 shape.
fn details_json() -> Value {
    json!({
        "display_name": "Heart of Destruction: the vortex",
        "participants": [
            {"role": "the_hunger", "creatures": [creature_ref(BOSS)]},
            {"role": "greed", "creatures": [creature_ref(ADD)]}
        ],
        "covers": [creature_ref(BOSS)],
        "anchors": [
            {"key": "hunger_vortex", "description": "Canary (32244, 31371, 14).",
             "location": {"kind": "point", "x": 32244, "y": 31371, "floor": 14}},
            {"key": "arena", "description": "Every tile from Canary (32200, 31340, 14) to (32280, 31400, 14).",
             "location": {"kind": "area", "boxes": [{"x": [32200, 32280], "y": [31340, 31400], "floor": 14}]}}
        ],
        "state": {
            "counters": [{"name": "hunger_summons", "initial": 0}],
            "flags": [{"name": "summon_delay", "initial": false}],
            "timers": [{"name": "summon_delay", "duration_ms": {"min": 15000, "max": 15000}, "repeat": false}]
        },
        "rules": [
            {"key": "the_hunger_summons_greed",
             "trigger": {"kind": "ability_cast", "role": "the_hunger",
                         "ability": {"family": "Ability", "key": SUMMON, "revision": REVISION}},
             "conditions": [
                 {"kind": "counter_compare", "counter": "hunger_summons", "op": "<", "value": 3},
                 {"kind": "flag", "flag": "summon_delay", "value": false},
                 {"kind": "has_condition", "role": "the_hunger", "conditions": ["poison", "bleeding"], "present": false}
             ],
             "actions": [
                 {"kind": "spawn", "creature": creature_ref(ADD), "role": "greed", "count": {"min": 1, "max": 1},
                  "at": {"kind": "relative", "x": 0, "y": -1}, "owner": "none", "health": {"kind": "full"}},
                 {"kind": "counter", "counter": "hunger_summons", "operation": "add", "value": 1},
                 {"kind": "timer", "timer": "summon_delay", "operation": "start"}
             ]},
            {"key": "the_hunger_opens_the_vortex",
             "trigger": {"kind": "stepped_on", "role": "the_hunger", "item": item_ref(VORTEX)},
             "actions": [
                 {"kind": "map_item", "operation": "transform", "item": item_ref(VORTEX), "into": item_ref(OPEN_VORTEX),
                  "anchor": "hunger_vortex"},
                 {"kind": "one_of", "branches": [
                     {"weight": 1, "actions": [{"kind": "say", "subject": {"kind": "role", "role": "the_hunger"},
                                                "text": "FEED ME!", "mode": "yell"}]},
                     {"weight": 3, "actions": [{"kind": "message", "players_in": "arena", "text": "The vortex opens."}]}
                 ]}
             ]},
            {"key": "greed_leaves",
             "trigger": {"kind": "stepped_on", "role": "greed", "item": item_ref(OPEN_VORTEX)},
             "actions": [
                 {"kind": "remove", "triggering": true},
                 {"kind": "counter", "counter": "hunger_summons", "operation": "add", "value": -1},
                 {"kind": "emit_outcome", "outcome": "vortex_fed", "credited": "players_in_anchor", "anchor": "arena"}
             ]}
        ],
        "outcomes": ["vortex_fed"]
    })
}

fn details() -> ProjectV2EncounterDetails {
    serde_json::from_value(details_json()).expect("encounter details")
}

fn profiles() -> Vec<ProjectV2AuthoringProfile> {
    vec![
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Encounter, ENCOUNTER),
            data: ProjectV2AuthoringProfileData::Encounter(ProjectV2EncounterAuthoring {
                encounter_type: ProjectV2EncounterType::Boss,
                scope: ProjectV2EncounterScope::Instance,
                areas: vec![],
                cooldown_seconds: None,
                repeatable: None,
                interactions: vec![],
                details: Some(Box::new(details())),
                fields: vec![],
            }),
        },
        // D45: the summon spell points to the encounter whose rule does the summon.
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Ability, SUMMON),
            data: ProjectV2AuthoringProfileData::Ability(ProjectV2AbilityAuthoring {
                details: Some(Box::new(ProjectV2AbilityDetails {
                    kind: ProjectV2AbilityKind::Spell,
                    range_tiles: 0,
                    needs_target: false,
                    needs_direction: false,
                    area: None,
                    effects: vec![],
                    variants: vec![],
                    encounter: Some(reference(ProjectV2Family::Encounter, ENCOUNTER)),
                    path_requirement: None,
                    chain: None,
                    cast_cue: None,
                    impact_cue: None,
                })),
                ..ProjectV2AbilityAuthoring::default()
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Creature, BOSS),
            data: ProjectV2AuthoringProfileData::Creature(ProjectV2CreatureAuthoring {
                health: Some(300_000),
                abilities: vec![reference(ProjectV2Family::Ability, SUMMON)],
                encounters: vec![reference(ProjectV2Family::Encounter, ENCOUNTER)],
                ..ProjectV2CreatureAuthoring::default()
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
            declarations: vec![ProjectV2Declaration::Encounter {
                identity: ProjectV2Identity {
                    key: ENCOUNTER.into(),
                    revision: REVISION.into(),
                },
                fields: vec![],
            }],
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

fn encounter_mut(draft: &mut ProjectV2Draft) -> &mut ProjectV2EncounterDetails {
    draft
        .state
        .authoring_profiles
        .iter_mut()
        .find_map(|profile| match &mut profile.data {
            ProjectV2AuthoringProfileData::Encounter(encounter) => encounter.details.as_deref_mut(),
            _ => None,
        })
        .expect("encounter details")
}

fn ability_mut(draft: &mut ProjectV2Draft) -> &mut ProjectV2AbilityDetails {
    draft
        .state
        .authoring_profiles
        .iter_mut()
        .find_map(|profile| match &mut profile.data {
            ProjectV2AuthoringProfileData::Ability(ability) => ability.details.as_deref_mut(),
            _ => None,
        })
        .expect("ability details")
}

#[test]
fn admitted_encounter_round_trips_and_stays_declarative() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(draft(), limits()).expect("encounter documents");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit encounter")
        .parse(limits())
        .expect("parse encounter");
    let state = parsed.v2().expect("v2 state");
    let mut expected = profiles();
    for profile in &mut expected {
        if let ProjectV2AuthoringProfileData::Encounter(encounter) = &mut profile.data {
            let details = encounter.details.as_mut().expect("details");
            // Sets are canonicalized: the has_condition list is sorted.
            if let ProjectV2EncounterCondition::HasCondition { conditions, .. } =
                &mut details.rules[0].conditions[2]
            {
                conditions.sort();
            }
        }
    }
    expected.sort_by(|left, right| left.target.cmp(&right.target));
    assert_eq!(state.authoring_profiles, expected);
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        documents.documents()
    );
    // Authored sequences keep their order.
    let encounter = state
        .authoring_profiles
        .iter()
        .find_map(|profile| match &profile.data {
            ProjectV2AuthoringProfileData::Encounter(encounter) => encounter.details.as_deref(),
            _ => None,
        })
        .expect("encounter");
    let keys: Vec<_> = encounter
        .rules
        .iter()
        .map(|rule| rule.key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "the_hunger_summons_greed",
            "the_hunger_opens_the_vortex",
            "greed_leaves"
        ]
    );
    let reference = parsed
        .lower_reference_source()
        .expect("reference projection stays executable-only");
    assert_eq!(reference.definitions.len(), records().len());
}

#[test]
fn each_broken_invariant_is_rejected() {
    type Mutation = fn(&mut ProjectV2Draft);
    let cases: [(&str, &str, Mutation); 26] = [
        (
            "an encounter-backed ability without its ability_cast rule",
            "v2 encounter-backed Ability has no ability_cast rule in its encounter",
            |draft| {
                encounter_mut(draft).rules[0].trigger = ProjectV2EncounterTrigger::CreatureDied {
                    role: "the_hunger".into(),
                };
            },
        ),
        (
            "an ability_cast rule on a role whose creatures do not own the ability",
            "v2 encounter ability_cast role has no creature that owns the ability",
            |draft| {
                if let ProjectV2EncounterTrigger::AbilityCast { role, .. } =
                    &mut encounter_mut(draft).rules[0].trigger
                {
                    *role = "greed".into();
                }
            },
        ),
        (
            "more counters than the evidence limit",
            "v2 encounter counters",
            |draft| {
                let state = &mut encounter_mut(draft).state;
                state
                    .counters
                    .extend((0..64).map(|n| ProjectV2EncounterCounter {
                        name: format!("extra_{n}"),
                        initial: 0,
                    }));
            },
        ),
        (
            "an encounter-backed ability with an area",
            "v2 encounter-backed Ability has no area or chain",
            |draft| {
                ability_mut(draft).area = Some(ProjectV2AbilityArea::Circle { radius_tiles: 2 });
            },
        ),
        (
            "a quest progress token outside the key grammar",
            "invalid v2 encounter domain token",
            |draft| {
                encounter_mut(draft).rules[0].conditions.push(
                    ProjectV2EncounterCondition::WorldState {
                        state: "X:a:b".into(),
                        op: ProjectV2CompareOp::Equal,
                        value: ProjectV2StateValue::Boolean(true),
                    },
                );
            },
        ),
        (
            "a per-player spawn owned by the death master",
            "v2 spawn_per_player owner is none or subject",
            |draft| {
                encounter_mut(draft).rules[0].actions[0] =
                    ProjectV2EncounterAction::SpawnPerPlayer {
                        players_in: "arena".into(),
                        by_base_vocation: ProjectV2SpawnByVocation {
                            knight: Some(reference(ProjectV2Family::Creature, ADD)),
                            ..ProjectV2SpawnByVocation::default()
                        },
                        at: ProjectV2EncounterPosition::ClosestFreeTile,
                        owner: ProjectV2SpawnOwner::DeathMaster,
                        health: ProjectV2EncounterHealth::Full,
                        counter: None,
                    };
            },
        ),
        (
            "a covered creature without its encounter binding",
            "v2 Creature encounters differ from the creatures their encounters cover",
            |draft| {
                for profile in &mut draft.state.authoring_profiles {
                    if let ProjectV2AuthoringProfileData::Creature(creature) = &mut profile.data {
                        creature.encounters.clear();
                    }
                }
            },
        ),
        (
            "an encounter-backed ability owner that its encounter does not cover",
            "v2 creature owning an encounter-backed Ability is not bound to its encounter",
            |draft| {
                encounter_mut(draft).covers.clear();
                for profile in &mut draft.state.authoring_profiles {
                    if let ProjectV2AuthoringProfileData::Creature(creature) = &mut profile.data {
                        creature.encounters.clear();
                    }
                }
            },
        ),
        (
            "an encounter covering a creature that is not a participant",
            "v2 encounter covers a creature that is not a participant",
            |draft| {
                let details = encounter_mut(draft);
                details
                    .participants
                    .retain(|participant| participant.role != "the_hunger");
                details.rules.clear();
                details.rules.push(ProjectV2EncounterRule {
                    key: "started".into(),
                    trigger: ProjectV2EncounterTrigger::EncounterStarted,
                    delay_ms: None,
                    conditions: vec![],
                    actions: vec![ProjectV2EncounterAction::Flag {
                        flag: "summon_delay".into(),
                        value: true,
                    }],
                });
            },
        ),
        (
            "an encounter profile without details",
            "v2 Encounter authoring requires details",
            |draft| {
                for profile in &mut draft.state.authoring_profiles {
                    match &mut profile.data {
                        ProjectV2AuthoringProfileData::Encounter(encounter) => {
                            encounter.details = None
                        }
                        ProjectV2AuthoringProfileData::Creature(creature) => {
                            creature.encounters.clear()
                        }
                        _ => {}
                    }
                }
            },
        ),
        (
            "an encounter-backed melee ability",
            "v2 encounter-backed Ability must be a spell",
            |draft| ability_mut(draft).kind = ProjectV2AbilityKind::Melee,
        ),
        (
            "an encounter-backed ability with effects",
            "v2 Ability details need exactly one of effects, variants and encounter",
            |draft| {
                ability_mut(draft).variants = vec![
                    reference(ProjectV2Family::Ability, SUMMON),
                    reference(ProjectV2Family::Ability, SUMMON),
                ];
            },
        ),
        (
            "unknown role",
            "v2 encounter names an unknown role",
            |draft| {
                encounter_mut(draft).rules[0].trigger = ProjectV2EncounterTrigger::CreatureDied {
                    role: "stranger".into(),
                };
            },
        ),
        (
            "a point where an area is needed",
            "v2 encounter needs an area anchor",
            |draft| {
                encounter_mut(draft).anchors[1].location = ProjectV2AnchorLocation::Point {
                    x: 1,
                    y: 1,
                    floor: 7,
                };
            },
        ),
        (
            "a box that starts after it ends",
            "v2 encounter box starts after it ends",
            |draft| {
                if let ProjectV2AnchorLocation::Area { boxes } =
                    &mut encounter_mut(draft).anchors[1].location
                {
                    boxes[0].x = [32280, 32200];
                }
            },
        ),
        (
            "a floor off the map",
            "v2 encounter anchor floor is outside 0..=15",
            |draft| {
                encounter_mut(draft).anchors[0].location = ProjectV2AnchorLocation::Point {
                    x: 1,
                    y: 1,
                    floor: 16,
                };
            },
        ),
        (
            "duplicate anchors",
            "v2 encounter anchors are not unique",
            |draft| {
                let details = encounter_mut(draft);
                details.anchors[1].key = details.anchors[0].key.clone();
            },
        ),
        (
            "unknown counter",
            "v2 encounter names an unknown counter",
            |draft| {
                encounter_mut(draft).state.counters[0].name = "other".into();
            },
        ),
        (
            "unknown outcome",
            "v2 encounter names an unknown outcome",
            |draft| {
                encounter_mut(draft).outcomes.clear();
            },
        ),
        (
            "remove with two targets",
            "v2 remove takes exactly one of role, all_in and triggering",
            |draft| {
                encounter_mut(draft).rules[2].actions[0] = ProjectV2EncounterAction::Remove {
                    role: Some("greed".into()),
                    all_in: None,
                    triggering: true,
                    keep_summons: None,
                };
            },
        ),
        (
            "a timer start with ms",
            "v2 timer add needs exactly a positive ms",
            |draft| {
                encounter_mut(draft).rules[0].actions[2] = ProjectV2EncounterAction::Timer {
                    timer: "summon_delay".into(),
                    operation: ProjectV2TimerOperation::Start,
                    ms: Some(1),
                };
            },
        ),
        (
            "a one_of with one branch",
            "v2 one_of needs two branches",
            |draft| {
                if let ProjectV2EncounterAction::OneOf { branches } =
                    &mut encounter_mut(draft).rules[1].actions[1]
                {
                    branches.truncate(1);
                }
            },
        ),
        (
            "an Item where a Creature is needed",
            "v2 encounter reference family mismatch",
            |draft| {
                encounter_mut(draft).participants[0].creatures =
                    vec![reference(ProjectV2Family::Item, VORTEX)];
            },
        ),
        (
            "an unadmitted Creature",
            "unresolved v2 typed definition reference",
            |draft| {
                encounter_mut(draft).participants[0].creatures = vec![reference(
                    ProjectV2Family::Creature,
                    "oteryn:creature.unknown",
                )];
            },
        ),
        (
            "a creature bound to an unadmitted encounter",
            "unresolved v2 typed definition reference",
            |draft| {
                for profile in &mut draft.state.authoring_profiles {
                    if let ProjectV2AuthoringProfileData::Creature(creature) = &mut profile.data {
                        creature.encounters = vec![reference(
                            ProjectV2Family::Encounter,
                            "oteryn:encounter.other",
                        )];
                    }
                }
            },
        ),
        (
            "a spawn with an empty count",
            "v2 encounter amount is out of range",
            |draft| {
                if let ProjectV2EncounterAction::Spawn { count, .. } =
                    &mut encounter_mut(draft).rules[0].actions[0]
                {
                    count.min = 0;
                }
            },
        ),
    ];
    for (label, expected, mutate) in cases {
        let mut broken = draft();
        mutate(&mut broken);
        let error = admit(broken).expect_err(label);
        assert!(
            error.contains(expected),
            "{label}: expected {expected:?}, got {error}"
        );
    }
}

#[test]
fn unknown_encounter_fields_fail_closed() {
    let mut value = details_json();
    assert!(serde_json::from_value::<ProjectV2EncounterDetails>(value.clone()).is_ok());
    value["rules"][0]["actions"][0]["script"] = json!("onThink");
    assert!(
        serde_json::from_value::<ProjectV2EncounterDetails>(value).is_err(),
        "unknown action field admitted"
    );
    let mut value = details_json();
    value["rules"][0]["trigger"]["kind"] = json!("on_think");
    assert!(
        serde_json::from_value::<ProjectV2EncounterDetails>(value).is_err(),
        "unknown trigger admitted"
    );
}
