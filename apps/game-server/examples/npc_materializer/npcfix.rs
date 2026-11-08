//! Exact-before NPC-AUDIT-1 corrections. This authoring step preserves unrelated declarations.
use super::*;

const PACKET: &[u8] = include_bytes!(
    "../../../../tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json"
);

const FLUID_FACTS: &[u8] = include_bytes!(
    "../../../../tools/content-schema/npc-authoring/samples/npcfix-fluid-containers.json"
);

/// Apply only evidenced fluid-container facts; all before-state checks precede writes.
fn apply_fluid_container_facts(
    draft: &mut ProjectV2Draft,
) -> Result<(), Box<dyn std::error::Error>> {
    if hex_sha256(FLUID_FACTS) != "efcae45114e079dd18dcd24728c7692c73b1f870f85da33e8f1c5c36d4ec8429"
    {
        return Err("fluid container fact packet digest drifted".into());
    }
    let facts: Value = serde_json::from_slice(FLUID_FACTS)?;
    if facts["schema"] != "OTERYN_NATIVE_FLUID_CONTAINERS/v1" {
        return Err("fluid container fact schema drifted".into());
    }
    let rows = facts["items"]
        .as_array()
        .ok_or("fluid container items missing")?;
    if rows.is_empty() || rows.len() > 4096 {
        return Err("fluid container fact count".into());
    }
    let mut slots = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let key = row["item_key"].as_str().ok_or("fluid container identity")?;
        let revision = row["revision"].as_str().ok_or("fluid container revision")?;
        if !seen.insert(key) {
            return Err("duplicate fluid container fact".into());
        }
        let slot = draft
            .core
            .records
            .iter()
            .position(|record| {
                matches!(record,
            ProjectReferenceRecord::Item { identity, .. }
                if identity.key == key && identity.revision == revision)
            })
            .ok_or("fluid container item absent")?;
        let ProjectReferenceRecord::Item { semantics, .. } = &draft.core.records[slot] else {
            return Err("fluid container item family".into());
        };
        if !semantics.fluid.is_unknown() {
            return Err(format!("fluid container baseline drift: {key}").into());
        }
        slots.push(slot);
    }
    let field: ReferenceItemField<ReferenceItemFluid> =
        serde_json::from_value(serde_json::json!({
            "state":"KNOWN", "value":{"fluid_type":{"state":"NOT_APPLICABLE"}}
        }))?;
    for slot in slots {
        if let ProjectReferenceRecord::Item { semantics, .. } = &mut draft.core.records[slot] {
            semantics.fluid = field.clone();
        }
    }
    Ok(())
}

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
    if hex_sha256(PACKET) != "48e3effd29273a3f890112d422572f61a5558e8c783fba1004f51824d9b3475c" {
        return Err("NPCFIX repair packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(PACKET)?;
    if packet.schema != "OTERYN_NPCFIX_NATIVE_REPAIRS/v1" || packet.repairs.len() != 66 {
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
            _ => {
                return Err(format!(
                    "NPCFIX exact before-state drifted: {identity}; expected={}; current={}",
                    serde_json::to_string(&repair.before)?,
                    serde_json::to_string(
                        &current
                            .iter()
                            .map(|(_, declaration)| *declaration)
                            .collect::<Vec<_>>()
                    )?
                )
                .into());
            }
        };
        replacements.push((slot, repair.after));
    }
    apply_fluid_container_facts(draft)?;
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
                records: [2874, 2901].into_iter().map(|id| serde_json::from_value(serde_json::json!({
                    "kind":"Item", "identity":{"family":"Item","key":format!("oteryn:item.tibia.i{id}"),"revision":"definition-r1"},
                    "client_projection":"ClientSafe","materializable":false,"stack_class":"Unknown"
                }))).collect::<Result<Vec<_>, _>>()?,
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
        for record in &draft.core.records {
            let ProjectReferenceRecord::Item { semantics, .. } = record else {
                return Err("expected fluid Item".into());
            };
            assert!(matches!(&semantics.fluid, ReferenceItemField::Known(fluid)
                if matches!(fluid.fluid_type, ReferenceItemField::NotApplicable)));
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
    #[test]
    fn npcfix_late_fluid_fact_drift_rejects_items_and_npcs_atomically()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut draft = fixture()?;
        let ProjectReferenceRecord::Item { semantics, .. } = &mut draft.core.records[1] else {
            return Err("expected fluid Item".into());
        };
        semantics.fluid = serde_json::from_value(serde_json::json!({
            "state":"KNOWN", "value":{"fluid_type":{"state":"NOT_APPLICABLE"}}
        }))?;
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        Ok(())
    }
}
