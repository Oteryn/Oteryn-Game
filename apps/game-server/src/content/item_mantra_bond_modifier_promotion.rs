//! Closed49 complete raw groups; preserve context Unknown and existing atomic setter.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft,
    item_stats_promotion::{ItemStatsPromotion, apply_packet},
    world_project_sha256,
};
use std::collections::BTreeMap;

pub const ITEM_MANTRA_BOND_MODIFIER_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json"
);
pub const ITEM_MANTRA_BOND_MODIFIER_PACKET_SHA256: &str =
    "ec0c44a1f85fd7a542815a8579ba7bede882b7bd5b71ac066f37357351960ff7";

fn validate_targets<'a>(
    bytes: &[u8],
    identities: impl Iterator<Item = &'a DefinitionIdentityDocument>,
    expected: usize,
    atoms: usize,
) -> Result<(), String> {
    let packet: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let rows = packet["promotions"]
        .as_array()
        .ok_or("modifier rows missing")?;
    if rows.len() != expected
        || rows
            .iter()
            .map(|r| r["typed_value"]["value"].as_array().map_or(0, Vec::len))
            .sum::<usize>()
            != atoms
    {
        return Err("closed modifier counts drift".into());
    }
    let mut native = BTreeMap::new();
    for identity in identities {
        if native.insert(identity.key.as_str(), identity).is_some() {
            return Err("duplicate native Item identity".into());
        }
    }
    for row in rows {
        let target: DefinitionIdentityDocument =
            serde_json::from_value(row["target"].clone()).map_err(|e| e.to_string())?;
        if target.family != "Item"
            || target.revision != "definition-r1"
            || row["item_key"].as_str() != Some(target.key.as_str())
            || row["field_path"].as_str() != Some("skill_modifiers.modifiers")
            || native.get(target.key.as_str()).copied() != Some(&target)
        {
            return Err("full modifier Item target mismatch".into());
        }
    }
    Ok(())
}

pub fn apply_item_mantra_bond_modifier_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<ItemStatsPromotion, String> {
    if world_project_sha256(ITEM_MANTRA_BOND_MODIFIER_PACKET)
        != ITEM_MANTRA_BOND_MODIFIER_PACKET_SHA256
    {
        return Err("Mantra/Bond packet digest drift".into());
    }
    validate_targets(
        ITEM_MANTRA_BOND_MODIFIER_PACKET,
        draft.core.records.iter().filter_map(|r| match r {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity),
            _ => None,
        }),
        49,
        114,
    )?;
    apply_packet(draft, ITEM_MANTRA_BOND_MODIFIER_PACKET).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_targets_reject_wrong_revision_family_key_and_late_missing()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut packet: serde_json::Value =
            serde_json::from_slice(ITEM_MANTRA_BOND_MODIFIER_PACKET)?;
        packet["promotions"] = serde_json::json!([packet["promotions"][0]]);
        let identity: DefinitionIdentityDocument =
            serde_json::from_value(packet["promotions"][0]["target"].clone())?;
        let atoms = packet["promotions"][0]["typed_value"]["value"]
            .as_array()
            .ok_or("vector missing")?
            .len();
        let bytes = serde_json::to_vec(&packet)?;
        assert!(validate_targets(&bytes, std::iter::once(&identity), 1, atoms).is_ok());
        for field in ["family", "key", "revision"] {
            let mut wrong = packet.clone();
            wrong["promotions"][0]["target"][field] = serde_json::json!("wrong");
            assert!(
                validate_targets(
                    &serde_json::to_vec(&wrong)?,
                    std::iter::once(&identity),
                    1,
                    atoms
                )
                .is_err()
            );
        }
        assert!(validate_targets(&bytes, std::iter::empty(), 1, atoms).is_err());
        assert!(validate_targets(&bytes, [&identity, &identity].into_iter(), 1, atoms).is_err());
        Ok(())
    }
}
