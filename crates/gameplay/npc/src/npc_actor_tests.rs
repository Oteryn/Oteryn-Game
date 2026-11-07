#![allow(clippy::expect_used, reason = "native NPC walking fixtures")]
use super::*;
use crate::content::{
    CanonicalProjectDocuments, DefinitionIdentityDocument, ProjectDraft, ProjectReferenceRecord,
    ProjectV2AuthoringProfile, ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2Family,
    ProjectV2Identity, ProjectV2Movement, ProjectV2State, ProjectV2Targeting, ProjectV2Wander,
    ProjectionDocument, npc_catalogue_preproduction_limits,
};
const NPC: &str = "oteryn:npc.alice";
const BEHAVIOR: &str = "oteryn:behavior.npc.alice";

fn source_fields(speed: i64) -> Vec<ProjectV2CandidateField> {
    vec![
        ProjectV2CandidateField {
            field_path: BASE_SPEED.into(),
            value: ProjectV2CandidateValue::Integer(speed),
        },
        ProjectV2CandidateField {
            field_path: FLOOR_CHANGE.into(),
            value: ProjectV2CandidateValue::Boolean(false),
        },
    ]
}

fn behavior(interval_ms: u64, radius_tiles: u16) -> ProjectV2BehaviorAuthoring {
    ProjectV2BehaviorAuthoring {
        movement: ProjectV2Movement {
            can_walk: true,
            pass_through: false,
            pushable: false,
            push_items: false,
            push_creatures: false,
            walks_on_energy: false,
            walks_on_fire: false,
            walks_on_poison: false,
            wander: Some(ProjectV2Wander {
                interval_ms,
                radius_tiles,
            }),
        },
        targeting: ProjectV2Targeting {
            hostile: false,
            can_target: false,
            sense_invisible: false,
            target_distance_tiles: 1,
            static_attack_chance_ppm: 0,
            flee_health: 0,
            change_target: None,
            strategy_weights: None,
        },
        attacks: vec![],
        defenses: vec![],
        voices: None,
        summons: None,
        periodic_audio: None,
        faction: None,
        event_bindings: vec![],
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: "definition-r1".into(),
    }
}

fn catalogue(revision: &str, speed: i64, interval_ms: u64, radius: u16) -> NpcDataCatalogue {
    let generic = |key: &str| ProjectReferenceRecord::Generic {
        identity: DefinitionIdentityDocument {
            family: "Behavior".into(),
            key: key.into(),
            revision: "definition-r1".into(),
        },
        client_projection: ProjectionDocument::ServerOnly,
    };
    let npc =
        |key: &str, fields: Vec<ProjectV2CandidateField>, linked: bool| ProjectV2Declaration::Npc {
            identity: ProjectV2Identity {
                key: key.into(),
                revision: "definition-r1".into(),
            },
            presentation: None,
            behavior: linked.then(|| reference(ProjectV2Family::Behavior, BEHAVIOR)),
            dialogue: None,
            services: vec![],
            fields,
        };
    let draft = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: revision.into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "license:project-owned-v1".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: vec![
                generic(BEHAVIOR),
                generic("oteryn:behavior.npc.unreferenced"),
            ],
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: vec![
                npc(NPC, source_fields(speed), true),
                npc("oteryn:npc.no-source", vec![], true),
                npc("oteryn:npc.no-behavior", source_fields(speed), false),
            ],
            authoring_profiles: vec![
                ProjectV2AuthoringProfile {
                    target: reference(ProjectV2Family::Behavior, BEHAVIOR),
                    data: ProjectV2AuthoringProfileData::Behavior(behavior(interval_ms, radius)),
                },
                ProjectV2AuthoringProfile {
                    target: reference(
                        ProjectV2Family::Behavior,
                        "oteryn:behavior.npc.unreferenced",
                    ),
                    data: ProjectV2AuthoringProfileData::Behavior(behavior(1_000, 200)),
                },
            ],
            ..ProjectV2State::default()
        },
    };
    let limits = npc_catalogue_preproduction_limits().project;
    let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)
        .expect("native canonical documents")
        .into_snapshot(limits)
        .expect("snapshot")
        .parse(limits)
        .expect("native project");
    NpcDataCatalogue::from_project(project, limits).expect("native NPC catalogue")
}

#[test]
fn book_resolves_native_behavior_and_exact_npc_presence_fields_without_defaults() {
    let source = catalogue("walking-r1", 55, 2_000, 2);
    let model = source.service_model_for_project().expect("native model");
    let book = NpcWalkBook::from_catalogue(&source, &model).expect("exact catalogue");
    assert_eq!(book.source_tree_digest(), source.source_tree_digest());
    assert_eq!(book.project_revision(), source.project_revision());
    assert_eq!(book.profiles.len(), 1);
    assert_eq!(
        book.profiles[NPC],
        WalkProfile {
            interval_ms: 2_000,
            radius_tiles: 2,
            base_speed: 55
        }
    );
    assert!(!book.profiles.contains_key("oteryn:npc.no-source"));
    assert!(!book.profiles.contains_key("oteryn:npc.no-behavior"));
    assert!(
        !book
            .profiles
            .contains_key("oteryn:behavior.npc.unreferenced")
    );
    for changed in [
        catalogue("walking-r1", 56, 2_000, 2),
        catalogue("walking-r2", 55, 2_000, 2),
    ] {
        assert!(matches!(
            NpcWalkBook::from_catalogue(&changed, &model),
            Err(WalkError::ContentSourceMismatch)
        ));
    }
}

#[test]
fn disabled_missing_mistyped_and_duplicate_source_values_cannot_supply_walk_profiles() {
    let mut source = behavior(2_000, 2);
    for speed in [-1, 0, 65_536, i64::MAX] {
        assert!(WalkProfile::from_source(&source_fields(speed), &source).is_none());
    }
    assert!(WalkProfile::from_source(&[], &source).is_none());
    let fields = source_fields(55);
    assert!(WalkProfile::from_source(&fields[..1], &source).is_none());
    assert!(WalkProfile::from_source(&fields[1..], &source).is_none());
    let mut duplicate = fields.clone();
    duplicate.push(fields[0].clone());
    assert!(WalkProfile::from_source(&duplicate, &source).is_none());
    duplicate = fields.clone();
    duplicate.push(fields[1].clone());
    assert!(WalkProfile::from_source(&duplicate, &source).is_none());
    let mut mistyped = fields.clone();
    mistyped[0].value = ProjectV2CandidateValue::Text("55".into());
    assert!(WalkProfile::from_source(&mistyped, &source).is_none());
    for floor in [
        ProjectV2CandidateValue::Integer(0),
        ProjectV2CandidateValue::Boolean(true),
    ] {
        mistyped = fields.clone();
        mistyped[1].value = floor;
        assert!(WalkProfile::from_source(&mistyped, &source).is_none());
    }
    source.movement.can_walk = false;
    assert!(WalkProfile::from_source(&fields, &source).is_none());
    source = behavior(0, 2);
    assert!(WalkProfile::from_source(&fields, &source).is_none());
    source.movement.wander = None;
    assert!(WalkProfile::from_source(&fields, &source).is_none());
}
