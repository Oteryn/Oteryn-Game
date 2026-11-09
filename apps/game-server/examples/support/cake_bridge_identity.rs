//! Owner decision 2026-11-08 admits Crystal's explicit bridge producer identity.
use super::*;

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    let packet: Value = serde_json::from_slice(include_bytes!(
        "../../../../tools/content-schema/native-gameplay/cake-bridge-identity.json"
    ))?;
    if packet["schema"] != "OTERYN_CAKE_BRIDGE_IDENTITY/v1"
        || packet["decision"] != "OWNER DECISION 2026-11-08: Zatwierdzam 48227"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["definition"]
            != serde_json::json!({
                "kind":"Item", "identity":{"family":"Item","key":"oteryn:item.tibia.i48227","revision":"definition-r1"},
                "client_projection":"ClientSafe", "materializable":false, "stack_class":"Unknown"
            })
    {
        return Err("Cake bridge identity packet drift".into());
    }
    let definition: ProjectReferenceRecord = serde_json::from_value(packet["definition"].clone())?;
    let existing: Vec<_> = draft.core.records.iter().filter(|r| {
        matches!(r, ProjectReferenceRecord::Item { identity, .. } if identity.key == "oteryn:item.tibia.i48227")
    }).collect();
    if !existing.is_empty() {
        if existing == [&definition] {
            draft.core.project_revision = "cake-bridge-owner-approved-20261108-r1".into();
            return Ok(());
        }
        return Err("Cake bridge canonical identity conflict".into());
    }
    draft.core.records.push(definition);
    draft.core.project_revision = "cake-bridge-owner-approved-20261108-r1".into();
    Ok(())
}
