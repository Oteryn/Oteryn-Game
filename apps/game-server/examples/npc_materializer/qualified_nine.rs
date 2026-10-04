//! Bounded offline authoring admission of nine source-qualified nine source-bound NPC definitions.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;
use serde::Serialize;

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r18/native-additions.json"
);
// Exact published 1132 predecessor and frozen source packet; root alone integrates this offline authoring stage.
const DIGEST: Option<&str> =
    Some("091b3ceb01b8dfe93139c0fc488f6ec0b1eda99120e9889506b9c4734be6fd71");
const PREDECESSOR_ECE: Option<&str> =
    Some("b17c71515e7917ef73317f89acb2eda2acae29d8e8555b8d0d90b2cce9de75bd");
const PREDECESSOR_REFERENCE: Option<&str> =
    Some("e3206701a025c032779718065cd0c93018a66bad8a4ff11023601298f1c98df2");
const MAPPER: &str = "NPC_BOUNDED_D15_D16_NINE_DEFINITION_BRIDGE/v1";
const MAPPER_DIGEST: &str = "eee39beed27c1b5fad1f7e0beaaea9d3978834752d3dbe85586f03ddb7600066";
const FROM: &str = "g4-npc-qualified-summer-object-r16";
const TO: &str = "g4-npc-qualified-nine-definitions-r17";
const DAKBUGS: &str = "oteryn:source.dakbugs";
const HAPPEN: &str = "oteryn:source.happen";
const FENCORE: &str = "oteryn:source.fencore";
// key, import batch, repository, qualified snapshot revision, artifact SHA; upstream Git pins remain in bound source artifacts.
const SOURCE_FACTS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "oteryn:source.dakbugs",
        "g4-npc-r18-definition-dakbugs",
        "dakotaotserver-glitch/dakbugs",
        "npc-definition-r18-4fe736b2e627a03a",
        "4fe736b2e627a03a226ee1aa41c8ba1967b685392cf728a47490bc1260e823ad",
    ),
    (
        "oteryn:source.happen",
        "g4-npc-r18-definition-happen",
        "Brunowots/Happen",
        "npc-definition-r18-fb3d7643d00af4f5",
        "fb3d7643d00af4f50e0d254db76df38a17e225b84738abe752bc8bbe8421b7b3",
    ),
    (
        "oteryn:source.fencore",
        "g4-npc-r18-definition-fencore",
        "fifth-empearl/fencore",
        "npc-definition-r18-0998d347997d2896",
        "0998d347997d2896b118bbc2cdfaf5c8a7b7fc7f5aacb10b65b41fab89567529",
    ),
];
// stem, BR page id, TP page stem, outfit id, palette, addon bits, autonomous walk, donor key, literal donor file.
type ActorSpec = (
    &'static str,
    &'static str,
    &'static str,
    u16,
    [u16; 4],
    u8,
    bool,
    &'static str,
    &'static str,
);
const ACTORS: &[ActorSpec] = &[
    (
        "coco",
        "62168",
        "Coco",
        1747,
        [86, 19, 110, 68],
        0,
        true,
        DAKBUGS,
        "data-otservbr-global/npc/coco.lua",
    ),
    (
        "gunther",
        "62158",
        "Gunther",
        128,
        [40, 79, 104, 49],
        0,
        true,
        DAKBUGS,
        "data-otservbr-global/npc/gunther.lua",
    ),
    (
        "kiru",
        "59877",
        "Kiru",
        1647,
        [115, 43, 20, 124],
        0,
        true,
        HAPPEN,
        "data/npc/kiru.lua",
    ),
    (
        "ra_clette",
        "50712",
        "Ra'Clette",
        533,
        [0, 113, 19, 0],
        0,
        true,
        DAKBUGS,
        "data-otservbr-global/npc/Ra'Clette.lua",
    ),
    (
        "sniff",
        "50710",
        "Sniff",
        1346,
        [38, 19, 78, 96],
        1,
        true,
        DAKBUGS,
        "data-otservbr-global/npc/sniff.lua",
    ),
    (
        "tamru",
        "59878",
        "Tamru",
        146,
        [95, 71, 128, 0],
        0,
        true,
        HAPPEN,
        "data/npc/tamru.lua",
    ),
    (
        "the_draccoon_npc",
        "61322",
        "The_Draccoon",
        1703,
        [0, 0, 0, 0],
        0,
        true,
        DAKBUGS,
        "data-otservbr-global/npc/The Draccoon.lua",
    ),
    (
        "toffee",
        "62172",
        "Toffee",
        1751,
        [0, 0, 0, 0],
        0,
        false,
        FENCORE,
        "data-otservbr-global/npc/toffee.lua",
    ),
    (
        "yerga",
        "59876",
        "Yerga",
        1648,
        [53, 76, 104, 71],
        2,
        true,
        HAPPEN,
        "data/npc/yerga.lua",
    ),
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
    reused_wiki_sources: Vec<ProjectV2Source>,
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
        return Err("duplicate nine-definition identity".into());
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
        let category = "outfit";
        let mut profile = serde_json::json!({"asset_binding":format!("canary.appearance:{category}/{}",actor.3),"light_level":0});
        let palette = ["Head", "Body", "Legs", "Feet"].iter().zip(actor.4).filter(|(_,color)|*color!=0)
            .map(|(slot, color)| serde_json::json!({"slot":slot,"asset_binding":format!("canary.appearance:palette/{color}")})).collect::<Vec<_>>();
        if !palette.is_empty() {
            profile["palette_bindings"] = serde_json::json!(palette);
        }
        let attachments = [1,2].into_iter().filter(|bit| actor.5 & *bit != 0)
            .map(|bit|serde_json::json!({"slot":"Addon","asset_binding":format!("canary.appearance:outfit/{}/addon-{bit}",actor.3)})).collect::<Vec<_>>();
        if !attachments.is_empty() {
            profile["attachment_bindings"] = serde_json::json!(attachments);
        }
        serde_json::json!({"kind":"Presentation","profile":profile})
    } else if family == ProjectV2Family::Behavior {
        // Toffee interval0 disables autonomous wander; source radius2 remains source metadata.
        let mut movement = serde_json::json!({"can_walk":actor.6,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false});
        if actor.6 {
            movement["wander"] = serde_json::json!({"interval_ms":2000,"radius_tiles":2});
        }
        serde_json::json!({"kind":"Behavior","profile":{"movement":movement,"targeting":{"hostile":false,"can_target":false,"sense_invisible":false,"target_distance_tiles":0,"static_attack_chance_ppm":0,"flee_health":0}}})
    } else {
        return Err("profile family outside scope".into());
    };
    Ok(serde_json::from_value(document)?)
}
fn source_preflight(draft: &ProjectV2Draft, packet: &Packet) -> AdmissionResult<()> {
    if packet.new_imports.len() != 3
        || packet.new_sources.len() != 3
        || packet.reused_wiki_sources.len() != 2
    {
        return Err("nine-definition exact import inventory drifted".into());
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
    for source in &packet.reused_wiki_sources {
        if source.key != "oteryn:source.tibiawiki"
            || source.evidence != ProjectV2EvidenceClass::Derived
            || !expected.iter().any(|e| {
                source.import_batch_id == e.0 && source.revision == e.1 && source.sha256 == e.2
            })
            || !seen.insert(source.revision.clone())
        {
            return Err("nine-definition reused wiki source drifted".into());
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
            return Err("nine-definition current wiki source/import fence drifted".into());
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
        let expected = SOURCE_FACTS
            .iter()
            .find(|row| row.0 == source.key)
            .ok_or("unqualified donor source")?;
        let (_, batch, repository, revision, artifact) = *expected;
        if source.evidence != ProjectV2EvidenceClass::OtsHypothesisOnly
            || source.revision != revision
            || source.sha256 != artifact
            || import.batch_id != batch
            || import.source_repository != repository
            || import.source_revision != source.revision
            || import.source_artifact_sha256 != source.sha256
            || !digest_shape(&source.sha256)
            || import.source_generation_profile != "NPC_R18_QUALIFIED_DEFINITION_SOURCE_FACTS/v1"
            || import.importer != MAPPER
            || import.mapper != MAPPER
            || import.mapper_revision != "npc-nine-qualified-definitions-r18"
            || import.mapper_sha256 != MAPPER_DIGEST
            || serde_json::to_value(import)?["access_disposition"] != "PENDING"
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
            return Err("nine-definition new source/import fence drifted".into());
        }
    }
    if SOURCE_FACTS.len() != 3
        || sources
            != SOURCE_FACTS
                .iter()
                .map(|row| (row.0.to_owned(), row.3.to_owned()))
                .collect()
    {
        return Err("nine-definition three donor snapshot closure drifted".into());
    }
    Ok(())
}
fn apply_packet(draft: &mut ProjectV2Draft, packet: Packet) -> AdmissionResult<usize> {
    if packet.schema != "OTERYN_NPC_QUALIFIED_NINE_DEFINITION_ADMISSION/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
    {
        return Err("nine-definition schema/revision drifted".into());
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
        || a.authoring_profiles.len() != 18
        || a.source_identity_bindings.len() != 27
    {
        return Err("nine-definition closed identity inventory drifted".into());
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
        return Err("nine-definition canonical identity collision".into());
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
            return Err("nine-definition declaration family".into());
        };
        let stem = identity
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("nine-definition NPC key")?;
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
            return Err("nine-definition NPC scope/reference drifted".into());
        }
    }
    for record in &a.records {
        let ProjectReferenceRecord::Generic {
            identity,
            client_projection,
        } = record
        else {
            return Err("nine-definition record family".into());
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
            return Err("nine-definition Generic family/projection drifted".into());
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
            .ok_or("nine-definition profile key")?;
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
            return Err("nine-definition profile scope/duplicate drifted".into());
        }
    }
    if profiles != profile_keys {
        return Err("nine-definition profile closure drifted".into());
    }
    let mut bindings = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for binding in &a.source_identity_bindings {
        let stem = binding
            .target
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("nine-definition binding target")?;
        let actor = ACTORS
            .iter()
            .find(|x| x.0 == stem)
            .ok_or("nine-definition binding actor")?;
        let source = packet
            .new_sources
            .iter()
            .chain(&packet.reused_wiki_sources)
            .find(|s| s.key == binding.source_key && s.revision == binding.source_revision)
            .ok_or("nine-definition binding source")?;
        let (namespace, external) = if source.key == actor.7 {
            let namespace = match source.key.as_str() {
                DAKBUGS => "dakbugs/npc-source-file",
                HAPPEN => "happen/npc-source-file",
                FENCORE => "fencore/npc-source-file",
                _ => return Err("unknown donor namespace".into()),
            };
            (namespace, actor.8.to_owned())
        } else if source.import_batch_id == "g4-npc-prices-tibiawiki-br-r1" {
            ("mediawiki/page_id", actor.1.to_owned())
        } else if source.import_batch_id == "g4-npc-prices-tibiopedia-r1" {
            (
                "tibiopedia/npc-page-url",
                format!("https://tibiopedia.pl/npcs/{}", actor.2),
            )
        } else {
            return Err("actor not registered in this donor".into());
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
            return Err("nine-definition source identity collision/drifted".into());
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
    Ok(9)
}
pub(super) fn apply(draft: &mut ProjectV2Draft) -> AdmissionResult<usize> {
    let digest =
        DIGEST.ok_or("nine-definition final packet custody pending published 1132 predecessor")?;
    if hex_sha256(PACKET) != digest {
        return Err("nine-definition packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    for (locator, expected) in [
        ("definitions/declarations.json", PREDECESSOR_ECE),
        ("definitions/reference.json", PREDECESSOR_REFERENCE),
    ] {
        let bytes = before
            .documents()
            .get(locator)
            .ok_or("missing complete nine-definition predecessor")?;
        if hex_sha256(bytes)
            != expected.ok_or(
                "nine-definition complete predecessor custody pending published 1132 predecessor",
            )?
        {
            return Err(format!("nine-definition complete predecessor digest drifted DIAG {locator} {} {}", expected.unwrap_or_default(), hex_sha256(bytes)).into());
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
        let sources = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r18/fixture-prior-sources.json")).unwrap();
        let imports = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r18/fixture-prior-imports.json")).unwrap();
        let prior: Additions=serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r18/fixture-prior-five-native.json")).unwrap();
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: FROM.to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-09-27".to_owned(),
                records: prior.records,
                imports,
                metadata: vec![],
            },
            state: ProjectV2State {
                sources,
                declarations: prior.declarations,
                authoring_profiles: prior.authoring_profiles,
                source_identity_bindings: prior.source_identity_bindings,
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
        assert_eq!(apply_packet(&mut draft, packet()).unwrap(), 9);
        assert_eq!(
            (
                draft.state.declarations.len(),
                draft.core.records.len(),
                draft.state.authoring_profiles.len(),
                draft.state.source_identity_bindings.len()
            ),
            (14, 28, 28, 42)
        );
        assert_eq!(draft.state.placements, before.state.placements);
        assert_eq!(draft.state.worlds, before.state.worlds);
        assert_eq!(draft.state.editor, before.state.editor);
        assert_eq!(draft.core.metadata, before.core.metadata);
        assert_eq!(&draft.state.sources[..4], before.state.sources.as_slice());
        assert_eq!(&draft.core.imports[..4], before.core.imports.as_slice());
        assert_eq!(
            &draft.state.authoring_profiles[..10],
            before.state.authoring_profiles.as_slice()
        );
        assert_eq!(
            &draft.state.declarations[..5],
            before.state.declarations.as_slice()
        );
        assert_eq!(&draft.core.records[..10], before.core.records.as_slice());
        assert_eq!(
            &draft.state.source_identity_bindings[..15],
            before.state.source_identity_bindings.as_slice()
        );
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
        match DIGEST {
            Some(digest) => {
                assert_eq!(hex_sha256(PACKET), digest);
                assert!(PREDECESSOR_ECE.is_some());
                assert!(PREDECESSOR_REFERENCE.is_some());
            }
            None => {
                assert!(PREDECESSOR_ECE.is_none());
                assert!(PREDECESSOR_REFERENCE.is_none());
            }
        }
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
                    .find(|p| p.target.key == "oteryn:behavior.npc.coco")
                    .unwrap();
                let mut value = serde_json::to_value(&profile.data).unwrap();
                value["profile"]["movement"]["wander"]["radius_tiles"] = serde_json::json!(3);
                profile.data = serde_json::from_value(value).unwrap();
            } else if let ProjectV2Declaration::Npc { presentation, .. } =
                &mut p.native_additions.declarations[0]
            {
                presentation.as_mut().unwrap().key = "oteryn:presentation.npc.yselda".to_owned();
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
    fn new_import_mapper_generation_and_donor_revision_drift_reject_atomically() {
        for operation in 0..3 {
            let mut draft = fixture();
            let mut p = packet();
            match operation {
                0 => p.new_imports[0].mapper_sha256 = "0".repeat(64),
                1 => p.new_imports[1].source_generation_profile = "unrelated-profile".to_owned(),
                _ => p.new_sources[1].revision = "stale-donor-revision".to_owned(),
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
            *client_projection = if *client_projection == ProjectionDocument::ClientSafe {
                ProjectionDocument::ServerOnly
            } else {
                ProjectionDocument::ClientSafe
            };
        }
        rejects(&mut draft, p);
        let mut draft = fixture();
        let mut p = packet();
        p.native_additions.authoring_profiles[1] = p.native_additions.authoring_profiles[0].clone();
        rejects(&mut draft, p);
    }
    #[test]
    fn zero_interval_toffee_never_gains_autonomous_wander() {
        let mut draft = fixture();
        let mut p = packet();
        let profile = p
            .native_additions
            .authoring_profiles
            .iter_mut()
            .find(|p| p.target.key == named("behavior", "toffee"))
            .unwrap();
        let mut value = serde_json::to_value(&profile.data).unwrap();
        value["profile"]["movement"]["wander"] =
            serde_json::json!({"interval_ms":2000,"radius_tiles":2});
        profile.data = serde_json::from_value(value).unwrap();
        rejects(&mut draft, p);
    }
    #[test]
    fn foreign_donor_actor_and_npc_dialogue_injection_reject_atomically() {
        let mut draft = fixture();
        let mut p = packet();
        let binding = p
            .native_additions
            .source_identity_bindings
            .iter_mut()
            .find(|b| b.source_key == FENCORE)
            .unwrap();
        binding.target.key = "oteryn:npc.coco".to_owned();
        rejects(&mut draft, p);
        let mut draft = fixture();
        let mut p = packet();
        if let ProjectV2Declaration::Npc { dialogue, .. } = &mut p.native_additions.declarations[0]
        {
            *dialogue = Some(ProjectV2DefinitionRef {
                family: ProjectV2Family::Dialogue,
                key: "oteryn:dialogue.npc.coco".to_owned(),
                revision: "definition-r1".to_owned(),
            });
        }
        rejects(&mut draft, p);
    }
    #[test]
    fn frozen_addons_and_small_visible_region_source_color_reject_drift() {
        for (actor, operation) in [("coco", 0), ("yerga", 1), ("coco", 2)] {
            let mut draft = fixture();
            let mut p = packet();
            let profile = p
                .native_additions
                .authoring_profiles
                .iter_mut()
                .find(|p| p.target.key == named("presentation", actor))
                .unwrap();
            let mut value = serde_json::to_value(&profile.data).unwrap();
            match operation {
                0 => {
                    value["profile"]["attachment_bindings"] = serde_json::json!([{"slot":"Addon","asset_binding":"canary.appearance:outfit/1747/addon-2"}])
                }
                1 => value["profile"]["attachment_bindings"] = serde_json::json!([]),
                _ => {
                    let rows = value["profile"]["palette_bindings"].as_array_mut().unwrap();
                    let feet = rows.iter_mut().find(|row| row["slot"] == "Feet").unwrap();
                    feet["asset_binding"] = serde_json::json!("canary.appearance:palette/38");
                }
            }
            profile.data = serde_json::from_value(value).unwrap();
            rejects(&mut draft, p);
        }
    }
    #[test]
    fn omitted_source_mount_never_gains_native_mount_binding() {
        for actor in [
            "coco",
            "tamru",
            "yerga",
            "the_draccoon_npc",
            "toffee",
            "gunther",
        ] {
            let mut draft = fixture();
            let mut p = packet();
            let profile = p
                .native_additions
                .authoring_profiles
                .iter_mut()
                .find(|p| p.target.key == named("presentation", actor))
                .unwrap();
            let mut value = serde_json::to_value(&profile.data).unwrap();
            value["profile"]["attachment_bindings"] = serde_json::json!([{"slot":"Mount","asset_binding":"canary.appearance:outfit/368"}]);
            profile.data = serde_json::from_value(value).unwrap();
            rejects(&mut draft, p);
        }
    }
}
