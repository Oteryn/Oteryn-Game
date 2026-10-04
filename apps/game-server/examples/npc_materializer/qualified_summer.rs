//! Bounded, offline authoring admission of twelve qualified summer NPCs.
use super::*;
use oteryn_game_server::content::ProjectV2DialogueKeyword;
use serde::Serialize;

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r13/native-additions.json"
);
const DIGEST: &str = "1c1f7b4b19fd807e2760df9e015a7c780d9e7545e204dd37f3b0ca3f4260dc69";
const FROM: &str = "g4-npc-service-scope-r13";
const TO: &str = "g4-npc-qualified-summer-r14";
const NPCS: &[&str] = &[
    "dhira",
    "g_ezkho",
    "goldro",
    "nekaret",
    "nilavarna",
    "niral",
    "omar",
    "saraki",
    "sharai",
    "tarisu",
    "udu",
    "zofia_bolter",
];
const SPOKEN: &[&str] = &[
    "dhira",
    "nilavarna",
    "niral",
    "saraki",
    "sharai",
    "tarisu",
    "udu",
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
    reused_crystal_import: ProjectV2Source,
}

fn definition_key<T: Serialize>(value: &T) -> AdmissionResult<String> {
    // All typed declaration/reference variants serialize their identity here.
    let document = serde_json::to_value(value)?;
    Ok(document["identity"]["key"]
        .as_str()
        .ok_or("missing identity key")?
        .to_owned())
}
fn keys<T: Serialize>(values: &[T]) -> AdmissionResult<BTreeSet<String>> {
    let set = values
        .iter()
        .map(definition_key)
        .collect::<AdmissionResult<BTreeSet<_>>>()?;
    if set.len() != values.len() {
        return Err("duplicate summer identity".into());
    }
    Ok(set)
}
fn named(family: &str, stem: &str) -> String {
    format!("oteryn:{family}.npc.{stem}")
}
fn node_count(nodes: &[ProjectV2DialogueKeyword]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node_count(&node.children))
        .sum()
}

fn binding_key(b: &ProjectV2SourceIdentityBinding) -> (&str, &str, &str, &str) {
    (
        &b.source_key,
        &b.source_revision,
        &b.identity_namespace,
        &b.external_id,
    )
}

fn apply_packet(draft: &mut ProjectV2Draft, packet: Packet) -> AdmissionResult<usize> {
    if packet.schema != "OTERYN_NPC_QUALIFIED_SUMMER_ADMISSION/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
        || packet.new_imports.len() != 1
        || packet.new_sources.len() != 1
    {
        return Err("summer schema/revision/import count drifted".into());
    }
    let sources = &draft.state.sources;
    let imports = &draft.core.imports;
    let crystal = &packet.reused_crystal_import;
    let wiki = &packet.new_sources[0];
    let import = &packet.new_imports[0];
    let matching_sources = sources
        .iter()
        .filter(|s| s.key == crystal.key && s.revision == crystal.revision)
        .collect::<Vec<_>>();
    let matching_imports = imports
        .iter()
        .filter(|i| i.batch_id == crystal.import_batch_id)
        .collect::<Vec<_>>();
    let valid_import = matching_imports.len() == 1
        && matching_imports[0].source_revision == crystal.revision
        && matching_imports[0].source_artifact_sha256 == crystal.sha256;
    if crystal.key != "oteryn:source.crystalserver"
        || crystal.revision != "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
        || matching_sources != vec![crystal]
        || !valid_import
        || wiki.key != "oteryn:source.tibiawiki"
        || wiki.import_batch_id != import.batch_id
        || wiki.revision != import.source_revision
        || wiki.sha256 != import.source_artifact_sha256
        || import.batch_id != "g4-npc-summer-definition-overlay-fandom-r13"
        || !import.candidates.is_empty()
        || !import.reimport_states.is_empty()
        || imports.iter().any(|i| i.batch_id == import.batch_id)
        || sources
            .iter()
            .any(|s| s.key == wiki.key && s.revision == wiki.revision)
    {
        return Err("summer source/import fence drifted".into());
    }
    let additions = packet.native_additions;
    let declarations = &draft.state.declarations;
    let old_profiles = &draft.state.authoring_profiles;
    let old_bindings = &draft.state.source_identity_bindings;
    let mut expected = NPCS
        .iter()
        .map(|s| format!("oteryn:npc.{s}"))
        .collect::<BTreeSet<_>>();
    expected.extend(SPOKEN.iter().map(|s| named("dialogue", s)));
    let record_keys = NPCS
        .iter()
        .flat_map(|s| [named("presentation", s), named("behavior", s)])
        .collect::<BTreeSet<_>>();
    if keys(&additions.declarations)? != expected
        || keys(&additions.records)? != record_keys
        || additions.authoring_profiles.len() != 24
        || additions.source_identity_bindings.len() != 24
    {
        return Err("summer closed identity inventory drifted".into());
    }
    let existing = declarations
        .iter()
        .map(definition_key)
        .chain(draft.core.records.iter().map(definition_key))
        .collect::<AdmissionResult<BTreeSet<_>>>()?;
    if existing
        .iter()
        .any(|key| expected.contains(key) || record_keys.contains(key))
        || old_profiles
            .iter()
            .any(|p| record_keys.contains(&p.target.key))
    {
        return Err("summer canonical identity already exists".into());
    }
    let mut nodes = 0;
    for declaration in &additions.declarations {
        match declaration {
            ProjectV2Declaration::Npc {
                identity,
                services,
                fields,
                ..
            } => {
                if !identity.key.starts_with("oteryn:npc.")
                    || identity.revision != "definition-r1"
                    || !services.is_empty()
                    || !fields.is_empty()
                {
                    return Err("summer NPC scope drifted".into());
                }
            }
            ProjectV2Declaration::Dialogue {
                identity,
                keywords,
                send_trade,
                fields,
                ..
            } => {
                if identity.revision != "definition-r1"
                    || !send_trade.is_empty()
                    || !fields.is_empty()
                {
                    return Err("summer static dialogue scope drifted".into());
                }
                nodes += node_count(keywords);
            }
            _ => return Err("summer declaration family is outside scope".into()),
        }
    }
    if nodes != 43 {
        return Err("summer static node count drifted".into());
    }
    let profiles = additions
        .authoring_profiles
        .iter()
        .map(|p| p.target.key.clone())
        .collect::<BTreeSet<_>>();
    if profiles != record_keys {
        return Err("summer profile closure drifted".into());
    }
    let mut bindings = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for binding in &additions.source_identity_bindings {
        let source = [crystal, wiki]
            .into_iter()
            .find(|s| s.key == binding.source_key)
            .ok_or("summer binding source")?;
        let stem = binding
            .target
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("summer binding target")?;
        let id = binding_key(binding);
        if binding.source_key != source.key
            || binding.source_revision != source.revision
            || !NPCS.contains(&stem)
            || binding.target.family != ProjectV2Family::Npc
            || binding.target.revision != "definition-r1"
            || binding.disposition != ProjectV2SourceIdentityDisposition::Exact
            || (source == crystal && binding.external_id != stem)
            || !bindings.insert(id)
            || !targets.insert((source.key.clone(), binding.target.key.clone()))
            || old_bindings
                .iter()
                .any(|old| binding_key(old) == id || old.target.key == binding.target.key)
        {
            return Err("summer source identity binding collision/drift".into());
        }
    }
    // No mutation precedes the complete typed identity, provenance and closure preflight.
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
    Ok(12)
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> AdmissionResult<usize> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("summer packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    for (locator, expected) in [
        (
            "definitions/declarations.json",
            "f9da1e09635faa0174114e77a4b5ea62f734da8302c1031f91212b4b50e9e487",
        ),
        (
            "definitions/reference.json",
            "3791f0142bcc3bfa0395153c3f40a84e971b47c38eaa301b07e2dc49cc5b49fc",
        ),
    ] {
        let bytes = before
            .documents()
            .get(locator)
            .ok_or("missing summer predecessor")?;
        if hex_sha256(bytes) != expected {
            return Err("summer complete predecessor digest drifted".into());
        }
    }
    apply_packet(draft, serde_json::from_slice(PACKET)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Packet {
        serde_json::from_slice(PACKET).expect("qualified packet")
    }
    fn fixture() -> ProjectV2Draft {
        let source = packet().reused_crystal_import;
        let imports: Value = serde_json::from_slice(include_bytes!(
            "../../../../content/world/provenance/imports.json"
        ))
        .unwrap();
        let batches = imports["batches"].as_array().unwrap();
        let import = batches
            .iter()
            .find(|i| i["batch_id"] == source.import_batch_id)
            .unwrap();
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: FROM.to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-09-27".to_owned(),
                records: vec![],
                imports: vec![serde_json::from_value(import.clone()).unwrap()],
                metadata: vec![],
            },
            state: ProjectV2State {
                sources: vec![source],
                ..ProjectV2State::default()
            },
        }
    }
    #[test]
    fn complete_predecessor_wrapper_rejects_substituted_package_atomically() {
        let mut draft = fixture();
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        assert_eq!(hex_sha256(PACKET), DIGEST);
    }
    #[test]
    fn admits_only_closed_authoring_scope_and_preserves_other_state() {
        let mut draft = fixture();
        let before = draft.clone();
        assert_eq!(apply_packet(&mut draft, packet()).unwrap(), 12);
        assert_eq!(draft.state.declarations.len(), 19);
        assert_eq!(draft.core.records.len(), 24);
        assert_eq!(draft.state.authoring_profiles.len(), 24);
        assert_eq!(draft.state.source_identity_bindings.len(), 24);
        assert_eq!(draft.state.placements, before.state.placements);
        assert_eq!(draft.state.worlds, before.state.worlds);
        assert_eq!(draft.state.editor, before.state.editor);
        assert_eq!(draft.core.metadata, before.core.metadata);
        assert_eq!(draft.state.sources[0], before.state.sources[0]);
        assert_eq!(draft.core.imports[0], before.core.imports[0]);
        assert_eq!(draft.core.project_revision, TO);
        let before = draft.clone();
        assert!(apply_packet(&mut draft, packet()).is_err());
        assert_eq!(draft, before);
    }
    #[test]
    fn missing_or_substituted_source_and_import_reject_atomically() {
        for operation in 0..3 {
            let mut draft = fixture();
            match operation {
                0 => draft.state.sources.clear(),
                1 => draft.state.sources[0].sha256 = "wrong".to_owned(),
                _ => draft.core.imports.clear(),
            }
            let before = draft.clone();
            assert!(apply_packet(&mut draft, packet()).is_err());
            assert_eq!(draft, before);
        }
    }
    #[test]
    fn all_identity_collision_lanes_reject_atomically_even_with_other_revision() {
        for operation in 0..6 {
            let mut draft = fixture();
            let mut p = packet();
            let additions = &mut p.native_additions;
            let state = &mut draft.state;
            match operation {
                0 => state
                    .declarations
                    .push(additions.declarations.last().unwrap().clone()),
                1 => draft
                    .core
                    .records
                    .push(additions.records.last().unwrap().clone()),
                2 => {
                    let mut v = additions.authoring_profiles.last().unwrap().clone();
                    v.target.revision = "other".to_owned();
                    state.authoring_profiles.push(v);
                }
                3 => {
                    let binding = additions.source_identity_bindings.last().unwrap().clone();
                    state.source_identity_bindings.push(binding);
                }
                4 => draft
                    .core
                    .imports
                    .push(p.new_imports.last().unwrap().clone()),
                _ => {
                    let mut binding = additions.source_identity_bindings.last().unwrap().clone();
                    binding.external_id = "other".to_owned();
                    state.source_identity_bindings.push(binding);
                }
            }
            let before = draft.clone();
            assert!(apply_packet(&mut draft, packet()).is_err());
            assert_eq!(draft, before);
        }
    }
    #[test]
    fn revision_and_late_binding_drift_reject_before_any_extension() {
        let mut draft = fixture();
        draft.core.project_revision = "wrong".to_owned();
        let before = draft.clone();
        assert!(apply_packet(&mut draft, packet()).is_err());
        assert_eq!(draft, before);
        let mut draft = fixture();
        let mut p = packet();
        let binding = p
            .native_additions
            .source_identity_bindings
            .last_mut()
            .unwrap();
        binding.source_revision = "wrong".to_owned();
        let before = draft.clone();
        assert!(apply_packet(&mut draft, p).is_err());
        assert_eq!(draft, before);
    }
}
