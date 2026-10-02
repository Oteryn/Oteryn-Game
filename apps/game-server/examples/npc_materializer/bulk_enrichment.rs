//! Closed successor enrichment of provisional authoring; no service or runtime activation.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    from_project_revision: String,
    project_revision: String,
    repairs: Vec<Repair>,
    profile_repairs: Vec<ProfileRepair>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Repair {
    before: ProjectV2Declaration,
    after: ProjectV2Declaration,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRepair {
    before: ProjectV2AuthoringProfile,
    after: ProjectV2AuthoringProfile,
}
fn actors() -> Result<BTreeSet<String>> {
    let mut keys = BTreeSet::new();
    for bytes in [NPC_BULK, NPC_BULK_MORE] {
        let packet: Value = serde_json::from_slice(bytes)?;
        for row in packet["native_additions"]["declarations"]
            .as_array()
            .ok_or("enrichment parent inventory")?
        {
            if row["kind"] == "NPC" {
                let key = row["identity"]["key"]
                    .as_str()
                    .ok_or("enrichment actor key")?;
                if !keys.insert(key.to_owned()) {
                    return Err("duplicate parent actor".into());
                }
            }
        }
    }
    if keys.len() != 133 {
        return Err("enrichment closed parent inventory".into());
    }
    Ok(keys)
}
fn fields_allowed(before: &[ProjectV2CandidateField], after: &[ProjectV2CandidateField]) -> bool {
    let mutable = [
        "quality",
        "profession",
        "voices",
        "source_reference",
        "source_metadata",
        "wiki_positions",
        "wiki_trade_reference",
        "wiki_quests",
        "wiki_services",
        "wiki_profession",
        "wiki_voices",
        "dialogue_source",
        "dialogue_reference",
    ];
    let mut seen = BTreeSet::new();
    after.iter().all(|f| {
        seen.insert(&f.field_path)
            && (before.contains(f)
                || (matches!(f.value, ProjectV2CandidateValue::Text(_))
                    && f.field_path
                        .strip_prefix("oteryn:source.npc.bulk.")
                        .is_some_and(|s| mutable.contains(&s))))
    }) && before
        .iter()
        .all(|f| after.iter().any(|n| n.field_path == f.field_path))
}
fn scope(
    before: &ProjectV2Declaration,
    after: &ProjectV2Declaration,
    actors: &BTreeSet<String>,
) -> bool {
    let mut allowed = before.clone();
    match (&mut allowed, after) {
        (
            ProjectV2Declaration::Npc {
                identity,
                services,
                fields,
                ..
            },
            ProjectV2Declaration::Npc { fields: next, .. },
        ) => {
            if !actors.contains(&identity.key)
                || identity.revision != "definition-r1"
                || !services.is_empty()
                || !fields_allowed(fields, next)
            {
                return false;
            }
            fields.clone_from(next);
        }
        (
            ProjectV2Declaration::Dialogue {
                identity,
                greet,
                farewell,
                keywords,
                fields,
                ..
            },
            ProjectV2Declaration::Dialogue {
                greet: g,
                farewell: f,
                keywords: k,
                fields: n,
                ..
            },
        ) => {
            let Some(stem) = identity.key.strip_prefix("oteryn:dialogue.npc.") else {
                return false;
            };
            if !actors.contains(&format!("oteryn:npc.{stem}"))
                || identity.revision != "definition-r1"
                || !fields_allowed(fields, n)
            {
                return false;
            }
            greet.clone_from(g);
            farewell.clone_from(f);
            keywords.clone_from(k);
            fields.clone_from(n);
        }
        _ => return false,
    }
    allowed == *after
}
fn profile_scope(repair: &ProfileRepair, actors: &BTreeSet<String>) -> bool {
    if repair.before.target != repair.after.target
        || repair.before.target.revision != "definition-r1"
    {
        return false;
    }
    let (prefix, family) = match (&repair.before.data, &repair.after.data) {
        (
            ProjectV2AuthoringProfileData::Presentation(_),
            ProjectV2AuthoringProfileData::Presentation(_),
        ) => ("oteryn:presentation.npc.", ProjectV2Family::Presentation),
        (
            ProjectV2AuthoringProfileData::Behavior(_),
            ProjectV2AuthoringProfileData::Behavior(_),
        ) => ("oteryn:behavior.npc.", ProjectV2Family::Behavior),
        _ => return false,
    };
    repair.before.target.family == family
        && repair
            .before
            .target
            .key
            .strip_prefix(prefix)
            .is_some_and(|stem| actors.contains(&format!("oteryn:npc.{stem}")))
}
pub(super) fn apply(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
    packet_sha256: &str,
    predecessor_sha256: &str,
) -> Result<usize> {
    if hex_sha256(bytes) != packet_sha256 {
        return Err("enrichment packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    if document_tree_digest(&before) != predecessor_sha256 {
        return Err("enrichment complete predecessor drifted".into());
    }
    let packet: Packet = serde_json::from_slice(bytes)?;
    if packet.schema != "OTERYN_NPC_BULK_ENRICHMENT/v1"
        || packet.from_project_revision != draft.core.project_revision
        || packet.project_revision == packet.from_project_revision
    {
        return Err("enrichment envelope drifted".into());
    }
    let actors = actors()?;
    let mut next = draft.clone();
    let mut seen = BTreeSet::new();
    let mut npc_count = 0;
    for repair in &packet.repairs {
        let (kind, id) =
            npc_repair_identity(&repair.before).ok_or("enrichment declaration family")?;
        if !seen.insert((kind, id.key.clone(), id.revision.clone()))
            || !scope(&repair.before, &repair.after, &actors)
        {
            return Err("enrichment declaration scope/duplicate drifted".into());
        }
        let indices = next
            .state
            .declarations
            .iter()
            .enumerate()
            .filter_map(|(i, d)| (d == &repair.before).then_some(i))
            .collect::<Vec<_>>();
        if indices.len() != 1 {
            return Err("enrichment original declaration drifted".into());
        }
        next.state.declarations[indices[0]] = repair.after.clone();
        npc_count += usize::from(kind == "NPC");
    }
    if npc_count != actors.len() {
        return Err("enrichment incomplete NPC inventory".into());
    }
    let mut seen = BTreeSet::new();
    for repair in &packet.profile_repairs {
        if !seen.insert(repair.before.target.key.clone()) || !profile_scope(repair, &actors) {
            return Err("enrichment profile scope/duplicate drifted".into());
        }
        let indices = next
            .state
            .authoring_profiles
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (p == &repair.before).then_some(i))
            .collect::<Vec<_>>();
        if indices.len() != 1 {
            return Err("enrichment original profile drifted".into());
        }
        next.state.authoring_profiles[indices[0]] = repair.after.clone();
    }
    next.core.project_revision = packet.project_revision;
    CanonicalProjectDocuments::from_v2_draft(next.clone(), limits())?;
    *draft = next;
    Ok(npc_count)
}

/// Test-only restoration of the exact predecessor, before older append fixtures reverse.
#[cfg(test)]
pub(super) fn reverse_for_fixture(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
    packet_sha256: &str,
    predecessor_sha256: &str,
) -> Result<()> {
    if hex_sha256(bytes) != packet_sha256 {
        return Err("fixture packet digest drifted".into());
    }
    let p: Packet = serde_json::from_slice(bytes)?;
    if draft.core.project_revision == p.from_project_revision {
        let documents = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
        if document_tree_digest(&documents) == predecessor_sha256 {
            return Ok(());
        }
        return Err("fixture existing predecessor drifted".into());
    }
    if draft.core.project_revision != p.project_revision {
        return Err("fixture successor revision drifted".into());
    }
    let mut next = draft.clone();
    for repair in p.repairs {
        let indices = next
            .state
            .declarations
            .iter()
            .enumerate()
            .filter_map(|(i, d)| (d == &repair.after).then_some(i))
            .collect::<Vec<_>>();
        if indices.len() != 1 {
            return Err("fixture successor declaration drifted".into());
        }
        next.state.declarations[indices[0]] = repair.before;
    }
    for repair in p.profile_repairs {
        let indices = next
            .state
            .authoring_profiles
            .iter()
            .enumerate()
            .filter_map(|(i, d)| (d == &repair.after).then_some(i))
            .collect::<Vec<_>>();
        if indices.len() != 1 {
            return Err("fixture successor profile drifted".into());
        }
        next.state.authoring_profiles[indices[0]] = repair.before;
    }
    next.core.project_revision = p.from_project_revision;
    let documents = CanonicalProjectDocuments::from_v2_draft(next.clone(), limits())?;
    if document_tree_digest(&documents) != predecessor_sha256 {
        return Err("fixture reverse tree drifted".into());
    }
    *draft = next;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    fn fixture() -> ProjectV2Draft {
        let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let fs_limits = ProjectFilesystemLimits {
            project: limits(),
            max_entries_per_directory_scan: 32,
            max_total_directory_entries_scanned: 201,
        };
        let mut draft = capture_world_project(&content, std::ffi::OsStr::new("world"), fs_limits)
            .expect("repository native project")
            .migrate_to_v2();
        reverse_for_fixture(
            &mut draft,
            NPC_ENRICH,
            NPC_ENRICH_SHA256,
            NPC_ENRICH_PREDECESSOR,
        )
        .expect("exact enrichment predecessor");
        draft
    }
    fn rejects(value: Value) {
        let bytes = serde_json::to_vec(&value).unwrap();
        let mut draft = fixture();
        let before = draft.clone();
        assert!(
            apply(
                &mut draft,
                &bytes,
                &hex_sha256(&bytes),
                NPC_ENRICH_PREDECESSOR
            )
            .is_err()
        );
        assert_eq!(draft, before);
    }
    #[test]
    fn packet_and_complete_predecessor_fences_are_atomic() {
        for wrong_packet in [true, false] {
            let mut draft = fixture();
            let before = draft.clone();
            assert!(
                apply(
                    &mut draft,
                    NPC_ENRICH,
                    if wrong_packet {
                        "wrong"
                    } else {
                        NPC_ENRICH_SHA256
                    },
                    if wrong_packet {
                        NPC_ENRICH_PREDECESSOR
                    } else {
                        "wrong"
                    }
                )
                .is_err()
            );
            assert_eq!(draft, before);
        }
    }
    #[test]
    fn enrichment_cannot_activate_services_or_change_actor_identity() {
        for foreign in [false, true] {
            let mut value: Value = serde_json::from_slice(NPC_ENRICH).unwrap();
            let row = value["repairs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["after"]["kind"] == "NPC")
                .unwrap();
            if foreign {
                row["after"]["identity"]["key"] = serde_json::json!("oteryn:npc.foreign_actor");
            } else {
                row["after"]["services"] = serde_json::json!([{
                    "family":"Service", "key":"oteryn:service.trade.foreign", "revision":"definition-r1"
                }]);
            }
            rejects(value);
        }
    }
    #[test]
    fn enrichment_preserves_disabled_features_and_rejects_duplicate_replacements() {
        let mut value: Value = serde_json::from_slice(NPC_ENRICH).unwrap();
        let row = value["repairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["after"]["kind"] == "NPC")
            .unwrap();
        let field = row["after"]["fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["field_path"] == "oteryn:source.npc.bulk.disabled_features")
            .unwrap();
        field["value"]["value"] = serde_json::json!("[]");
        rejects(value);
        let mut value: Value = serde_json::from_slice(NPC_ENRICH).unwrap();
        let rows = value["repairs"].as_array_mut().unwrap();
        rows.push(rows[0].clone());
        rejects(value);
    }
}
