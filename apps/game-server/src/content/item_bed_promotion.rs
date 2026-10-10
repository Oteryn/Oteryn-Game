//! Bed facts (BED-CONTENT-1): group 19 on the Canary bed Items.
//!
//! The pinned packet `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json` is lowered by
//! `lower_bed_packet.py` from the pinned Canary `items.xml` (OTS_HYPOTHESIS_ONLY) under
//! ITEM-SEM-BED-PACKET-1 section 1.5. The materializer applies it once, after the earlier item
//! promotions. Rows are decoded strictly (exact shape, closed enums); each names an existing Item
//! record, an Item gets one `bed` row, and both occupied targets name existing Item records. An
//! occupied target equal to the row's own Item means "no change". The set rule of section 1.2
//! (every other occupied target carries group 19 with the same part and direction) is checked here
//! and again when the artifact compiles. Anything else fails closed.

use super::{
    ProjectReferenceRecord, ProjectV2Draft, ReferenceBedDirection, ReferenceBedPart,
    ReferenceItemBed, ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget,
    world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The pinned packet bytes; any change is a new candidate with a new digest.
pub const ITEM_BED_PROMOTION_V1_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261005-bed-facts-v1.json");
pub const ITEM_BED_PROMOTION_V1_PACKET_SHA256: &str =
    "0278941b6da2637c4dbc1117068ef9155350001ce7e4d8e01db3586012b41bbe";
pub const ITEM_BED_PROMOTION_V1_FIELD_COUNT: usize = 359;
pub const ITEM_BED_PROMOTION_V1_ITEM_COUNT: usize = 359;
const SCHEMA: &str = "OTERYN_ITEM_BED_PROMOTION/v1";
const DEFINITION_REVISION: &str = "definition-r1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemBedPromotionError {
    Digest,
    Decode(String),
    Row {
        item_key: String,
        reason: &'static str,
    },
    Counts,
}

impl std::fmt::Display for ItemBedPromotionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest => formatter.write_str("item bed promotion packet digest mismatch"),
            Self::Decode(error) => write!(formatter, "item bed promotion packet: {error}"),
            Self::Row { item_key, reason } => {
                write!(formatter, "item bed promotion {item_key}: {reason}")
            }
            Self::Counts => formatter.write_str("item bed promotion counts drifted"),
        }
    }
}

impl std::error::Error for ItemBedPromotionError {}

/// What one application changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemBedPromotion {
    pub fields: usize,
    pub items: usize,
    /// Rows whose `bed` already held a different value (an earlier promotion).
    pub replaced: usize,
}

#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}

#[derive(Deserialize)]
struct Counts {
    fields: usize,
    items: usize,
}

#[derive(Deserialize)]
struct Row {
    item_key: String,
    field_path: String,
    typed_value: TypedValue,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
enum TypedValue {
    Bed(BedValue),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BedValue {
    part: Part,
    partner_direction: Direction,
    occupied_male: String,
    occupied_female: String,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Part {
    Head,
    Foot,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Direction {
    North,
    East,
    South,
    West,
}

/// Apply the pinned packet to every Item record in `draft`.
pub fn apply_item_bed_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<ItemBedPromotion, ItemBedPromotionError> {
    if world_project_sha256(ITEM_BED_PROMOTION_V1_PACKET) != ITEM_BED_PROMOTION_V1_PACKET_SHA256 {
        return Err(ItemBedPromotionError::Digest);
    }
    let applied = apply_packet(draft, ITEM_BED_PROMOTION_V1_PACKET)?;
    if applied.fields != ITEM_BED_PROMOTION_V1_FIELD_COUNT
        || applied.items != ITEM_BED_PROMOTION_V1_ITEM_COUNT
    {
        return Err(ItemBedPromotionError::Counts);
    }
    Ok(applied)
}

fn apply_packet(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
) -> Result<ItemBedPromotion, ItemBedPromotionError> {
    let keys = draft
        .core
        .records
        .iter()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity.key.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } => Some((identity.key.as_str(), semantics)),
            _ => None,
        });
    apply_rows(items, &keys, bytes)
}

fn row_error(row: &Row, reason: &'static str) -> ItemBedPromotionError {
    ItemBedPromotionError::Row {
        item_key: row.item_key.clone(),
        reason,
    }
}

/// Apply a packet to `(Item key, semantics)` pairs; every row must name one of them and every
/// occupied target must be one of `keys`.
fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, &'a mut ReferenceItemSemantics)>,
    keys: &BTreeSet<String>,
    bytes: &[u8],
) -> Result<ItemBedPromotion, ItemBedPromotionError> {
    let packet: Packet = serde_json::from_slice(bytes)
        .map_err(|error| ItemBedPromotionError::Decode(error.to_string()))?;
    if packet.schema != SCHEMA {
        return Err(ItemBedPromotionError::Decode("schema".to_owned()));
    }
    let mut by_item: BTreeMap<&str, &Row> = BTreeMap::new();
    for row in &packet.promotions {
        if row.field_path != "bed" {
            return Err(row_error(row, "unknown field path"));
        }
        if by_item.insert(row.item_key.as_str(), row).is_some() {
            return Err(row_error(row, "duplicate bed row"));
        }
    }
    if packet.counts.fields != packet.promotions.len() || packet.counts.items != by_item.len() {
        return Err(ItemBedPromotionError::Counts);
    }
    // The set rule: every occupied target other than the row's own Item is itself a row of the
    // packet with the same part and direction.
    for (key, row) in &by_item {
        let TypedValue::Bed(bed) = &row.typed_value;
        for target in [&bed.occupied_male, &bed.occupied_female] {
            if target == key {
                continue;
            }
            let Some(other) = by_item.get(target.as_str()) else {
                return Err(row_error(row, "occupied target carries no bed group"));
            };
            let TypedValue::Bed(other) = &other.typed_value;
            if (other.part as u8, other.partner_direction as u8)
                != (bed.part as u8, bed.partner_direction as u8)
            {
                return Err(row_error(
                    row,
                    "occupied target differs in part or direction",
                ));
            }
        }
    }

    let mut applied = ItemBedPromotion {
        fields: 0,
        items: 0,
        replaced: 0,
    };
    for (key, semantics) in items {
        let Some(row) = by_item.remove(key) else {
            continue;
        };
        let TypedValue::Bed(value) = &row.typed_value;
        let target = |wire: &str| {
            if !keys.contains(wire) {
                return Err(row_error(row, "occupied target is not an Item record"));
            }
            ReferenceItemTarget::new(wire, DEFINITION_REVISION)
                .map_err(|_| row_error(row, "occupied target is not a definition reference"))
        };
        let bed = ReferenceItemBed {
            part: match value.part {
                Part::Head => ReferenceBedPart::Head,
                Part::Foot => ReferenceBedPart::Foot,
            },
            partner_direction: match value.partner_direction {
                Direction::North => ReferenceBedDirection::North,
                Direction::East => ReferenceBedDirection::East,
                Direction::South => ReferenceBedDirection::South,
                Direction::West => ReferenceBedDirection::West,
            },
            occupied_male: target(&value.occupied_male)?,
            occupied_female: target(&value.occupied_female)?,
        };
        if matches!(&semantics.bed, ReferenceItemField::Known(old) if *old != bed) {
            applied.replaced += 1;
        }
        semantics.bed = ReferenceItemField::Known(bed);
        applied.fields += 1;
        applied.items += 1;
    }
    if let Some((item_key, _)) = by_item.into_iter().next() {
        return Err(ItemBedPromotionError::Row {
            item_key: item_key.to_owned(),
            reason: "no Item record with this key",
        });
    }
    Ok(applied)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const HEAD: &str = "oteryn:item.tibia.i100";
    const HEAD_OCCUPIED: &str = "oteryn:item.tibia.i101";

    fn keys() -> BTreeSet<String> {
        [HEAD, HEAD_OCCUPIED]
            .map(str::to_owned)
            .into_iter()
            .collect()
    }

    fn row(key: &str, part: &str, direction: &str, male: &str, female: &str) -> String {
        format!(
            r#"{{"item_key":"{key}","field_path":"bed","typed_value":{{"kind":"BED","value":{{"part":"{part}","partner_direction":"{direction}","occupied_male":"{male}","occupied_female":"{female}"}}}}}}"#
        )
    }

    fn packet(rows: &[String]) -> Vec<u8> {
        format!(
            r#"{{"schema":"{SCHEMA}","counts":{{"fields":{n},"items":{n}}},"promotions":[{rows}]}}"#,
            n = rows.len(),
            rows = rows.join(",")
        )
        .into_bytes()
    }

    fn apply(
        bytes: &[u8],
        semantics: &mut [(&'static str, ReferenceItemSemantics)],
    ) -> Result<ItemBedPromotion, ItemBedPromotionError> {
        apply_rows(
            semantics.iter_mut().map(|(key, value)| (*key, value)),
            &keys(),
            bytes,
        )
    }

    fn pair() -> [(&'static str, ReferenceItemSemantics); 2] {
        [
            (HEAD, ReferenceItemSemantics::default()),
            (HEAD_OCCUPIED, ReferenceItemSemantics::default()),
        ]
    }

    #[test]
    fn applies_a_bed_group_with_own_and_other_targets() {
        let bytes = packet(&[
            row(HEAD, "HEAD", "SOUTH", HEAD_OCCUPIED, HEAD),
            row(HEAD_OCCUPIED, "HEAD", "SOUTH", HEAD_OCCUPIED, HEAD_OCCUPIED),
        ]);
        let mut items = pair();
        let applied = apply(&bytes, &mut items).expect("applies");
        assert_eq!((applied.fields, applied.items), (2, 2));
        let ReferenceItemField::Known(bed) = &items[0].1.bed else {
            panic!("bed set");
        };
        assert_eq!(bed.part, ReferenceBedPart::Head);
        assert_eq!(bed.partner_direction, ReferenceBedDirection::South);
        assert_eq!(bed.occupied_female.key, HEAD);
        assert_eq!(bed.occupied_male.key, HEAD_OCCUPIED);
    }

    #[test]
    fn set_rule_rejects_a_target_with_another_direction() {
        let bytes = packet(&[
            row(HEAD, "HEAD", "SOUTH", HEAD_OCCUPIED, HEAD),
            row(HEAD_OCCUPIED, "HEAD", "NORTH", HEAD_OCCUPIED, HEAD_OCCUPIED),
        ]);
        assert!(matches!(
            apply(&bytes, &mut pair()),
            Err(ItemBedPromotionError::Row { .. })
        ));
    }

    #[test]
    fn set_rule_rejects_a_target_without_a_bed_row() {
        let bytes = packet(&[row(HEAD, "HEAD", "SOUTH", HEAD_OCCUPIED, HEAD)]);
        assert!(matches!(
            apply(&bytes, &mut pair()),
            Err(ItemBedPromotionError::Row { .. })
        ));
    }

    #[test]
    fn rejects_unknown_target_duplicate_and_unknown_item() {
        let missing = format!("oteryn:item.tibia.i{}", 9);
        let unknown = packet(&[row(HEAD, "HEAD", "SOUTH", &missing, HEAD)]);
        assert!(apply(&unknown, &mut pair()).is_err());
        let duplicate = packet(&[
            row(HEAD, "HEAD", "SOUTH", HEAD, HEAD),
            row(HEAD, "HEAD", "SOUTH", HEAD, HEAD),
        ]);
        assert!(apply(&duplicate, &mut pair()).is_err());
        let one = packet(&[row(HEAD, "HEAD", "SOUTH", HEAD, HEAD)]);
        let mut only_other = [(HEAD_OCCUPIED, ReferenceItemSemantics::default())];
        assert!(apply(&one, &mut only_other).is_err());
    }

    #[test]
    fn decode_is_strict() {
        let bytes = packet(&[row(HEAD, "PILLOW", "SOUTH", HEAD, HEAD)]);
        assert!(matches!(
            apply(&bytes, &mut pair()),
            Err(ItemBedPromotionError::Decode(_))
        ));
    }

    #[test]
    fn pinned_packet_has_the_pinned_digest_and_counts() {
        assert_eq!(
            world_project_sha256(ITEM_BED_PROMOTION_V1_PACKET),
            ITEM_BED_PROMOTION_V1_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_BED_PROMOTION_V1_PACKET).expect("packet decodes");
        assert_eq!(packet.counts.fields, ITEM_BED_PROMOTION_V1_FIELD_COUNT);
        assert_eq!(packet.promotions.len(), ITEM_BED_PROMOTION_V1_FIELD_COUNT);
    }
}
