//! Bounded offline authoring admission of eight source-qualified structural NPC definitions.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;
use serde::Serialize;

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/native-additions.json"
);
// Exact published 1141 predecessor and frozen source packet; root alone integrates this offline authoring stage.
const DIGEST: Option<&str> =
    Some("66034fba2019f2eefe0d793275c87dc56920e52dabc26f65860290daa373e2d2");
const PREDECESSOR_ECE: Option<&str> =
    Some("4068e0abf18068b75b682d2d554c36e7a31d0aa306110a50e09de32c6cc0a1d3");
const PREDECESSOR_REFERENCE: Option<&str> =
    Some("0b1a1af3ab3fb2b1cd187b557e59796039a9f350af5c0a902a8b1c6303497cb7");
const PREDECESSOR_TREE: &str = "e3cd78fc9002cd127e5066aaf379408bc118c884f23ece7d8bcafa91bfa7681c";
const MAPPER: &str = "NPC_BOUNDED_D15_D16_STRUCTURAL_DEFINITION_BRIDGE/v1";
const MAPPER_DIGEST: &str = "3f98bbdd656be5a4e3ee0a0f9bcfaad050605cfbdf5aaa537d6c66518edc8492";
const FROM: &str = "g4-npc-qualified-nine-definitions-r17";
const TO: &str = "g4-npc-qualified-bounded-definitions-r18";
const DAKBUGS: &str = "oteryn:source.dakbugs";
const PLAYERBOTS: &str = "oteryn:source.tibia_playerbots_project";
const OTG: &str = "oteryn:source.otg_br_global_11x";
const NEXA: &str = "oteryn:source.nexa_map_editor";
const VALERIA: &str = "oteryn:source.valeria_ot";
// Source snapshot revisions are derived; pinned upstream Git references remain in qualified artifacts.
const SOURCE_FACTS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "oteryn:source.tibia_playerbots_project",
        "g4-npc-r19-definition-tibia_playerbots_project",
        "adrunkhuman/tibia-playerbots-project",
        "npc-definition-r19-24ea516b447781bd",
        "24ea516b447781bd5acf914544ba46580eddf2ac71945450fa8af3458c15f762",
    ),
    (
        "oteryn:source.otg_br_global_11x",
        "g4-npc-r19-definition-otg_br_global_11x",
        "otg-br/global-11x",
        "npc-definition-r19-edaf99beb22fe65b",
        "edaf99beb22fe65b578b8792f5e4ea80d8d4ef79c77fd997469061a121522130",
    ),
    (
        "oteryn:source.dakbugs",
        "g4-npc-r19-definition-dakbugs",
        "dakotaotserver-glitch/dakbugs",
        "npc-definition-r19-f750ebace2d53e91",
        "f750ebace2d53e911515ad6d18d0a3c8b4c431d7a4db31e45222ee72f641733c",
    ),
    (
        "oteryn:source.nexa_map_editor",
        "g4-npc-r19-definition-nexa_map_editor",
        "Mateuzkl/NexaMap-Editor",
        "npc-definition-r19-caaac8ba60b7dffc",
        "caaac8ba60b7dffcb1b049bd42a35cfe568b8186d2bf0de6914beb97f697e73f",
    ),
    (
        "oteryn:source.valeria_ot",
        "g4-npc-r19-definition-valeria_ot",
        "ValeriaOT/Test",
        "npc-definition-r19-57ae5b848ff72f02",
        "57ae5b848ff72f02d35b49556c6e210a2b4f57b339785284be5852c8b229f9c3",
    ),
];
// stem, exact BR/TP identities, appearance, optional palette/addons, source walk interval, literal donor.
type ActorSpec = (
    &'static str,
    &'static str,
    &'static str,
    u16,
    Option<[u16; 4]>,
    Option<u8>,
    u32,
    &'static str,
    &'static str,
);
const ACTORS: &[ActorSpec] = &[
    (
        "arenamaster",
        "46220",
        "Arenamaster",
        63,
        None,
        None,
        0,
        OTG,
        "data/npc/Arenamaster.xml",
    ),
    (
        "ben",
        "12392",
        "Ben",
        16,
        None,
        None,
        1500,
        PLAYERBOTS,
        "server/data/npc/Ben.xml",
    ),
    (
        "gerib",
        "59875",
        "Gerib",
        1646,
        Some([78, 86, 113, 94]),
        Some(0),
        2000,
        DAKBUGS,
        "data-otservbr-global/npc/Gerib.lua",
    ),
    (
        "lai",
        "63414",
        "Lai",
        1817,
        Some([79, 0, 79, 94]),
        Some(3),
        0,
        DAKBUGS,
        "data-otservbr-global/npc/lai.lua",
    ),
    (
        "alberto",
        "38895",
        "Alberto",
        133,
        Some([21, 0, 1, 20]),
        Some(1),
        2000,
        VALERIA,
        "data-otservbr-global/npc/drome_npc_kazordoon.lua",
    ),
    (
        "emilio",
        "38710",
        "Emilio",
        133,
        Some([21, 123, 29, 57]),
        Some(1),
        2000,
        VALERIA,
        "data-otservbr-global/npc/drome_edron_npc.lua",
    ),
    (
        "fabiana",
        "38897",
        "Fabiana",
        141,
        Some([130, 75, 76, 76]),
        Some(0),
        2000,
        VALERIA,
        "data-otservbr-global/npc/drome_npc_rathleton.lua",
    ),
    (
        "lorenzo",
        "38256",
        "Lorenzo",
        133,
        Some([57, 60, 21, 22]),
        Some(1),
        2000,
        VALERIA,
        "data-otservbr-global/npc/drome_npc_thais.lua",
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
        return Err("duplicate bounded-definition identity".into());
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
        let palette = ["Head", "Body", "Legs", "Feet"].iter().zip(actor.4.unwrap_or([0;4])).filter(|(_,color)|*color!=0)
            .map(|(slot, color)| serde_json::json!({"slot":slot,"asset_binding":format!("canary.appearance:palette/{color}")})).collect::<Vec<_>>();
        if !palette.is_empty() {
            profile["palette_bindings"] = serde_json::json!(palette);
        }
        let attachments = [1,2].into_iter().filter(|bit| actor.5.unwrap_or(0) & *bit != 0)
            .map(|bit|serde_json::json!({"slot":"Addon","asset_binding":format!("canary.appearance:outfit/{}/addon-{bit}",actor.3)})).collect::<Vec<_>>();
        if !attachments.is_empty() {
            profile["attachment_bindings"] = serde_json::json!(attachments);
        }
        serde_json::json!({"kind":"Presentation","profile":profile})
    } else if family == ProjectV2Family::Behavior {
        // Source zero interval disables autonomous wander; source radii remain documentary metadata.
        let mut movement = serde_json::json!({"can_walk":actor.6>0,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false});
        if actor.6 > 0 {
            movement["wander"] = serde_json::json!({"interval_ms":actor.6,"radius_tiles":2});
        }
        serde_json::json!({"kind":"Behavior","profile":{"movement":movement,"targeting":{"hostile":false,"can_target":false,"sense_invisible":false,"target_distance_tiles":0,"static_attack_chance_ppm":0,"flee_health":0}}})
    } else {
        return Err("profile family outside scope".into());
    };
    Ok(serde_json::from_value(document)?)
}
fn source_preflight(draft: &ProjectV2Draft, packet: &Packet) -> AdmissionResult<()> {
    if packet.new_imports.len() != 5
        || packet.new_sources.len() != 5
        || packet.reused_wiki_sources.len() != 2
    {
        return Err("bounded-definition exact import inventory drifted".into());
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
            return Err("bounded-definition reused wiki source drifted".into());
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
            return Err("bounded-definition current wiki source/import fence drifted".into());
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
            || import.source_generation_profile != "NPC_R19_QUALIFIED_STRUCTURAL_SOURCE_FACTS/v1"
            || import.importer != MAPPER
            || import.mapper != MAPPER
            || import.mapper_revision != "npc-structural-definitions-r19"
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
            return Err("bounded-definition new source/import fence drifted".into());
        }
    }
    if SOURCE_FACTS.len() != 5
        || sources
            != SOURCE_FACTS
                .iter()
                .map(|row| (row.0.to_owned(), row.3.to_owned()))
                .collect()
    {
        return Err("bounded-definition five donor snapshot closure drifted".into());
    }
    Ok(())
}
fn apply_packet(draft: &mut ProjectV2Draft, packet: Packet) -> AdmissionResult<usize> {
    if packet.schema != "OTERYN_NPC_QUALIFIED_BOUNDED_DEFINITION_ADMISSION/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
    {
        return Err("bounded-definition schema/revision drifted".into());
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
        || a.authoring_profiles.len() != 16
        || a.source_identity_bindings.len() != 25
    {
        return Err("bounded-definition closed identity inventory drifted".into());
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
        return Err("bounded-definition canonical identity collision".into());
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
            return Err("bounded-definition declaration family".into());
        };
        let stem = identity
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("bounded-definition NPC key")?;
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
            return Err("bounded-definition NPC scope/reference drifted".into());
        }
    }
    for record in &a.records {
        let ProjectReferenceRecord::Generic {
            identity,
            client_projection,
        } = record
        else {
            return Err("bounded-definition record family".into());
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
            return Err("bounded-definition Generic family/projection drifted".into());
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
            .ok_or("bounded-definition profile key")?;
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
            return Err("bounded-definition profile scope/duplicate drifted".into());
        }
    }
    if profiles != profile_keys {
        return Err("bounded-definition profile closure drifted".into());
    }
    let mut bindings = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for binding in &a.source_identity_bindings {
        let stem = binding
            .target
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("bounded-definition binding target")?;
        let actor = ACTORS
            .iter()
            .find(|x| x.0 == stem)
            .ok_or("bounded-definition binding actor")?;
        let source = packet
            .new_sources
            .iter()
            .chain(&packet.reused_wiki_sources)
            .find(|s| s.key == binding.source_key && s.revision == binding.source_revision)
            .ok_or("bounded-definition binding source")?;
        let (namespace, external) = if source.key == actor.7 {
            let namespace = match source.key.as_str() {
                DAKBUGS => "dakbugs/npc-source-file",
                PLAYERBOTS => "tibia_playerbots_project/npc-source-file",
                OTG => "otg_br_global_11x/npc-source-file",
                VALERIA => "valeria_ot/npc-source-file",
                _ => return Err("unknown donor namespace".into()),
            };
            (namespace, actor.8.to_owned())
        } else if source.key == NEXA && stem == "arenamaster" {
            ("rme/npc-name", "Arenamaster".to_owned())
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
            return Err("bounded-definition source identity collision/drifted".into());
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
    Ok(8)
}
pub(super) fn apply(draft: &mut ProjectV2Draft) -> AdmissionResult<usize> {
    let digest = DIGEST
        .ok_or("bounded-definition final packet custody pending published 1141 predecessor")?;
    if hex_sha256(PACKET) != digest {
        return Err("bounded-definition packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    for (locator, expected) in [
        ("definitions/declarations.json", PREDECESSOR_ECE),
        ("definitions/reference.json", PREDECESSOR_REFERENCE),
    ] {
        let bytes = before
            .documents()
            .get(locator)
            .ok_or("missing complete bounded-definition predecessor")?;
        if hex_sha256(bytes) != expected.ok_or("bounded-definition complete predecessor custody pending published 1141 predecessor")? {
            return Err("bounded-definition complete predecessor digest drifted".into());
        }
    }
    let mut tree = sha2::Sha256::new();
    for (locator, bytes) in before.documents() {
        tree.update((locator.len() as u64).to_be_bytes());
        tree.update(locator.as_bytes());
        tree.update((bytes.len() as u64).to_be_bytes());
        tree.update(bytes);
    }
    let actual = tree
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != PREDECESSOR_TREE {
        return Err("bounded-definition complete canonical tree drifted".into());
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
        let sources = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/fixture-prior-sources.json")).unwrap();
        let imports = serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/fixture-prior-imports.json")).unwrap();
        let prior: Additions=serde_json::from_slice(include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/fixture-prior-five-native.json")).unwrap();
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
        assert_eq!(apply_packet(&mut draft, packet()).unwrap(), 8);
        assert_eq!(
            (
                draft.state.declarations.len(),
                draft.core.records.len(),
                draft.state.authoring_profiles.len(),
                draft.state.source_identity_bindings.len()
            ),
            (13, 26, 26, 40)
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
                    .find(|p| p.target.key == "oteryn:behavior.npc.ben")
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
    fn zero_interval_lai_never_gains_autonomous_wander() {
        let mut draft = fixture();
        let mut p = packet();
        let profile = p
            .native_additions
            .authoring_profiles
            .iter_mut()
            .find(|p| p.target.key == named("behavior", "lai"))
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
            .find(|b| b.source_key == OTG)
            .unwrap();
        binding.target.key = "oteryn:npc.ben".to_owned();
        rejects(&mut draft, p);
        let mut draft = fixture();
        let mut p = packet();
        if let ProjectV2Declaration::Npc { dialogue, .. } = &mut p.native_additions.declarations[0]
        {
            *dialogue = Some(ProjectV2DefinitionRef {
                family: ProjectV2Family::Dialogue,
                key: "oteryn:dialogue.npc.ben".to_owned(),
                revision: "definition-r1".to_owned(),
            });
        }
        rejects(&mut draft, p);
    }
    #[test]
    fn unknown_single_layer_palette_and_selector_stay_omitted() {
        for actor in ["ben", "arenamaster"] {
            for field in ["palette_bindings", "attachment_bindings"] {
                let mut draft = fixture();
                let mut p = packet();
                let profile = p
                    .native_additions
                    .authoring_profiles
                    .iter_mut()
                    .find(|p| p.target.key == named("presentation", actor))
                    .unwrap();
                let mut value = serde_json::to_value(&profile.data).unwrap();
                value["profile"][field] = if field == "palette_bindings" {
                    serde_json::json!([{"slot":"Head","asset_binding":"canary.appearance:palette/79"}])
                } else {
                    serde_json::json!([{"slot":"Addon","asset_binding":format!("canary.appearance:outfit/{}/addon-1",if actor=="ben" {16} else {63})}])
                };
                profile.data = serde_json::from_value(value).unwrap();
                rejects(&mut draft, p);
            }
        }
    }
    #[test]
    fn retained_gerib_feet_and_corrected_lai_addons_are_closed() {
        for actor in ["gerib", "lai"] {
            let mut draft = fixture();
            let mut p = packet();
            let profile = p
                .native_additions
                .authoring_profiles
                .iter_mut()
                .find(|p| p.target.key == named("presentation", actor))
                .unwrap();
            let mut value = serde_json::to_value(&profile.data).unwrap();
            if actor == "lai" {
                value["profile"]["attachment_bindings"] = serde_json::json!([]);
            } else {
                let feet = value["profile"]["palette_bindings"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["slot"] == "Feet")
                    .unwrap();
                feet["asset_binding"] = serde_json::json!("canary.appearance:palette/95");
            }
            profile.data = serde_json::from_value(value).unwrap();
            rejects(&mut draft, p);
        }
    }
    #[test]
    fn extra_sixth_import_is_rejected_before_mutation() {
        let mut draft = fixture();
        let mut p = packet();
        let mut extra = p.new_imports[0].clone();
        extra.batch_id = "g4-npc-extra-import-r19".to_owned();
        p.new_imports.push(extra);
        rejects(&mut draft, p);
    }
    #[test]
    fn new_valeria_actor_palette_and_walk_interval_reject_drift() {
        for actor in ["alberto", "emilio", "fabiana", "lorenzo"] {
            for family in ["presentation", "behavior"] {
                let mut draft = fixture();
                let mut p = packet();
                let profile = p
                    .native_additions
                    .authoring_profiles
                    .iter_mut()
                    .find(|p| p.target.key == named(family, actor))
                    .unwrap();
                let mut value = serde_json::to_value(&profile.data).unwrap();
                if family == "presentation" {
                    value["profile"]["palette_bindings"][0]["asset_binding"] =
                        serde_json::json!("canary.appearance:palette/99");
                } else {
                    value["profile"]["movement"]["wander"]["interval_ms"] = serde_json::json!(1500);
                }
                profile.data = serde_json::from_value(value).unwrap();
                rejects(&mut draft, p);
            }
        }
    }
    #[test]
    fn mount_omission_and_old_arena_appearance_are_closed() {
        for actor in ["ben", "arenamaster", "gerib", "lai"] {
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
        let mut draft = fixture();
        let mut p = packet();
        let profile = p
            .native_additions
            .authoring_profiles
            .iter_mut()
            .find(|p| p.target.key == named("presentation", "arenamaster"))
            .unwrap();
        let mut value = serde_json::to_value(&profile.data).unwrap();
        value["profile"]["asset_binding"] = serde_json::json!("canary.appearance:outfit/990");
        profile.data = serde_json::from_value(value).unwrap();
        rejects(&mut draft, p);
    }
}
