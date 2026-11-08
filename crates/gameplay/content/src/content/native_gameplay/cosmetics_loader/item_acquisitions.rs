//! Source acquisition is opt-in data, independently promoted from cosmetic grant mappings.
use super::{CosmeticsDiagnostic, ProductionKey, Result, SourceCitation, hash, text};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Admission {
    item_key: String,
    donor: String,
    catalogue_sha256: String,
    decision: String,
    source: SourceCitation,
}

fn ascii(value: &str, max: usize) -> bool {
    text(value, max) && value.bytes().all(|b| (32..=126).contains(&b))
}

pub(super) fn validate(rows: &[Admission]) -> Result<()> {
    let mut keys = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        if rows.len() > 128
            || row
                .item_key
                .strip_prefix("oteryn:item.")
                .is_none_or(str::is_empty)
            || ProductionKey::new(&row.item_key).is_err()
            || !keys.insert(row.item_key.as_str())
            || !ascii(&row.donor, 1024)
            || hash(&row.catalogue_sha256).is_none()
            || !row.decision.starts_with("owner:")
            || row.decision.len() == "owner:".len()
            || !ascii(&row.decision, 1024)
            || !row.source.valid()
            || !ascii(&row.source.reference, 1024)
            || !ascii(&row.source.revision, 128)
        {
            return Err(CosmeticsDiagnostic::Definition {
                family: "ItemAcquisition",
                index,
            });
        }
    }
    Ok(())
}

pub(super) fn allows(
    rows: &[Admission],
    item_key: &str,
    donor: &str,
    catalogue_sha256: [u8; 32],
) -> bool {
    rows.iter().any(|row| {
        row.item_key == item_key
            && row.donor == donor
            && hash(&row.catalogue_sha256) == Some(catalogue_sha256)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row() -> Admission {
        Admission {
            item_key: "oteryn:item.tibia.i6579".into(),
            donor: "source:addon_doll.lua:67".into(),
            catalogue_sha256: "11".repeat(32),
            decision: "owner:source-acquisition".into(),
            source: SourceCitation {
                reference: "source:addon_doll.lua".into(),
                revision: "source-r1".into(),
                sha256: "22".repeat(32),
            },
        }
    }
    #[test]
    fn source_admission_requires_exact_item_donor_and_catalogue() {
        assert!(!allows(
            &[],
            "oteryn:item.tibia.i6579",
            "source:addon_doll.lua:67",
            [0x11; 32]
        ));
        let rows = [row()];
        assert!(validate(&rows).is_ok());
        assert!(allows(
            &rows,
            "oteryn:item.tibia.i6579",
            "source:addon_doll.lua:67",
            [0x11; 32]
        ));
        assert!(!allows(
            &rows,
            "oteryn:item.tibia.i37746",
            "source:addon_doll.lua:67",
            [0x11; 32]
        ));
        assert!(!allows(
            &rows,
            "oteryn:item.tibia.i6579",
            "source:other.lua:67",
            [0x11; 32]
        ));
        assert!(!allows(
            &rows,
            "oteryn:item.tibia.i6579",
            "source:addon_doll.lua:67",
            [0x12; 32]
        ));
    }
    #[test]
    fn source_admissions_are_bounded_unique_and_source_bound() {
        let mut rows: Vec<_> = (0..128)
            .map(|i| {
                let mut r = row();
                r.item_key = format!("oteryn:item.tibia.i{i}");
                r
            })
            .collect();
        assert!(validate(&rows).is_ok());
        rows.push(row());
        assert!(validate(&rows).is_err());
        assert!(validate(&[row(), row()]).is_err());
        let mut bad = row();
        bad.source.sha256 = "0".repeat(64);
        assert!(validate(&[bad]).is_err());
        let mut bad = row();
        bad.catalogue_sha256 = "0".repeat(64);
        assert!(validate(&[bad]).is_err());
        let mut bad = row();
        bad.decision = "implicit".into();
        assert!(validate(&[bad]).is_err());
        let mut bad = row();
        bad.item_key = "oteryn:creature.native".into();
        assert!(validate(&[bad]).is_err());
        let mut bad = row();
        bad.donor.clear();
        assert!(validate(&[bad]).is_err());
    }
    #[test]
    fn source_policy_requires_independent_owner_promotion_in_native_packet()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use crate::content::native_gameplay::{native_cosmetics, test_support::cosmetics_fixture};
        let mut packet = cosmetics_fixture::cosmetics_test_packet();
        let rows = serde_json::json!([{
            "item_key":"oteryn:item.tibia.i6579", "donor":"source:addon_doll.lua:67",
            "catalogue_sha256":"11".repeat(32), "decision":"owner:source-acquisition",
            "source":{"reference":"source:addon_doll.lua","revision":"source-r1","sha256":"22".repeat(32)}
        }]);
        packet["authoring"]["document"]["item_acquisitions"] = rows.clone();
        assert!(native_cosmetics::decode(&serde_json::to_vec(&packet)?).is_err());
        let mut promotion = packet["authoring"]["promotions"][0].clone();
        promotion["path"] = "/item_acquisitions".into();
        promotion["value"] = rows;
        promotion["source_kind"] = "OWNER_DECISION".into();
        packet["authoring"]["promotions"]
            .as_array_mut()
            .ok_or("promotions")?
            .push(promotion);
        let view = native_cosmetics::decode(&serde_json::to_vec(&packet)?)
            .map_err(|e| format!("{e:?}"))?
            .ok_or("authored view")?;
        assert!(view.allows_item_acquisition(
            "oteryn:item.tibia.i6579",
            "source:addon_doll.lua:67",
            [0x11; 32]
        ));
        let last = packet["authoring"]["promotions"]
            .as_array_mut()
            .ok_or("promotions")?
            .last_mut()
            .ok_or("admission promotion")?;
        last["source_kind"] = "STRUCTURED_REFERENCE_DATA".into();
        assert!(native_cosmetics::decode(&serde_json::to_vec(&packet)?).is_err());
        Ok(())
    }
}
