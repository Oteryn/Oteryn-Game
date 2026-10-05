//! Source-specific authored phase state registry, with actual pinned Crystal participant bindings.
//! Does NOT assert a donor Lua producer or full shared-life/spawn/quest implementation.
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2SourceIdentityBinding,
    ProjectV2SourceIdentityDisposition,
};
use crate::foundation::{BoneOverlordCagePhase, BonePhaseError, ChannelRuntimeV1, ExactActorRef};
const SOURCE_REVISION: &str =
    "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f";
const ENCOUNTER: &str = "cc3143372125a5bd4bff66bbd1dc0b3cf2989e6892e3135901f5083f2cd689ef";
/// Raw arrays must come from current native artifact owning loader (same trusted seam as MeleeSource).
/// Passing a caller-authored digest is not a cryptographic proof of array membership.
pub(crate) fn register_bone_phase(
    runtime: &ChannelRuntimeV1,
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    records: &[ProjectReferenceRecord],
    profiles: &[ProjectV2AuthoringProfile],
    bindings: &[ProjectV2SourceIdentityBinding],
    artifact_digest: [u8; 32],
) -> Result<BoneOverlordCagePhase, BonePhaseError> {
    if runtime.content_pin().server_artifact_digest() != artifact_digest {
        return Err(BonePhaseError::ContentChanged);
    }
    for (slug, name, file, hp) in [
        (
            "elyrax_s_soulcage",
            "Elyrax's Soulcage",
            "elyraxs_soulcage.lua",
            120000,
        ),
        (
            "myzareth_s_soulcage",
            "Myzareth's Soulcage",
            "myzareths_soulcage.lua",
            120000,
        ),
        (
            "scarith_s_soulcage",
            "Scarith's Soulcage",
            "scariths_soulcage.lua",
            120000,
        ),
        (
            "zharvorin_s_soulcage",
            "Zharvorin's Soulcage",
            "zharvorins_soulcage.lua",
            120000,
        ),
        (
            "bonelord_s_phylactery",
            "Bonelord's Phylactery",
            "bonelords_phylactery.lua",
            50000,
        ),
    ] {
        let target = Ref {
            family: ProjectV2Family::Creature,
            key: format!("oteryn:creature.{slug}"),
            revision: "definition-r1".into(),
        };
        if records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..} if identity.family=="Creature" && identity.key==target.key && identity.revision==target.revision)).count()!=1 {return Err(BonePhaseError::InvalidSource)}
        let chosen: Vec<_> = profiles.iter().filter(|p| p.target == target).collect();
        if chosen.len() != 1 {
            return Err(BonePhaseError::InvalidSource);
        }
        let Data::Creature(c) = &chosen[0].data else {
            return Err(BonePhaseError::InvalidSource);
        };
        if c.health != Some(hp) || c.details.as_ref().map(|d| d.display_name.as_str()) != Some(name)
        {
            return Err(BonePhaseError::InvalidSource);
        }
        let bound: Vec<_> = bindings
            .iter()
            .filter(|b| {
                b.target == target
                    && b.source_key == "oteryn:source.crystalserver"
                    && b.identity_namespace == "crystalserver/monster-file"
            })
            .collect();
        if bound.len() != 1
            || bound[0].source_revision != SOURCE_REVISION
            || bound[0].external_id
                != format!("data-global/monster/winter_update_2025/quests/{file}")
            || bound[0].disposition != ProjectV2SourceIdentityDisposition::Exact
        {
            return Err(BonePhaseError::InvalidSource);
        }
    }
    runtime.bind_bone_overlord_cage_phase(cages, phylactery, ENCOUNTER, artifact_digest)
}

/// Native encounter-admission seam. Explicit placement/actor association comes from current
/// Map owner, never a global name scan. UNKNOWN/unpromoted isolated placements are permitted
/// as PROJECT local fixtures; they do not assert donor/Global map anchor coordinates.
// Keep register_bone_phase_shared ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn register_bone_phase_shared(
    runtime: &mut ChannelRuntimeV1,
    project: &crate::content::WorldProject,
    map: &crate::content::CanonicalReferencePlayableContent,
    map_fence: &crate::world_runtime::ScopeContentGenerationFence,
    current: &crate::foundation::ScopeRuntimeFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    placements: [&crate::content::PlacementKey; 5],
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
) -> Result<BoneOverlordCagePhase, BonePhaseError> {
    use crate::content::DefinitionFamily;
    use crate::foundation::RuntimeScopeRefV1;
    let binding = runtime.binding();
    let scope = RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id());
    if !current.is_current_for_scope(scope, binding.scope_generation())
        || !current.accepts_stamp(stamp)
    {
        return Err(BonePhaseError::Carrier(
            crate::foundation::CarrierError::WrongScope,
        ));
    }
    if project
        .lower_reference_source()
        .map_err(|_| BonePhaseError::InvalidSource)?
        .world_id
        != binding.world_id()
        || map.world_id != binding.world_id()
    {
        return Err(BonePhaseError::InvalidSource);
    }
    map_fence
        .qualifies_current_content(scope, binding.scope_generation(), map)
        .map_err(|_| BonePhaseError::ContentChanged)?;
    let keys = [
        "oteryn:creature.elyrax_s_soulcage",
        "oteryn:creature.myzareth_s_soulcage",
        "oteryn:creature.scarith_s_soulcage",
        "oteryn:creature.zharvorin_s_soulcage",
        "oteryn:creature.bonelord_s_phylactery",
    ];
    let actors = [cages[0], cages[1], cages[2], cages[3], phylactery];
    for i in 0..5 {
        if placements[..i].contains(&placements[i]) {
            return Err(BonePhaseError::WrongParticipant);
        }
        let mut chosen = map.placements.iter().filter(|p| &p.key == placements[i]);
        let p = chosen.next().ok_or(BonePhaseError::InvalidSource)?;
        if chosen.next().is_some() {
            return Err(BonePhaseError::InvalidSource);
        }
        if p.definition.family() != DefinitionFamily::Creature
            || p.definition.key().as_str() != keys[i]
            || p.definition.revision().as_str() != "definition-r1"
            || p.address.world_id != binding.world_id()
            || p.address.coordinate_frame != map.coordinate_frame
            || !map.definitions.iter().any(|d| d.definition == p.definition)
        {
            return Err(BonePhaseError::InvalidSource);
        }
        let position = runtime.read_actor_position(actors[i])?;
        let actual = position.position();
        if position.context() != runtime.pinned_movement_context()
            || actual.x != p.address.cell.x
            || actual.y != p.address.cell.y
            || i32::from(actual.floor) != p.address.cell.z
        {
            return Err(BonePhaseError::WrongParticipant);
        }
    }
    let draft = project.migrate_to_v2();
    let state = register_bone_phase(
        runtime,
        cages,
        phylactery,
        &draft.core.records,
        &draft.state.authoring_profiles,
        &draft.state.source_identity_bindings,
        runtime.content_pin().server_artifact_digest(),
    )?;
    // Full source pin/exact physical generations revalidated by native group setter before any
    // registration write. The source-authored actor stats remain untouched.
    runtime.register_bone_shared_hp(&state, current, stamp)?;
    Ok(state)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_hp_actual_source_tests {
    use super::*;
    use crate::content::*;
    #[test]
    #[ignore = "Requires retained actual eleven-document native import capture; no synthetic source membership"]
    fn actual_source_shared_native_hp_generic_pin_phase_and_atomicity()
    -> Result<(), Box<dyn std::error::Error>> {
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96000000,
            max_total_bytes: 160000000,
            max_json_depth: 24,
            max_decoded_fields: 2120000,
            max_string_bytes: 43000000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 16,
            max_reimport_states: 404,
        };
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let p = retained_native_capture_path.as_path();
        let project = capture_world_project(
            p.parent().expect("bone_phase_registry.rs:shared_hp_actual_source_tests:122: qualified fixture operation must succeed"),
            p.file_name().expect("bone_phase_registry.rs:shared_hp_actual_source_tests:123: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )?;
        let world = project.lower_reference_source()?.world_id;
        let draft = project.migrate_to_v2();
        crate::foundation::bone_shared_actual_owner_harness(world, |runtime, cages, phyl| {
            let good = register_bone_phase(
                runtime,
                cages,
                phyl,
                &draft.core.records,
                &draft.state.authoring_profiles,
                &draft.state.source_identity_bindings,
                runtime.content_pin().server_artifact_digest(),
            )
            .expect("actual five source profiles/bindings");
            let mut bad = draft.state.source_identity_bindings.clone();
            bad.iter_mut()
                .find(|b| b.target.key == "oteryn:creature.elyrax_s_soulcage")
                .expect("bone_phase_registry.rs:shared_hp_actual_source_tests:145: qualified fixture operation must succeed")
                .source_revision = "not-pinned".into();
            assert!(
                register_bone_phase(
                    runtime,
                    cages,
                    phyl,
                    &draft.core.records,
                    &draft.state.authoring_profiles,
                    &bad,
                    runtime.content_pin().server_artifact_digest()
                )
                .is_err()
            );
            good
        });
        Ok(())
    }
}
