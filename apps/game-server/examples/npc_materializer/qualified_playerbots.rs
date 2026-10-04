//! Bounded offline authoring admission of five source-qualified Playerbots NPCs.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;
use serde::Serialize;

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r16/native-additions.json"
);
// Exact R13 published parent 8bdc12d54909c4667da63ecf56b2cae163fff34c.
const DIGEST: &str = "9e367aaf332eed920bc512aab0e027f6591a7f52aca69231bff631b311be2acc";
const PREDECESSOR_ECE: &str = "bbfb6811419f4f7073f3564ff9f3660fe95acd83974877028fec0819d1a84cc2";
const PREDECESSOR_REFERENCE: &str =
    "4793348d210d67b4794ce222c44a290ba9a64a2fd7edaab0828190448355cba2";
const MAPPER: &str = "NPC_BOUNDED_PLAYERBOTS_XML_OVERLAY/v1";
const MAPPER_DIGEST: &str = "8ccb44ed014fd5b69af98ae4c5b4b6f33a9cea2bc5400638936f071b013bab43";
const FROM: &str = "g4-npc-qualified-summer-r14";
const TO: &str = "g4-npc-qualified-playerbots-r15";
const DONOR: &str = "oteryn:source.tibia_playerbots_project";
const PIN: &str = "1afc43cf7efbf4ccc32df28ff45b726617050fa0";
const ACTORS: &[(&str, &str, &str, u16, [u16; 4])] = &[
    ("garzon", "Garzon", "12396", 130, [96, 63, 71, 97]),
    ("kito", "Kito", "12349", 143, [115, 42, 97, 116]),
    ("makao", "Makao", "12350", 143, [114, 115, 97, 116]),
    ("namasa", "Namasa", "12351", 147, [78, 118, 97, 114]),
    ("tatak", "Tatak", "12352", 143, [77, 101, 96, 116]),
];
type AdmissionResult<T> = Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Additions {
    declarations: Vec<ProjectV2Declaration>,
    records: Vec<ProjectReferenceRecord>,
    authoring_profiles: Vec<ProjectV2AuthoringProfile>,
    source_identity_bindings: Vec<ProjectV2SourceIdentityBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    from_project_revision: String,
    project_revision: String,
    native_additions: Additions,
    new_imports: Vec<ImportBatch>,
    new_sources: Vec<ProjectV2Source>,
    reused_wiki_imports: Vec<ProjectV2Source>,
}
fn key<T: Serialize>(value: &T) -> AdmissionResult<String> {
    Ok(serde_json::to_value(value)?["identity"]["key"]
        .as_str()
        .ok_or("missing identity key")?
        .to_owned())
}
fn keys<T: Serialize>(rows: &[T]) -> AdmissionResult<BTreeSet<String>> {
    let values = rows
        .iter()
        .map(key)
        .collect::<AdmissionResult<BTreeSet<_>>>()?;
    if values.len() != rows.len() {
        return Err("duplicate playerbots identity".into());
    }
    Ok(values)
}
fn named(family: &str, stem: &str) -> String {
    format!("oteryn:{family}.npc.{stem}")
}
fn valid_ref(value: &ProjectV2DefinitionRef, family: ProjectV2Family, expected: &str) -> bool {
    value.family == family && value.key == expected && value.revision == "definition-r1"
}
fn binding_key(b: &ProjectV2SourceIdentityBinding) -> (&str, &str, &str, &str) {
    (
        &b.source_key,
        &b.source_revision,
        &b.identity_namespace,
        &b.external_id,
    )
}
fn digest_shape(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn expected_profile(
    stem: &str,
    family: ProjectV2Family,
) -> AdmissionResult<ProjectV2AuthoringProfileData> {
    let actor = ACTORS
        .iter()
        .find(|a| a.0 == stem)
        .ok_or("profile actor outside scope")?;
    let document = if family == ProjectV2Family::Presentation {
        let palette = ["Head", "Body", "Legs", "Feet"].iter().zip(actor.4)
            .map(|(slot, color)| serde_json::json!({"slot":slot,"asset_binding":format!("canary.appearance:palette/{color}")}))
            .collect::<Vec<_>>();
        serde_json::json!({"kind":"Presentation","profile":{"asset_binding":format!("canary.appearance:outfit/{}",actor.3),"light_level":0,"palette_bindings":palette}})
    } else if family == ProjectV2Family::Behavior {
        // Uses the mature wave-A target semantics; interval/radius are explicit in all five XMLs.
        serde_json::json!({"kind":"Behavior","profile":{"movement":{"can_walk":true,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false,"wander":{"interval_ms":1500,"radius_tiles":2}},"targeting":{"hostile":false,"can_target":false,"sense_invisible":false,"target_distance_tiles":0,"static_attack_chance_ppm":0,"flee_health":0}}})
    } else {
        return Err("profile family outside scope".into());
    };
    Ok(serde_json::from_value(document)?)
}
fn source_preflight(draft: &ProjectV2Draft, packet: &Packet) -> AdmissionResult<()> {
    if packet.new_imports.len() != 2
        || packet.new_sources.len() != 2
        || packet.reused_wiki_imports.len() != 2
    {
        return Err("playerbots exact import inventory drifted".into());
    }
    let expected = [
        (
            "g4-npc-prices-tibiawiki-br-r1",
            "tibiawiki-br-npc-0773232ddd356be2",
            "0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a",
        ),
        (
            "g4-npc-prices-tibiopedia-r1",
            "tibiopedia-npc-43bfc91ec7721df1",
            "43bfc91ec7721df150d3803f7123df1909a167c6606e54b112075a433fda8651",
        ),
    ];
    let mut seen = BTreeSet::new();
    for source in &packet.reused_wiki_imports {
        if source.key != "oteryn:source.tibiawiki"
            || source.evidence != ProjectV2EvidenceClass::Derived
            || !expected.iter().any(|e| {
                source.import_batch_id == e.0 && source.revision == e.1 && source.sha256 == e.2
            })
            || !seen.insert(source.revision.clone())
        {
            return Err("playerbots reused wiki source drifted".into());
        }
        let current = draft
            .state
            .sources
            .iter()
            .filter(|s| s.key == source.key && s.revision == source.revision)
            .collect::<Vec<_>>();
        let imports = draft
            .core
            .imports
            .iter()
            .filter(|i| i.batch_id == source.import_batch_id)
            .collect::<Vec<_>>();
        if current != vec![source]
            || imports.len() != 1
            || imports[0].source_revision != source.revision
            || imports[0].source_artifact_sha256 != source.sha256
        {
            return Err("playerbots current wiki source/import fence drifted".into());
        }
    }
    let mut batches = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for source in &packet.new_sources {
        let import = packet
            .new_imports
            .iter()
            .find(|i| i.batch_id == source.import_batch_id)
            .ok_or("missing new import")?;
        let donor = source.key == DONOR;
        let (batch, repository, evidence) = if donor {
            (
                "g4-npc-playerbots-xml-five-r16",
                "adrunkhuman/tibia-playerbots-project",
                ProjectV2EvidenceClass::OtsHypothesisOnly,
            )
        } else {
            (
                "g4-npc-playerbots-wiki-images-r16",
                "tibiawiki.com.br",
                ProjectV2EvidenceClass::Derived,
            )
        };
        if (!donor && source.key != "oteryn:source.tibiawiki")
            || source.evidence != evidence
            || (donor && source.revision != PIN)
            || (!donor && !source.revision.starts_with("npc-wiki-image-qualification-"))
            || import.batch_id != batch
            || import.source_repository != repository
            || import.source_revision != source.revision
            || import.source_artifact_sha256 != source.sha256
            || !digest_shape(&source.sha256)
            || import.source_generation_profile
                != if donor {
                    "NPC_PINNED_XML_SOURCE_CENSUS/v1"
                } else {
                    "NPC_FIVE_XML_WIKI_IMAGE_QUALIFICATION_FACTS/v1"
                }
            || import.importer != MAPPER
            || import.mapper != MAPPER
            || import.mapper_revision != "npc-playerbots-xml-five-r16"
            || import.mapper_sha256 != MAPPER_DIGEST
            || serde_json::to_value(import)?["access_disposition"] != "PENDING"
            || (!donor
                && source.revision
                    != format!("npc-wiki-image-qualification-{}", &source.sha256[..16]))
            || !import.candidates.is_empty()
            || !import.reimport_states.is_empty()
            || !batches.insert(import.batch_id.clone())
            || !sources.insert((source.key.clone(), source.revision.clone()))
            || draft
                .core
                .imports
                .iter()
                .any(|i| i.batch_id == import.batch_id)
            || draft
                .state
                .sources
                .iter()
                .any(|s| s.key == source.key && s.revision == source.revision)
        {
            return Err("playerbots new source/import fence drifted".into());
        }
    }
    if batches.len() != 2 || !packet.new_sources.iter().any(|s| s.key == DONOR) {
        return Err("playerbots donor/image import closure drifted".into());
    }
    Ok(())
}
fn apply_packet(draft: &mut ProjectV2Draft, packet: Packet) -> AdmissionResult<usize> {
    if packet.schema != "OTERYN_NPC_QUALIFIED_PLAYERBOTS_ADMISSION/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
    {
        return Err("playerbots schema/revision drifted".into());
    }
    source_preflight(draft, &packet)?;
    let a = &packet.native_additions;
    let npc_keys = ACTORS
        .iter()
        .map(|x| format!("oteryn:npc.{}", x.0))
        .collect::<BTreeSet<_>>();
    let profile_keys = ACTORS
        .iter()
        .flat_map(|x| [named("presentation", x.0), named("behavior", x.0)])
        .collect::<BTreeSet<_>>();
    if keys(&a.declarations)? != npc_keys
        || keys(&a.records)? != profile_keys
        || a.authoring_profiles.len() != 10
        || a.source_identity_bindings.len() != 15
    {
        return Err("playerbots closed identity inventory drifted".into());
    }
    let existing = draft
        .state
        .declarations
        .iter()
        .map(key)
        .chain(draft.core.records.iter().map(key))
        .collect::<AdmissionResult<BTreeSet<_>>>()?;
    if existing
        .iter()
        .any(|k| npc_keys.contains(k) || profile_keys.contains(k))
        || draft
            .state
            .authoring_profiles
            .iter()
            .any(|p| profile_keys.contains(&p.target.key))
    {
        return Err("playerbots canonical identity collision".into());
    }
    for declaration in &a.declarations {
        let ProjectV2Declaration::Npc {
            identity,
            presentation,
            behavior,
            dialogue,
            services,
            fields,
        } = declaration
        else {
            return Err("playerbots declaration family".into());
        };
        let stem = identity
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("playerbots NPC key")?;
        if identity.revision != "definition-r1"
            || dialogue.is_some()
            || !services.is_empty()
            || !fields.is_empty()
            || presentation.as_ref().is_none_or(|r| {
                !valid_ref(
                    r,
                    ProjectV2Family::Presentation,
                    &named("presentation", stem),
                )
            })
            || behavior
                .as_ref()
                .is_none_or(|r| !valid_ref(r, ProjectV2Family::Behavior, &named("behavior", stem)))
        {
            return Err("playerbots NPC scope/reference drifted".into());
        }
    }
    for record in &a.records {
        let ProjectReferenceRecord::Generic {
            identity,
            client_projection,
        } = record
        else {
            return Err("playerbots record family".into());
        };
        let value = serde_json::to_value(identity)?;
        let family = if identity.key.starts_with("oteryn:presentation.npc.") {
            "Presentation"
        } else {
            "Behavior"
        };
        let projection = if family == "Presentation" {
            ProjectionDocument::ClientSafe
        } else {
            ProjectionDocument::ServerOnly
        };
        if identity.revision != "definition-r1"
            || value["family"] != family
            || *client_projection != projection
        {
            return Err("playerbots Generic family/projection drifted".into());
        }
    }
    let mut profiles = BTreeSet::new();
    for profile in &a.authoring_profiles {
        let (family, prefix) = if profile.target.key.starts_with("oteryn:presentation.npc.") {
            (ProjectV2Family::Presentation, "oteryn:presentation.npc.")
        } else {
            (ProjectV2Family::Behavior, "oteryn:behavior.npc.")
        };
        let stem = profile
            .target
            .key
            .strip_prefix(prefix)
            .ok_or("playerbots profile key")?;
        if !valid_ref(
            &profile.target,
            family,
            &named(
                if family == ProjectV2Family::Presentation {
                    "presentation"
                } else {
                    "behavior"
                },
                stem,
            ),
        ) || profile.data != expected_profile(stem, family)?
            || !profiles.insert(profile.target.key.clone())
        {
            return Err("playerbots profile scope/duplicate drifted".into());
        }
    }
    if profiles != profile_keys {
        return Err("playerbots profile closure drifted".into());
    }
    let mut bindings = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for binding in &a.source_identity_bindings {
        let stem = binding
            .target
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("playerbots binding target")?;
        let actor = ACTORS
            .iter()
            .find(|x| x.0 == stem)
            .ok_or("playerbots binding actor")?;
        let source = packet
            .new_sources
            .iter()
            .chain(&packet.reused_wiki_imports)
            .find(|s| s.key == binding.source_key && s.revision == binding.source_revision)
            .ok_or("playerbots binding source")?;
        let (namespace, external) = if source.key == DONOR {
            (
                "tibia-playerbots-project/npc-xml",
                format!("server/data/npc/{}.xml", actor.1),
            )
        } else if source.import_batch_id == "g4-npc-prices-tibiawiki-br-r1" {
            ("mediawiki/page_id", actor.2.to_owned())
        } else if source.import_batch_id == "g4-npc-prices-tibiopedia-r1" {
            ("tibiopedia/npc-name", actor.1.to_owned())
        } else {
            return Err("image import cannot supply actor identity".into());
        };
        let id = binding_key(binding);
        if !valid_ref(
            &binding.target,
            ProjectV2Family::Npc,
            &format!("oteryn:npc.{stem}"),
        ) || binding.disposition != ProjectV2SourceIdentityDisposition::Exact
            || binding.identity_namespace != namespace
            || binding.external_id != external
            || !bindings.insert(id)
            || !targets.insert((
                binding.source_key.clone(),
                binding.source_revision.clone(),
                binding.target.key.clone(),
            ))
            || draft
                .state
                .source_identity_bindings
                .iter()
                .any(|old| binding_key(old) == id || old.target.key == binding.target.key)
        {
            return Err("playerbots source identity collision/drifted".into());
        }
    }
    // All typed references, immutable facts and current predecessor collisions preflight before mutation.
    let additions = packet.native_additions;
    draft.state.declarations.extend(additions.declarations);
    draft.core.records.extend(additions.records);
    draft
        .state
        .authoring_profiles
        .extend(additions.authoring_profiles);
    draft
        .state
        .source_identity_bindings
        .extend(additions.source_identity_bindings);
    draft.core.imports.extend(packet.new_imports);
    draft.state.sources.extend(packet.new_sources);
    draft.core.project_revision = TO.to_owned();
    Ok(5)
}
pub(super) fn apply(draft: &mut ProjectV2Draft) -> AdmissionResult<usize> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("playerbots packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    for (locator, expected) in [
        ("definitions/declarations.json", PREDECESSOR_ECE),
        ("definitions/reference.json", PREDECESSOR_REFERENCE),
    ] {
        let bytes = before
            .documents()
            .get(locator)
            .ok_or("missing complete playerbots predecessor")?;
        if hex_sha256(bytes) != expected {
            return Err(format!("playerbots complete predecessor digest drifted DIAG {locator} {expected} {}", hex_sha256(bytes)).into());
        }
    }
    apply_packet(draft, serde_json::from_slice(PACKET)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Packet {
        serde_json::from_slice(PACKET).expect("closed qualified DTO fixture")
    }
    fn fixture() -> ProjectV2Draft {
        // Captured prior BR/TP state, independent of the proposed packet's authority claims.
        let sources = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r16/fixture-wiki-sources.json")).unwrap();
        let imports = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r16/fixture-wiki-imports.json")).unwrap();
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: FROM.to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-09-27".to_owned(),
                records: vec![],
                imports,
                metadata: vec![],
            },
            state: ProjectV2State {
                sources,
                ..ProjectV2State::default()
            },
        }
    }
    fn rejects(draft: &mut ProjectV2Draft, p: Packet) {
        let before = draft.clone();
        assert!(apply_packet(draft, p).is_err());
        assert_eq!(*draft, before);
    }
    #[test]
    fn closed_scope_preserves_all_other_authoring_state() {
        let mut draft = fixture();
        let before = draft.clone();
        assert_eq!(apply_packet(&mut draft, packet()).unwrap(), 5);
        assert_eq!(
            (
                draft.state.declarations.len(),
                draft.core.records.len(),
                draft.state.authoring_profiles.len(),
                draft.state.source_identity_bindings.len()
            ),
            (5, 10, 10, 15)
        );
        assert_eq!(draft.state.placements, before.state.placements);
        assert_eq!(draft.state.worlds, before.state.worlds);
        assert_eq!(draft.state.editor, before.state.editor);
        assert_eq!(draft.core.metadata, before.core.metadata);
        assert_eq!(&draft.state.sources[..2], before.state.sources.as_slice());
        assert_eq!(&draft.core.imports[..2], before.core.imports.as_slice());
        assert_eq!(draft.core.project_revision, TO);
        rejects(&mut draft, packet());
    }
    #[test]
    fn substituted_current_source_import_and_revision_reject_atomically() {
        for operation in 0..3 {
            let mut draft = fixture();
            match operation {
                0 => draft.state.sources[0].sha256 = "wrong".to_owned(),
                1 => draft.core.imports[0].source_artifact_sha256 = "wrong".to_owned(),
                _ => draft.core.project_revision = "stale".to_owned(),
            }
            rejects(&mut draft, packet());
        }
    }
    #[test]
    fn full_predecessor_wrapper_rejects_substituted_package_atomically() {
        let mut draft = fixture();
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        assert_eq!(hex_sha256(PACKET), DIGEST);
    }
    #[test]
    fn late_binding_namespace_target_and_actor_id_reject_atomically() {
        for operation in 0..3 {
            let mut draft = fixture();
            let mut p = packet();
            let b = p
                .native_additions
                .source_identity_bindings
                .last_mut()
                .unwrap();
            match operation {
                0 => b.identity_namespace = "wrong".to_owned(),
                1 => b.target.revision = "stale".to_owned(),
                _ => b.external_id = "other actor".to_owned(),
            }
            rejects(&mut draft, p);
        }
    }
    #[test]
    fn profile_and_npc_reference_mutations_reject_without_scope_removal() {
        for operation in 0..3 {
            let mut draft = fixture();
            let mut p = packet();
            if operation == 0 {
                p.native_additions.authoring_profiles[0].target.revision = "stale".to_owned();
            } else if operation == 1 {
                let profile = p
                    .native_additions
                    .authoring_profiles
                    .iter_mut()
                    .find(|p| p.target.family == ProjectV2Family::Behavior)
                    .unwrap();
                let mut value = serde_json::to_value(&profile.data).unwrap();
                value["profile"]["movement"]["wander"]["radius_tiles"] = serde_json::json!(3);
                profile.data = serde_json::from_value(value).unwrap();
            } else if let ProjectV2Declaration::Npc { presentation, .. } =
                &mut p.native_additions.declarations[0]
            {
                presentation.as_mut().unwrap().key = "oteryn:presentation.npc.kito".to_owned();
            }
            rejects(&mut draft, p);
        }
    }
    #[test]
    fn new_source_evidence_and_existing_identity_collisions_reject_atomically() {
        let mut draft = fixture();
        let mut p = packet();
        p.new_sources[0].evidence = ProjectV2EvidenceClass::Proven;
        rejects(&mut draft, p);
        let mut draft = fixture();
        draft
            .core
            .records
            .push(packet().native_additions.records[0].clone());
        rejects(&mut draft, packet());
        let mut draft = fixture();
        draft.core.imports.push(packet().new_imports[0].clone());
        rejects(&mut draft, packet());
    }
    #[test]
    fn packet_envelope_rejects_unrecognized_authority_fields() {
        let mut value: Value = serde_json::from_slice(PACKET).unwrap();
        value["runtime_eligible"] = Value::Bool(true);
        assert!(serde_json::from_value::<Packet>(value).is_err());
    }
    #[test]
    fn new_import_mapper_and_source_image_generation_drift_reject_atomically() {
        for operation in 0..3 {
            let mut draft = fixture();
            let mut p = packet();
            match operation {
                0 => p.new_imports[0].mapper_sha256 = "0".repeat(64),
                1 => p.new_imports[1].source_generation_profile = "unrelated-profile".to_owned(),
                _ => {
                    p.new_sources[1].revision =
                        "npc-wiki-image-qualification-0000000000000000".to_owned()
                }
            }
            rejects(&mut draft, p);
        }
    }
    #[test]
    fn generic_projection_and_profile_duplicates_reject_atomically() {
        let mut draft = fixture();
        let mut p = packet();
        if let ProjectReferenceRecord::Generic {
            client_projection, ..
        } = &mut p.native_additions.records[0]
        {
            *client_projection = ProjectionDocument::ClientSafe;
        }
        rejects(&mut draft, p);
        let mut draft = fixture();
        let mut p = packet();
        p.native_additions.authoring_profiles[1] = p.native_additions.authoring_profiles[0].clone();
        rejects(&mut draft, p);
    }
}
