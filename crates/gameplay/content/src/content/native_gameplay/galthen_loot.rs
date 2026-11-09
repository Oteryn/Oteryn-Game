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
