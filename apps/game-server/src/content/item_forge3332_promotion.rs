//! Separate current-cut Forge pair into its existing authoring owner.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ProjectV2ItemAuthoring,
    ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition, ReferenceItemField,
    ReferenceItemSemantics, world_project_sha256,
};
use serde::Deserialize;
pub const ITEM_FORGE3332_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json"
);
pub const ITEM_FORGE3332_PACKET_SHA256: &str =
    "9cb2e9a0aec51870449de0a05c2d3796256ae6db30d114c63db4ace175884df1";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    promotion: ProjectV2ItemAuthoring,
}
pub fn apply_item_forge3332_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_FORGE3332_PACKET) != ITEM_FORGE3332_PACKET_SHA256 {
        return Err("Forge3332 packet digest drift".into());
    }
    let items = draft.core.records.iter().filter_map(|record| match record {
        ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } => Some((identity, semantics)),
        _ => None,
    });
    apply(
        items,
        &draft.state.source_identity_bindings,
        &mut draft.state.item_authoring,
        ITEM_FORGE3332_PACKET,
    )
}
fn apply<'a>(
    items: impl Iterator<Item = (&'a DefinitionIdentityDocument, &'a ReferenceItemSemantics)>,
    bindings: &[ProjectV2SourceIdentityBinding],
    owners: &mut Vec<ProjectV2ItemAuthoring>,
    bytes: &[u8],
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let row = packet.promotion;
    let minimal: ProjectV2ItemAuthoring = serde_json::from_value(serde_json::json!({"item":{"family":"Item","key":"oteryn:item.tibia.i3332","revision":"definition-r1"},"forge":{"classification":2,"max_tier":2}})).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_FORGE3332_PROMOTION/v1"
        || row != minimal
        || packet.bindings.len() != 1
    {
        return Err("Forge3332 closed source pair/target drift".into());
    }
    for expected in &packet.bindings {
        let matching: Vec<_> = bindings
            .iter()
            .filter(|b| {
                b.source_key == expected.source_key
                    && (b.target == row.item
                        || (b.source_revision == expected.source_revision
                            && b.identity_namespace == expected.identity_namespace
                            && b.external_id == expected.external_id))
            })
            .collect();
        if matching != vec![expected]
            || expected.target != row.item
            || expected.disposition != ProjectV2SourceIdentityDisposition::Exact
        {
            return Err("Forge3332 current source binding drift".into());
        }
    }
    let items: Vec<_> = items
        .filter(|(identity, _)| identity.key == row.item.key)
        .collect();
    if items.len() != 1 {
        return Err("Forge3332 current unique Item missing".into());
    }
    let (identity, semantics) = items[0];
    if identity.family != "Item"
        || identity.revision != row.item.revision
        || !matches!(&semantics.presentation, ReferenceItemField::Known(p) if p.name == ReferenceItemField::Known("hammer of wrath".into()))
    {
        return Err("Forge3332 current native target/name drift".into());
    }
    let matching: Vec<_> = owners
        .iter()
        .enumerate()
        .filter(|(_, owner)| owner.item.key == row.item.key)
        .collect();
    if matching.len() > 1
        || matching.iter().any(|(_, owner)| {
            owner.item != row.item || owner.forge.is_some_and(|forge| Some(forge) != row.forge)
        })
    {
        return Err("Forge3332 existing owner blocked/conflict".into());
    }
    let index = matching.first().map(|(index, _)| *index);
    // Every source, Item and owner precondition has passed; mutate only Forge.
    if let Some(index) = index {
        owners[index].forge = row.forge;
    } else {
        owners.push(row);
    }
    Ok(2)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Result<
        (Packet, DefinitionIdentityDocument, ReferenceItemSemantics),
        Box<dyn std::error::Error>,
    > {
        let packet: Packet = serde_json::from_slice(ITEM_FORGE3332_PACKET)?;
        let definition: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/agents/evidence/OTV2-20261002-item-forge3332-source-qualification-v2.json"
        ))?;
        let identity = serde_json::from_value(definition["target"].clone())?;
        let semantics = serde_json::from_value(
            definition["current_native_identity"]["retained_current_record"]["semantics"].clone(),
        )?;
        Ok((packet, identity, semantics))
    }
    #[test]
    fn absent_existing_and_idempotent_owner_preserve_siblings()
    -> Result<(), Box<dyn std::error::Error>> {
        let (packet, identity, semantics) = source()?;
        let mut owners = vec![];
        apply(
            std::iter::once((&identity, &semantics)),
            &packet.bindings,
            &mut owners,
            ITEM_FORGE3332_PACKET,
        )?;
        assert_eq!(owners, vec![packet.promotion.clone()]);
        owners[0].required_magic_level = Some(7);
        owners[0].use_observation =
            Some(serde_json::from_value(serde_json::json!({"mana_cost": 8}))?);
        let before = owners.clone();
        apply(
            std::iter::once((&identity, &semantics)),
            &packet.bindings,
            &mut owners,
            ITEM_FORGE3332_PACKET,
        )?;
        assert_eq!(owners, before);
        owners[0].forge = None;
        apply(
            std::iter::once((&identity, &semantics)),
            &packet.bindings,
            &mut owners,
            ITEM_FORGE3332_PACKET,
        )?;
        assert_eq!(owners, before);
        Ok(())
    }
    #[test]
    fn owner_binding_and_native_conflicts_fail_without_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (packet, identity, semantics) = source()?;
        for mode in 0..7 {
            let mut owners = vec![packet.promotion.clone()];
            let mut bindings = packet.bindings.clone();
            let mut id = identity.clone();
            match mode {
                0 => owners[0].forge.as_mut().ok_or("forge missing")?.max_tier = 3,
                1 => owners.push(owners[0].clone()),
                2 => owners[0].item.revision = "other".into(),
                3 => bindings[0].external_id = "other".into(),
                4 => id.revision = "other".into(),
                5 => id.family = "WorldObject".into(),
                _ => bindings[0].disposition = ProjectV2SourceIdentityDisposition::AcceptedAlias,
            }
            let before = owners.clone();
            assert!(
                apply(
                    std::iter::once((&id, &semantics)),
                    &bindings,
                    &mut owners,
                    ITEM_FORGE3332_PACKET
                )
                .is_err()
            );
            assert_eq!(owners, before);
        }
        let mut owners = vec![];
        assert!(
            apply(
                std::iter::empty(),
                &packet.bindings,
                &mut owners,
                ITEM_FORGE3332_PACKET
            )
            .is_err()
        );
        assert!(owners.is_empty());
        Ok(())
    }
}
