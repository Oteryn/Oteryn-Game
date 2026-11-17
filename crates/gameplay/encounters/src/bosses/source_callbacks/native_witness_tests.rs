//! The active definition closure, not a matching name or entry-room project, qualifies a callback.
use super::native_witness::qualify_document;
use crate::content::{
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref,
    ProjectV2Family as Family, native_gameplay::CreatureProfilesDocument,
};
use crate::creature_auto_attack::AttackError;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use sha2::{Digest, Sha256};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn document() -> std::result::Result<CreatureProfilesDocument, Box<dyn std::error::Error>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/creatures/definitions/spell-native-profiles.json"
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn cerebellum() -> Ref {
    Ref {
        family: Family::Creature,
        key: "oteryn:creature.cerebellum".into(),
        revision: "definition-r1".into(),
    }
}

#[test]
fn native_callback_witness_accepts_actual_body_and_root() -> Result {
    let source = document()?;
    assert!(
        qualify_document(&source, &cerebellum(), 1, ScheduleList::Attack)
            .is_ok_and(|entry| entry.is_some())
    );
    let gaz = Ref {
        family: Family::Creature,
        key: "oteryn:creature.gaz_haragoth".into(),
        revision: "definition-r1".into(),
    };
    assert!(
        qualify_document(&source, &gaz, 11, ScheduleList::Attack)
            .is_ok_and(|entry| entry.is_some())
    );
    let zamulosh = Ref {
        family: Family::Creature,
        key: "oteryn:creature.zamulosh".into(),
        revision: "definition-r1".into(),
    };
    assert!(
        qualify_document(&source, &zamulosh, 1, ScheduleList::Defence)
            .is_ok_and(|entry| entry.is_some())
    );
    Ok(())
}

#[test]
fn native_callback_witness_rejects_entry_project_as_monster_source() -> Result {
    let tiny: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../content/src/content/project/native_entry_room.json"
    ))?;
    let root = serde_json::json!({
        "schema":"OTERYN_NATIVE_MONSTER_DEFINITIONS/v1",
        "records":tiny["records"],"authoring_profiles":tiny["authoring_profiles"],
        "declarations":tiny["declarations"],"sources":[],"source_identity_bindings":[],
    });
    let mut source = document()?;
    source.source_definitions_sha256 = Some(
        Sha256::digest(serde_json::to_vec(&root)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    );
    assert!(matches!(
        qualify_document(&source, &cerebellum(), 1, ScheduleList::Attack),
        Err(AttackError::InvalidSource)
    ));
    Ok(())
}

#[test]
fn native_callback_witness_rejects_changed_parent_body() -> Result {
    let mut source = document()?;
    let record = source
        .records
        .iter_mut()
        .find(|record| record.profile.target == cerebellum())
        .ok_or("source Cerebellum")?;
    let ability = record
        .attack_definitions
        .as_mut()
        .ok_or("source closure")?
        .profiles
        .iter_mut()
        .find(|profile| {
            profile.target.key == "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head"
        })
        .ok_or("source callback")?;
    let Data::Ability(body) = &mut ability.data else {
        return Err("source callback kind".into());
    };
    body.details
        .as_mut()
        .ok_or("source callback body")?
        .needs_target = true;
    assert!(matches!(
        qualify_document(&source, &cerebellum(), 1, ScheduleList::Attack),
        Err(AttackError::InvalidSource)
    ));
    Ok(())
}

#[test]
fn native_callback_witness_rejects_changed_native_schedule() -> Result {
    let mut source = document()?;
    let record = source
        .records
        .iter_mut()
        .find(|record| record.profile.target == cerebellum())
        .ok_or("source Cerebellum")?;
    let Data::Behavior(behavior) = &mut record.behavior.as_mut().ok_or("source behavior")?.data
    else {
        return Err("source behavior kind".into());
    };
    behavior
        .attacks
        .get_mut(1)
        .ok_or("source callback entry")?
        .chance_ppm = 1_000_000;
    assert!(matches!(
        qualify_document(&source, &cerebellum(), 1, ScheduleList::Attack),
        Err(AttackError::InvalidSource)
    ));
    Ok(())
}
