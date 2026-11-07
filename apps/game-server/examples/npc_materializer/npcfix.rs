//! Exact-before NPC-AUDIT-1 corrections. This authoring step preserves unrelated declarations.
use super::*;

const PACKET: &[u8] = include_bytes!(
    "../../../../tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json"
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    repairs: Vec<Repair>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Repair {
    before: Option<ProjectV2Declaration>,
    after: ProjectV2Declaration,
    research: Vec<String>,
}

fn declaration_key(declaration: &ProjectV2Declaration) -> Option<&str> {
    match declaration {
        ProjectV2Declaration::Npc { identity, .. }
        | ProjectV2Declaration::Dialogue { identity, .. }
        | ProjectV2Declaration::Service { identity, .. } => Some(identity.key.as_str()),
        _ => None,
    }
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    if draft.core.project_revision != "e06-market-crystal-parity-20261031-r1" {
        return Err("NPCFIX predecessor revision drifted".into());
    }
    if hex_sha256(PACKET) != "5efa3c245199ba4c9b1926b21d7afadb76939c2a634746b1ca0e8702beafd480" {
        return Err("NPCFIX repair packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(PACKET)?;
    if packet.schema != "OTERYN_NPCFIX_NATIVE_REPAIRS/v1" || packet.repairs.len() != 53 {
        return Err("NPCFIX repair schema or count drifted".into());
    }
    let mut replacements = Vec::new();
    let mut seen = BTreeSet::new();
    for repair in packet.repairs {
        let identity =
            declaration_key(&repair.after).ok_or("NPCFIX cannot change another family")?;
        if !seen.insert(identity.to_owned()) || repair.research.is_empty() {
            return Err("NPCFIX duplicate identity or absent evidence".into());
        }
        let current: Vec<_> = draft
            .state
            .declarations
            .iter()
            .enumerate()
            .filter(|(_, d)| declaration_key(d) == Some(identity))
            .collect();
        let slot = match (&repair.before, current.as_slice()) {
            (None, []) => None,
            (Some(before), [(slot, current)]) if *current == before => Some(*slot),
            _ => return Err(format!("NPCFIX exact before-state drifted: {identity}").into()),
        };
        replacements.push((slot, repair.after));
    }
    // No partial repair when a later before-state fails.
    for (slot, declaration) in replacements {
        if let Some(slot) = slot {
            draft.state.declarations[slot] = declaration;
        } else {
            draft.state.declarations.push(declaration);
        }
    }
    draft.core.project_revision = "npcfix-retail-20261106-r1".into();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Result<ProjectV2Draft, Box<dyn std::error::Error>> {
        let packet: Packet = serde_json::from_slice(PACKET)?;
        Ok(ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "e06-market-crystal-parity-20261031-r1".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "PENDING".into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: Vec::new(),
                imports: Vec::new(),
                metadata: Vec::new(),
            },
            state: ProjectV2State {
                declarations: packet
                    .repairs
                    .into_iter()
                    .filter_map(|repair| repair.before)
                    .collect(),
                ..ProjectV2State::default()
            },
        })
    }

    #[test]
    fn npcfix_exact_before_repairs_admit_all_new_identities()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut draft = fixture()?;
        apply(&mut draft)?;
        let packet: Packet = serde_json::from_slice(PACKET)?;
        assert_eq!(draft.state.declarations.len(), packet.repairs.len());
        for repair in packet.repairs {
            assert!(draft.state.declarations.contains(&repair.after));
        }
        assert_eq!(draft.core.project_revision, "npcfix-retail-20261106-r1");
        Ok(())
    }

    #[test]
    fn npcfix_late_before_drift_rejects_the_entire_repair() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut draft = fixture()?;
        let Some(ProjectV2Declaration::Service { fields, .. }) = draft
            .state
            .declarations
            .iter_mut()
            .rfind(|declaration| matches!(declaration, ProjectV2Declaration::Service { .. }))
        else {
            return Err("expected final existing service".into());
        };
        fields.push(ProjectV2CandidateField {
            field_path: "oteryn:source.npc.drift".into(),
            value: ProjectV2CandidateValue::Integer(1),
        });
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        Ok(())
    }

    #[test]
    fn npcfix_npc_before_drift_rejects_the_entire_repair() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut draft = fixture()?;
        let Some(ProjectV2Declaration::Npc { fields, .. }) =
            draft.state.declarations.iter_mut().find(|declaration| {
                matches!(declaration, ProjectV2Declaration::Npc { identity, .. }
                    if identity.key == "oteryn:npc.edgar_ellen")
            })
        else {
            return Err("expected exact-before Edgar-Ellen NPC".into());
        };
        fields.push(ProjectV2CandidateField {
            field_path: "oteryn:source.npc.drift".into(),
            value: ProjectV2CandidateValue::Integer(1),
        });
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        Ok(())
    }
}
