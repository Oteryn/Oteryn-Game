//! Ahau's existing throne identity keeps its pinned, passable Ground-only policy.
use super::*;
#[path = "galthen_furniture_pin.rs"]
mod pin;

const CAPTURE: &[u8] = include_bytes!(
    "../../../../../../tools/content-schema/native-gameplay/galthen-boss-furniture.json"
);

pub(super) fn qualified_records() -> Result<Vec<ItemProfileRecord>, ContentError> {
    qualify(CAPTURE)
}

fn qualify(raw: &[u8]) -> Result<Vec<ItemProfileRecord>, ContentError> {
    if hex(sha256(raw)) != pin::CAPTURE_SHA256 {
        return Err(invalid("M25 throne complete source hash"));
    }
    let document: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| invalid("M25 throne source policy"))?;
    let records: Vec<ItemProfileRecord> = serde_json::from_value(document["records"].clone())
        .map_err(|_| invalid("M25 throne complete policy"))?;
    if records.len() != 2
        || records
            .iter()
            .map(|record| record.production_binding.external_id.as_str())
            .ne(["21287", "31483"])
    {
        return Err(invalid("M25 throne exact policy set"));
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m25_ahau_throne_keeps_exact_passable_ground_only_source_policy() -> Result<(), ContentError>
    {
        let records = qualified_records()?;
        let record = &records[1];
        assert_eq!(record.production_binding.external_id, "31483");
        assert_eq!(record.attributes.blocks_movement, Some(false));
        assert_eq!(record.attributes.blocks_projectile, Some(false));
        assert_eq!(
            record.admission.legal_destinations,
            [NativeItemDestination::Ground]
        );
        assert!(record.admission.materializable);
        let mut changed = CAPTURE.to_vec();
        changed.push(b' ');
        assert!(qualify(&changed).is_err());
        Ok(())
    }

    #[test]
    fn m25_mitmah_waiting_and_return_decoration_is_passable_ground_only() -> Result<(), ContentError>
    {
        let records = qualified_records()?;
        let record = &records[0];
        assert_eq!(record.production_binding.external_id, "21287");
        assert_eq!(record.attributes.blocks_movement, Some(false));
        assert_eq!(record.attributes.blocks_projectile, Some(false));
        assert_eq!(
            record.admission.legal_destinations,
            [NativeItemDestination::Ground]
        );
        Ok(())
    }
}
