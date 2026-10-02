//! Atomic admission of flagged provisional NPC data; no gameplay or service activation.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;

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
    // Exact existing objects; donor/wiki snapshots are reused, not promoted.
    reused_sources: Vec<ProjectV2Source>,
    native_additions: Additions,
}
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn reference(r: &ProjectV2DefinitionRef, family: ProjectV2Family, key: &str) -> bool {
    r.family == family && r.key == key && r.revision == "definition-r1"
}
fn marked(fields: &[ProjectV2CandidateField]) -> bool {
    fields.iter().any(|f| {
        f.field_path == "oteryn:source.npc.bulk.status"
            && f.value == ProjectV2CandidateValue::Text("provisional".into())
    })
}

// The caller pins these three values once for the whole batch, never per NPC.
pub(super) fn apply(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
    packet_sha256: &str,
    predecessor_tree_sha256: &str,
    count: usize,
) -> Result<usize> {
    if hex_sha256(bytes) != packet_sha256 {
        return Err("provisional packet digest drifted".into());
    }
    let before = CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits())?;
    if document_tree_digest(&before) != predecessor_tree_sha256 {
        return Err("provisional complete predecessor drifted".into());
    }
    let p: Packet = serde_json::from_slice(bytes)?;
    if p.schema != "OTERYN_NPC_BULK_PROVISIONAL/v1"
        || p.from_project_revision != draft.core.project_revision
        || p.project_revision == p.from_project_revision
    {
        return Err("provisional envelope drifted".into());
    }
    let mut reused = BTreeSet::new();
    for source in &p.reused_sources {
        if !reused.insert((source.key.clone(), source.revision.clone()))
            || draft
                .state
                .sources
                .iter()
                .filter(|s| s.key == source.key && s.revision == source.revision)
                .collect::<Vec<_>>()
                != vec![source]
        {
            return Err("provisional reused source drifted".into());
        }
    }
    let a = &p.native_additions;
    let mut actors = BTreeSet::new();
    let mut dialogues = BTreeSet::new();
    for declaration in &a.declarations {
        match declaration {
            ProjectV2Declaration::Npc {
                identity,
                presentation,
                behavior,
                dialogue,
                services,
                fields,
            } => {
                let stem = identity
                    .key
                    .strip_prefix("oteryn:npc.")
                    .ok_or("provisional NPC key")?;
                if identity.revision != "definition-r1"
                    || !marked(fields)
                    || !services.is_empty()
                    || !actors.insert(stem.to_owned())
                    || presentation.as_ref().is_none_or(|r| {
                        !reference(
                            r,
                            ProjectV2Family::Presentation,
                            &format!("oteryn:presentation.npc.{stem}"),
                        )
                    })
                    || behavior.as_ref().is_none_or(|r| {
                        !reference(
                            r,
                            ProjectV2Family::Behavior,
                            &format!("oteryn:behavior.npc.{stem}"),
                        )
                    })
                    || dialogue.as_ref().is_none_or(|r| {
                        !reference(
                            r,
                            ProjectV2Family::Dialogue,
                            &format!("oteryn:dialogue.npc.{stem}"),
                        )
                    })
                {
                    return Err("provisional NPC inventory/reference drifted".into());
                }
            }
            ProjectV2Declaration::Dialogue {
                identity, fields, ..
            } => {
                let stem = identity
                    .key
                    .strip_prefix("oteryn:dialogue.npc.")
                    .ok_or("provisional dialogue key")?;
                if identity.revision != "definition-r1"
                    || !marked(fields)
                    || !dialogues.insert(stem.to_owned())
                {
                    return Err("provisional dialogue inventory drifted".into());
                }
            }
            _ => return Err("provisional foreign declaration family".into()),
        }
    }
    if actors.len() != count
        || dialogues != actors
        || a.records.len() != count * 2
        || a.authoring_profiles.len() != count * 2
    {
        return Err("provisional closed batch inventory drifted".into());
    }
    let expected = actors
        .iter()
        .flat_map(|s| {
            [
                format!("oteryn:presentation.npc.{s}"),
                format!("oteryn:behavior.npc.{s}"),
            ]
        })
        .collect::<BTreeSet<_>>();
    let mut records = BTreeSet::new();
    for record in &a.records {
        let ProjectReferenceRecord::Generic {
            identity,
            client_projection,
        } = record
        else {
            return Err("provisional foreign record family".into());
        };
        let presentation = identity.key.starts_with("oteryn:presentation.npc.");
        if !expected.contains(&identity.key)
            || !records.insert(identity.key.clone())
            || identity.revision != "definition-r1"
            || identity.family
                != if presentation {
                    "Presentation"
                } else {
                    "Behavior"
                }
            || *client_projection
                != if presentation {
                    ProjectionDocument::ClientSafe
                } else {
                    ProjectionDocument::ServerOnly
                }
        {
            return Err("provisional record inventory/projection drifted".into());
        }
    }
    let mut profiles = BTreeSet::new();
    for profile in &a.authoring_profiles {
        let family = match &profile.data {
            ProjectV2AuthoringProfileData::Presentation(_) => ProjectV2Family::Presentation,
            ProjectV2AuthoringProfileData::Behavior(_) => ProjectV2Family::Behavior,
            _ => return Err("provisional foreign profile family".into()),
        };
        if !expected.contains(&profile.target.key)
            || !reference(&profile.target, family, &profile.target.key)
            || !profiles.insert(profile.target.key.clone())
        {
            return Err("provisional profile inventory drifted".into());
        }
    }
    for binding in &a.source_identity_bindings {
        let stem = binding
            .target
            .key
            .strip_prefix("oteryn:npc.")
            .ok_or("provisional binding target key")?;
        if !actors.contains(stem)
            || !reference(&binding.target, ProjectV2Family::Npc, &binding.target.key)
            || !reused.contains(&(binding.source_key.clone(), binding.source_revision.clone()))
        {
            return Err("provisional source binding scope drifted".into());
        }
    }
    // Canonical native validation resolves all references and checks duplicate identities,
    // candidate fields, source bindings, asset/profile shape, and existing limits.
    // Append to a clone, leaving the caller entirely unchanged on any validation error.
    let mut next = draft.clone();
    next.state.declarations.extend(a.declarations.clone());
    next.core.records.extend(a.records.clone());
    next.state
        .authoring_profiles
        .extend(a.authoring_profiles.clone());
    next.state
        .source_identity_bindings
        .extend(a.source_identity_bindings.clone());
    next.core.project_revision = p.project_revision;
    CanonicalProjectDocuments::from_v2_draft(next.clone(), limits())?;
    *draft = next;
    Ok(count)
}

// Append inside bulk_provisional.rs. Uses the real native repository catalog;
// reverse-removes only this packet when run after generated content is checked in.
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
        let p: Packet = serde_json::from_slice(NPC_BULK).unwrap();
        let more: Packet = serde_json::from_slice(NPC_BULK_MORE).unwrap();
        let ids = p
            .native_additions
            .declarations
            .iter()
            .chain(more.native_additions.declarations.iter())
            .map(|d| {
                serde_json::to_value(d).unwrap()["identity"]["key"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();
        let records = p
            .native_additions
            .records
            .iter()
            .chain(more.native_additions.records.iter())
            .map(|r| {
                serde_json::to_value(r).unwrap()["identity"]["key"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();
        draft.state.declarations.retain(|d| {
            !ids.contains(
                serde_json::to_value(d).unwrap()["identity"]["key"]
                    .as_str()
                    .unwrap(),
            )
        });
        draft.core.records.retain(|r| {
            !records.contains(
                serde_json::to_value(r).unwrap()["identity"]["key"]
                    .as_str()
                    .unwrap(),
            )
        });
        draft
            .state
            .authoring_profiles
            .retain(|p| !records.contains(&p.target.key));
        draft.state.source_identity_bindings.retain(|b| {
            !p.native_additions.source_identity_bindings.contains(b)
                && !more.native_additions.source_identity_bindings.contains(b)
        });
        draft.core.project_revision = p.from_project_revision;
        assert_eq!(
            document_tree_digest(
                &CanonicalProjectDocuments::from_v2_draft(draft.clone(), limits()).unwrap()
            ),
            NPC_BULK_PREDECESSOR
        );
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
                NPC_BULK_PREDECESSOR,
                NPC_BULK_COUNT
            )
            .is_err()
        );
        assert_eq!(draft, before);
    }
    #[test]
    fn packet_and_predecessor_fences_reject_without_mutation() {
        for wrong_packet in [true, false] {
            let mut draft = fixture();
            let before = draft.clone();
            assert!(
                apply(
                    &mut draft,
                    NPC_BULK,
                    if wrong_packet {
                        "wrong"
                    } else {
                        NPC_BULK_SHA256
                    },
                    if wrong_packet {
                        NPC_BULK_PREDECESSOR
                    } else {
                        "wrong"
                    },
                    NPC_BULK_COUNT
                )
                .is_err()
            );
            assert_eq!(draft, before);
        }
    }
    #[test]
    fn injected_service_reference_rejects_without_mutation() {
        let mut value: Value = serde_json::from_slice(NPC_BULK).unwrap();
        let npc = value["native_additions"]["declarations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["kind"] == "NPC")
            .unwrap();
        npc["services"] = serde_json::json!([{
            "family":"Service", "key":"oteryn:service.npc.injected", "revision":"definition-r1"
        }]);
        rejects(value);
    }
    #[test]
    fn duplicate_source_binding_fails_late_native_validation_atomically() {
        let mut value: Value = serde_json::from_slice(NPC_BULK).unwrap();
        let bindings = value["native_additions"]["source_identity_bindings"]
            .as_array_mut()
            .unwrap();
        bindings.push(bindings[0].clone());
        rejects(value);
    }
}
