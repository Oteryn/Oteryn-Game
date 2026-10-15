#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{ffi::OsStr, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world")
}
fn captured() -> WorldProject {
    capture_world_project(
        root().parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source")
}
#[test]
fn repository_import_preserves_all_data_and_approximation_flags() {
    let project = captured();
    let baseline = project.v2().expect("v2");
    let npc = baseline.declarations.iter().find(|d| matches!(d,ProjectV2Declaration::Npc {identity,..} if identity.key=="oteryn:npc.mud")).expect("MUD").clone();
    let sources = baseline.sources.clone();
    let bindings = baseline.source_identity_bindings.clone();
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
            .expect("data import");
    // Revision and source tree digest measured on the NPC-presence repository source (NPC-CONTENT-2).
    assert_eq!(
        catalogue.project_revision(),
        "npc-content-2-presence-20261015-r1"
    );
    assert_eq!(
        catalogue.source_tree_digest(),
        "d7abdc3958d2bf73f39539ab659952693bc1c836ee4c5e05de2dbeded82c1659"
    );
    assert_eq!(catalogue.npc_count(), 1282);
    assert_eq!(catalogue.dialogue_count(), 836);
    assert_eq!(catalogue.service_count(), 380);
    assert_eq!(catalogue.sources(), sources);
    assert_eq!(catalogue.source_identity_bindings(), bindings);
    let (reference, imported) = catalogue
        .records()
        .find(|(r, _)| r.key == "oteryn:npc.mud")
        .expect("imported MUD");
    assert_eq!(imported, &npc);
    assert_eq!(catalogue.declaration(reference), Some(&npc));
    if let ProjectV2Declaration::Npc {
        presentation,
        behavior,
        fields,
        ..
    } = imported
    {
        assert!(
            presentation
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(
            behavior
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(fields.iter().any(|f|matches!(&f.value,ProjectV2CandidateValue::Text(v) if v.contains("APPROXIMATE_WIKI_VISUAL_INVISIBLE_MAPPING"))));
    }
}
#[test]
fn npc_presence_facts_and_voices_are_staged_on_npc_definitions() {
    // NPC-CONTENT-2 (NPC-BEHAVIOUR0 §5, §6): movement facts on every sourced NPC; voices only
    // where the bound sources agree, each line within CHAT0-RL-01 and free of placeholders.
    let project = captured();
    let (mut moving, mut voiced, mut lines) = (0, 0, 0);
    for declaration in &project.v2().expect("v2").declarations {
        let ProjectV2Declaration::Npc { fields, .. } = declaration else {
            continue;
        };
        let value = |path: &str| {
            fields
                .iter()
                .find(|f| f.field_path == format!("oteryn:source.npc.{path}"))
                .map(|f| &f.value)
        };
        if let Some(speed) = value("base_speed") {
            moving += 1;
            assert_eq!(speed, &ProjectV2CandidateValue::Integer(55));
            assert_eq!(
                value("floor_change"),
                Some(&ProjectV2CandidateValue::Boolean(false))
            );
        }
        let Some(ProjectV2CandidateValue::Text(text)) = value("voice_lines") else {
            assert!(
                value("voice_interval_ms").is_none() && value("voice_chance_percent").is_none()
            );
            continue;
        };
        voiced += 1;
        assert!(
            matches!(value("voice_interval_ms"), Some(ProjectV2CandidateValue::Integer(i)) if *i > 0)
        );
        assert!(
            matches!(value("voice_chance_percent"), Some(ProjectV2CandidateValue::Integer(c)) if (1..=100).contains(c))
        );
        let parsed: Vec<serde_json::Value> = serde_json::from_str(text).expect("voice lines");
        for line in parsed {
            let said = line["text"].as_str().expect("line text");
            assert!(!said.is_empty() && said.chars().count() <= 255 && said.len() <= 1020);
            assert!(!said.contains('|'), "unresolved placeholder: {said}");
            assert!(line["yell"].is_boolean());
            lines += 1;
        }
    }
    assert_eq!((moving, voiced, lines), (1122, 282, 626));
}
#[test]
fn dangling_dialogue_is_rejected_by_the_existing_import_admission() {
    let mut draft = captured().migrate_to_v2();
    let actor = draft
        .state
        .declarations
        .iter_mut()
        .find(|d| {
            matches!(
                d,
                ProjectV2Declaration::Npc {
                    dialogue: Some(_),
                    ..
                }
            )
        })
        .expect("NPC with dialogue");
    if let ProjectV2Declaration::Npc {
        dialogue: Some(reference),
        ..
    } = actor
    {
        reference.key = "oteryn:dialogue.npc.missing_data_import_target".to_owned();
    }
    assert!(
        CanonicalProjectDocuments::from_v2_draft(
            draft,
            npc_catalogue_preproduction_limits().project
        )
        .is_err()
    );
}
#[test]
fn malformed_and_mismatched_source_pins_fail_closed() {
    assert!(load_data_only_npc_catalogue(&root(), &"0".repeat(64)).is_err());
    assert!(
        load_data_only_npc_catalogue(&PathBuf::from("/nonexistent/npc-project"), "not-a-sha256")
            .is_err()
    );
}

#[test]
fn legacy_project_without_v2_cannot_be_imported_as_npc_data() {
    let core = captured().migrate_to_v2().core;
    let limits = npc_catalogue_preproduction_limits().project;
    let legacy = CanonicalProjectDocuments::from_draft(core, limits)
        .expect("valid legacy project")
        .into_snapshot(limits)
        .expect("legacy snapshot")
        .parse(limits)
        .expect("validated legacy");
    assert!(legacy.v2().is_none());
    assert!(NpcDataCatalogue::from_project(legacy, limits).is_err());
}
