//! Exact carried policies of the nineteen missing canonical Galthen loot yields.
use super::*;
#[path = "galthen_loot_pin.rs"]
mod pin;

const IDS: [&str; 19] = [
    "40527", "40528", "40529", "40535", "40578", "44438", "44439", "44619", "44620", "44636",
    "44637", "44642", "44643", "44648", "44649", "44727", "44728", "50255", "50291",
];
const CAPTURE: &[u8] =
    include_bytes!("../../../../../../tools/content-schema/native-gameplay/galthen-boss-loot.json");

pub(super) fn qualified_records() -> Result<Vec<ItemProfileRecord>, ContentError> {
    qualify(CAPTURE)
}

pub(super) fn qualified_global_records(
    source: &[ItemProfileRecord],
) -> Result<BTreeMap<String, ItemProfileRecord>, ContentError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GlobalDetails {
        semantics: Option<super::super::ReferenceItemSemantics>,
        forge: Option<super::super::ProjectV2ItemForgeProfile>,
        ranged: Option<super::super::ProjectV2RangedWeapon>,
    }

    let mut details: BTreeMap<String, GlobalDetails> = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/galthen-global-item-details.json"
    ))
    .map_err(|_| invalid("M25 Global Item Details projection"))?;
    let mut profiles = BTreeMap::new();
    for row in source {
        let key = &row.production_binding.external_id;
        let Some(detail) = details.remove(key) else {
            continue;
        };
        let mut successor = row.clone();
        if let Some(semantics) = detail.semantics {
            successor.semantics = semantics;
        }
        if let Some(forge) = detail.forge {
            successor.authoring.forge = Some(forge);
        }
        if let Some(ranged) = detail.ranged {
            successor.authoring.ranged = Some(ranged);
        }
        if &successor == row {
            return Err(invalid("M25 Global Item Details unchanged row"));
        }
        profiles.insert(key.clone(), successor);
    }
    if !details.is_empty() {
        return Err(invalid("M25 Global Item Details source identity"));
    }
    Ok(profiles)
}

/// Exact pre-correction profiles used by previously issued artifacts.
pub(super) fn legacy_robes() -> Result<Vec<ItemProfileRecord>, ContentError> {
    serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/galthen-boss-loot-legacy-robes.json"
    ))
    .map_err(|_| invalid("M25 historical robe policy"))
}

fn qualify(raw: &[u8]) -> Result<Vec<ItemProfileRecord>, ContentError> {
    if hex(sha256(raw)) != pin::CAPTURE_SHA256 {
        return Err(invalid("M25 carried loot complete source hash"));
    }
    let document: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| invalid("M25 loot source packet"))?;
    let records: Vec<ItemProfileRecord> = serde_json::from_value(document["records"].clone())
        .map_err(|_| invalid("M25 complete carried loot policies"))?;
    if records.len() != IDS.len()
        || records
            .iter()
            .map(|row| row.production_binding.external_id.as_str())
            .ne(IDS)
        || records.iter().any(|row| {
            !row.admission.materializable
                || row.admission.legal_destinations
                    != [
                        NativeItemDestination::CharacterInventory,
                        NativeItemDestination::Ground,
                    ]
        })
    {
        return Err(invalid("M25 exact carried loot source set"));
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m25_loot_native_decoder_retains_source_speed_and_complete_protection()
    -> Result<(), ContentError> {
        use crate::content::item_abilities::{ItemAbility, item_ability_profile};
        use crate::content::{ReferenceItemField, ReferenceSignedPoints};
        let records = qualified_records()?;
        let mut boots_seen = false;
        for row in records {
            let view = item_ability_profile(&row.semantics)
                .map_err(|_| invalid("M25 loot native ability decoder"))?;
            let speed: i32 = view
                .abilities
                .iter()
                .filter_map(|ability| match ability {
                    ItemAbility::Speed(value) => Some(*value),
                    _ => None,
                })
                .sum();
            assert_eq!(Some(speed), row.attributes.speed_bonus);
            if row.production_binding.external_id == "44648" {
                boots_seen = true;
                assert!(view.abilities.contains(&ItemAbility::Speed(15)));
                assert!(matches!(&row.semantics.protection,
                    ReferenceItemField::Known(group)
                    if group.armor == ReferenceItemField::Known(ReferenceSignedPoints(2))));
            }
        }
        assert!(boots_seen);
        Ok(())
    }

    #[test]
    fn m25_loot_keeps_all_exact_existing_identities_and_rejects_changed_source()
    -> Result<(), ContentError> {
        for row in qualified_records()? {
            assert_eq!(
                row.production_binding.target.key,
                format!("oteryn:item.tibia.i{}", row.production_binding.external_id)
            );
            assert!(row.production_binding_qualification.is_none());
        }
        let mut changed = CAPTURE.to_vec();
        changed.push(b' ');
        assert!(qualify(&changed).is_err());
        Ok(())
    }
}
