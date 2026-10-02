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
        "profession_selection",
        "appearance_selection",
        "fallback_selection",
        "completion",
        "appearance_reference",
        "dialogue_followup",
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
// Exact placeholder inventory in the pinned R22 successor; donors cannot be overwritten.
fn placeholder_presentations() -> Result<BTreeSet<String>> {
    if hex_sha256(NPC_ENRICH_MORE) != NPC_ENRICH_MORE_SHA256 {
        return Err("placeholder source packet drifted".into());
    }
    let p: Packet = serde_json::from_slice(NPC_ENRICH_MORE)?;
    let mut keys = BTreeSet::new();
    for repair in p.repairs {
        if let ProjectV2Declaration::Npc {
            identity, fields, ..
        } = repair.after
        {
            let quality = fields
                .iter()
                .find(|f| f.field_path == "oteryn:source.npc.bulk.quality")
                .ok_or("placeholder quality missing")?;
            let ProjectV2CandidateValue::Text(text) = &quality.value else {
                return Err("placeholder quality shape".into());
            };
            let quality: Value = serde_json::from_str(text)?;
            if quality["presentation"] == "placeholder" {
                let stem = identity
                    .key
                    .strip_prefix("oteryn:npc.")
                    .ok_or("placeholder NPC key")?;
                if !keys.insert(format!("oteryn:presentation.npc.{stem}")) {
                    return Err("duplicate placeholder target".into());
                }
            }
        }
    }
    if keys.len() != 112 {
        return Err("placeholder closed inventory drifted".into());
    }
    Ok(keys)
}
// R25 changes only presentation authoring and its documentary visual evidence.
fn visual_targets() -> Result<BTreeSet<String>> {
    if hex_sha256(NPC_ENRICH_UPGRADE) != NPC_ENRICH_UPGRADE_SHA256 {
        return Err("visual predecessor packet drifted".into());
    }
    let p: Packet = serde_json::from_slice(NPC_ENRICH_UPGRADE)?;
    let mut keys = BTreeSet::new();
    for repair in p.repairs {
        if let ProjectV2Declaration::Npc {
            identity, fields, ..
        } = repair.after
        {
            let q = fields
                .iter()
                .find(|f| f.field_path == "oteryn:source.npc.bulk.quality")
                .ok_or("visual quality missing")?;
            let ProjectV2CandidateValue::Text(text) = &q.value else {
                return Err("visual quality shape".into());
            };
            let q: Value = serde_json::from_str(text)?;
            if q["presentation"] == "defaulted" {
                keys.insert(
                    identity
                        .key
                        .replacen("oteryn:npc.", "oteryn:presentation.npc.", 1),
                );
            }
        }
    }
    if keys.len() != 110 {
        return Err("visual closed inventory drifted".into());
    }
    Ok(keys)
}
fn visual_followup_targets() -> Result<BTreeSet<String>> {
    let mut keys = visual_targets()?;
    if hex_sha256(NPC_APPEARANCE_VISUAL) != NPC_APPEARANCE_VISUAL_SHA256 {
        return Err("visual follow-up predecessor packet drifted".into());
    }
    let p: Packet = serde_json::from_slice(NPC_APPEARANCE_VISUAL)?;
    if p.profile_repairs.len() != 98 {
        return Err("visual follow-up original selection inventory drifted".into());
    }
    for repair in p.profile_repairs {
        if !keys.remove(&repair.after.target.key) {
            return Err("visual follow-up selected profile drifted".into());
        }
    }
    if keys.len() != 12 {
        return Err("visual follow-up held inventory drifted".into());
    }
    Ok(keys)
}
fn visual_scope(
    before: &ProjectV2Declaration,
    after: &ProjectV2Declaration,
    from_revision: &str,
) -> Result<bool> {
    let child = if from_revision == "g4-npc-provisional-enrichment-r25" {
        "r26_visual_mapping"
    } else {
        "r25_visual_mapping"
    };
    let (ProjectV2Declaration::Npc { fields, .. }, ProjectV2Declaration::Npc { fields: next, .. }) =
        (before, after)
    else {
        return Ok(false);
    };
    for old in fields {
        let Some(new) = next.iter().find(|f| f.field_path == old.field_path) else {
            return Ok(false);
        };
        if old == new {
            continue;
        }
        let (ProjectV2CandidateValue::Text(a), ProjectV2CandidateValue::Text(b)) =
            (&old.value, &new.value)
        else {
            return Ok(false);
        };
        let mut a: Value = serde_json::from_str(a)?;
        let mut b: Value = serde_json::from_str(b)?;
        match old.field_path.as_str() {
            "oteryn:source.npc.bulk.quality" => {
                let (Some(a), Some(b)) = (a.as_object_mut(), b.as_object_mut()) else {
                    return Ok(false);
                };
                for component in [
                    "look_type",
                    "head",
                    "body",
                    "legs",
                    "feet",
                    "addons",
                    "mount",
                    "item_look",
                ] {
                    if b.get(&format!("presentation.{component}"))
                        .is_some_and(|v| v != "defaulted")
                    {
                        return Ok(false);
                    }
                }
                a.retain(|k, _| !k.starts_with("presentation."));
                b.retain(|k, _| !k.starts_with("presentation."));
                if a != b {
                    return Ok(false);
                }
            }
            "oteryn:source.npc.bulk.source_metadata" => {
                let (Some(a), Some(b)) = (a.as_object_mut(), b.as_object_mut()) else {
                    return Ok(false);
                };
                a.remove(child);
                b.remove(child);
                if a != b {
                    return Ok(false);
                }
            }
            "oteryn:source.npc.bulk.appearance_selection" => {
                let documented_invisible = from_revision == "g4-npc-provisional-enrichment-r25"
                    && matches!(before,ProjectV2Declaration::Npc{identity,..}
                        if identity.key=="oteryn:npc.opticorder_forge_npc")
                    && b["classification"] == "APPROXIMATE_WIKI_DOCUMENTED_INVISIBLE_MAPPING"
                    && b["visibility"] == "invisible"
                    && b["outfit"].is_null();
                let documentary_default = from_revision == "g4-npc-provisional-enrichment-r25"
                    && matches!(before,ProjectV2Declaration::Npc{identity,..}
                        if matches!(identity.key.as_str(),"oteryn:npc.mud"|"oteryn:npc.planestrider_npc"))
                    && b["classification"] == "PROJECT_DEFAULT_NO_SOURCE_SPRITE"
                    && b["source_completeness"] == "unknown"
                    && b["visual_correspondence"] == "unknown";
                if (!documented_invisible
                    && !documentary_default
                    && !matches!(
                        b["classification"].as_str(),
                        Some(
                            "APPROXIMATE_WIKI_VISUAL_MAPPING"
                                | "APPROXIMATE_WIKI_VISUAL_OBJECT_MAPPING"
                        )
                    ))
                    || b["actor_exact_match"] != false
                    || b["canonical_tibia_fidelity_claim"] != false
                {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(fields.len() == next.len())
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
    let placeholders = if packet.from_project_revision == "g4-npc-provisional-enrichment-r25" {
        Some(visual_followup_targets()?)
    } else if packet.from_project_revision == "g4-npc-provisional-enrichment-r24" {
        Some(visual_targets()?)
    } else if matches!(
        packet.from_project_revision.as_str(),
        "g4-npc-provisional-enrichment-r22" | "g4-npc-provisional-enrichment-r23"
    ) {
        Some(placeholder_presentations()?)
    } else {
        None
    };
    let actors = actors()?;
    let mut next = draft.clone();
    let mut seen = BTreeSet::new();
    let mut npc_count = 0;
    for repair in &packet.repairs {
        if matches!(
            packet.from_project_revision.as_str(),
            "g4-npc-provisional-enrichment-r24" | "g4-npc-provisional-enrichment-r25"
        ) && !visual_scope(&repair.before, &repair.after, &packet.from_project_revision)?
        {
            return Err("appearance-only successor cannot change dialogue/roles/history".into());
        }
        // R22 supplies selected replies only; keyword matching and flow remain unchanged.
        if matches!(
            packet.from_project_revision.as_str(),
            "g4-npc-provisional-enrichment-r21"
                | "g4-npc-provisional-enrichment-r22"
                | "g4-npc-provisional-enrichment-r23"
        ) && let (
            ProjectV2Declaration::Dialogue { keywords: k, .. },
            ProjectV2Declaration::Dialogue { keywords: nk, .. },
        ) = (&repair.before, &repair.after)
        {
            let retained = if nk.len() == k.len() + 1 {
                let last = nk.last().ok_or("missing successor story")?;
                if serde_json::to_value(last)?
                    != serde_json::json!({
                        "key":"story", "triggers":["story"], "reply":last.reply
                    })
                    || !(1..=2).contains(&last.reply.len())
                {
                    return Err("successor story is not a plain static leaf".into());
                }
                &nk[..nk.len() - 1]
            } else {
                nk.as_slice()
            };
            if !npc_dialogue_replies_allowed(k, retained) {
                return Err("successor dialogue trigger/flow drifted".into());
            }
        }
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
        if packet.from_project_revision == "g4-npc-provisional-enrichment-r25"
            && matches!(
                repair.after.target.key.as_str(),
                "oteryn:presentation.npc.mud" | "oteryn:presentation.npc.planestrider_npc"
            )
        {
            return Err("source-unknown neutral choices must preserve existing profiles".into());
        }

        if packet.from_project_revision == "g4-npc-provisional-enrichment-r25"
            && matches!(&repair.after.data,ProjectV2AuthoringProfileData::Presentation(p)
                if p.asset_binding.is_none())
            && repair.after.target.key != "oteryn:presentation.npc.opticorder_forge_npc"
        {
            return Err("documented invisibility cannot hide another actor".into());
        }

        if placeholders.as_ref().is_some_and(|keys| {
            !keys.contains(&repair.before.target.key)
                || !matches!(
                    (&repair.before.data, &repair.after.data),
                    (
                        ProjectV2AuthoringProfileData::Presentation(_),
                        ProjectV2AuthoringProfileData::Presentation(_)
                    )
                )
        }) {
            return Err("project defaults cannot overwrite donor appearance or behavior".into());
        }
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

/// Restore an exact chain using one complete predecessor validation, not one per wave.
#[cfg(test)]
pub(super) fn reverse_for_fixture(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
    packet_sha256: &str,
    predecessor_sha256: &str,
) -> Result<()> {
    let chain = if bytes == NPC_ENRICH {
        vec![
            (NPC_APPEARANCE_FOLLOWUP, NPC_APPEARANCE_FOLLOWUP_SHA256),
            (NPC_APPEARANCE_VISUAL, NPC_APPEARANCE_VISUAL_SHA256),
            (NPC_ENRICH_UPGRADE, NPC_ENRICH_UPGRADE_SHA256),
            (NPC_ENRICH_FINAL, NPC_ENRICH_FINAL_SHA256),
            (NPC_ENRICH_MORE, NPC_ENRICH_MORE_SHA256),
            (bytes, packet_sha256),
        ]
    } else if bytes == NPC_ENRICH_MORE {
        vec![
            (NPC_APPEARANCE_FOLLOWUP, NPC_APPEARANCE_FOLLOWUP_SHA256),
            (NPC_APPEARANCE_VISUAL, NPC_APPEARANCE_VISUAL_SHA256),
            (NPC_ENRICH_UPGRADE, NPC_ENRICH_UPGRADE_SHA256),
            (NPC_ENRICH_FINAL, NPC_ENRICH_FINAL_SHA256),
            (bytes, packet_sha256),
        ]
    } else if bytes == NPC_ENRICH_FINAL {
        vec![
            (NPC_APPEARANCE_FOLLOWUP, NPC_APPEARANCE_FOLLOWUP_SHA256),
            (NPC_APPEARANCE_VISUAL, NPC_APPEARANCE_VISUAL_SHA256),
            (NPC_ENRICH_UPGRADE, NPC_ENRICH_UPGRADE_SHA256),
            (bytes, packet_sha256),
        ]
    } else if bytes == NPC_ENRICH_UPGRADE {
        vec![
            (NPC_APPEARANCE_FOLLOWUP, NPC_APPEARANCE_FOLLOWUP_SHA256),
            (NPC_APPEARANCE_VISUAL, NPC_APPEARANCE_VISUAL_SHA256),
            (bytes, packet_sha256),
        ]
    } else if bytes == NPC_APPEARANCE_VISUAL {
        vec![
            (NPC_APPEARANCE_FOLLOWUP, NPC_APPEARANCE_FOLLOWUP_SHA256),
            (bytes, packet_sha256),
        ]
    } else {
        vec![(bytes, packet_sha256)]
    };
    let mut next = draft.clone();
    for (bytes, sha) in chain {
        if hex_sha256(bytes) != sha {
            return Err("fixture packet digest drifted".into());
        }
        let p: Packet = serde_json::from_slice(bytes)?;
        if p.schema != "OTERYN_NPC_BULK_ENRICHMENT/v1" {
            return Err("fixture packet schema drifted".into());
        }
        // R21 catalogues are also valid inputs for fixtures before R22 is installed.
        if next.core.project_revision == p.from_project_revision {
            continue;
        }
        if next.core.project_revision != p.project_revision {
            return Err("fixture successor revision drifted".into());
        }
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
    }
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
    #[test]
    fn r22_successor_cannot_change_matching_or_add_conversation_flow() {
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
            NPC_ENRICH_MORE,
            NPC_ENRICH_MORE_SHA256,
            NPC_ENRICH_MORE_PREDECESSOR,
        )
        .expect("R21 predecessor");
        let before = draft.clone();
        let mut value: Value = serde_json::from_slice(NPC_ENRICH_MORE).unwrap();
        let row = value["repairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["after"]["kind"] == "Dialogue")
            .unwrap();
        row["after"]["keywords"][0]["triggers"] = serde_json::json!(["changed"]);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            apply(
                &mut draft,
                &bytes,
                &hex_sha256(&bytes),
                NPC_ENRICH_MORE_PREDECESSOR
            )
            .is_err()
        );
        assert_eq!(draft, before);
    }
    #[test]
    fn r23_project_defaults_cannot_override_donor_or_foreign_presentations() {
        let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let fs_limits = ProjectFilesystemLimits {
            project: limits(),
            max_entries_per_directory_scan: 32,
            max_total_directory_entries_scanned: 201,
        };
        let mut predecessor =
            capture_world_project(&content, std::ffi::OsStr::new("world"), fs_limits)
                .expect("repository native project")
                .migrate_to_v2();
        reverse_for_fixture(
            &mut predecessor,
            NPC_ENRICH_FINAL,
            NPC_ENRICH_FINAL_SHA256,
            NPC_ENRICH_FINAL_PREDECESSOR,
        )
        .expect("R22 predecessor");
        let placeholders = placeholder_presentations().unwrap();
        let actors = actors().unwrap();
        let donor = predecessor
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target
                    .key
                    .strip_prefix("oteryn:presentation.npc.")
                    .is_some_and(|stem| actors.contains(&format!("oteryn:npc.{stem}")))
                    && !placeholders.contains(&p.target.key)
            })
            .expect("one preserved donor presentation");
        for foreign in [false, true] {
            let mut profile = serde_json::to_value(donor).unwrap();
            if foreign {
                profile["target"]["key"] =
                    serde_json::json!("oteryn:presentation.npc.foreign_actor");
            }
            let mut packet: Value = serde_json::from_slice(NPC_ENRICH_FINAL).unwrap();
            packet["profile_repairs"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"before": profile.clone(), "after": profile}));
            let bytes = serde_json::to_vec(&packet).unwrap();
            let mut draft = predecessor.clone();
            assert!(
                apply(
                    &mut draft,
                    &bytes,
                    &hex_sha256(&bytes),
                    NPC_ENRICH_FINAL_PREDECESSOR
                )
                .is_err()
            );
            assert_eq!(draft, predecessor);
        }
    }
    #[test]
    fn visual_successor_scope_preserves_roles_dialogue_and_donor_quality() {
        let p: Packet = serde_json::from_slice(NPC_ENRICH_UPGRADE).unwrap();
        let before = p
            .repairs
            .iter()
            .find(|r| matches!(r.after, ProjectV2Declaration::Npc { .. }))
            .unwrap()
            .after
            .clone();
        assert!(visual_scope(&before, &before, "g4-npc-provisional-enrichment-r24").unwrap());
        let mut after = before.clone();
        if let ProjectV2Declaration::Npc { fields, .. } = &mut after {
            let field = fields
                .iter_mut()
                .find(|f| f.field_path == "oteryn:source.npc.bulk.quality")
                .unwrap();
            let ProjectV2CandidateValue::Text(text) = &mut field.value else {
                panic!("typed quality");
            };
            let mut q: Value = serde_json::from_str(text).unwrap();
            q["profession"] = serde_json::json!("foreign");
            *text = serde_json::to_string(&q).unwrap();
        }
        assert!(!visual_scope(&before, &after, "g4-npc-provisional-enrichment-r24").unwrap());
        let dialogue = p
            .repairs
            .iter()
            .find(|r| matches!(r.after, ProjectV2Declaration::Dialogue { .. }))
            .unwrap();
        assert!(
            !visual_scope(
                &dialogue.before,
                &dialogue.after,
                "g4-npc-provisional-enrichment-r24"
            )
            .unwrap()
        );
    }
}
