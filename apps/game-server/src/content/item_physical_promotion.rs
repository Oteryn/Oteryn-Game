//! Source-qualified physical leaves, independent of identity admission and gameplay.
use super::{
    ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceItemField,
    ReferenceItemPhysical, ReferenceItemSemantics, ReferenceItemStack, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_PHYSICAL_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-physical-promotion-v1.json"
);
pub const ITEM_PHYSICAL_PACKET_SHA256: &str =
    "e09fd6094c566c72731731db84baa5dc6cb5ce59510736216bcb672d6d2987ba";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    promotions: usize,
    fields: usize,
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Kind {
    PickupableTrue,
    MovableTrue,
    #[serde(rename = "RUNE_STACK_100")]
    RuneStack100,
}
#[derive(Deserialize)]
struct Row {
    item_key: String,
    kind: Kind,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PhysicalPromotion {
    pub fields: usize,
    pub changed: usize,
}

pub fn apply_item_physical_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<PhysicalPromotion, String> {
    if world_project_sha256(ITEM_PHYSICAL_PACKET) != ITEM_PHYSICAL_PACKET_SHA256 {
        return Err("physical packet digest drift".into());
    }
    apply_rows(
        draft.core.records.iter_mut().filter_map(|r| match r {
            ProjectReferenceRecord::Item {
                identity,
                stack_class,
                semantics,
                ..
            } => Some((identity.key.as_str(), *stack_class, semantics)),
            _ => None,
        }),
        ITEM_PHYSICAL_PACKET,
        6794,
        6833,
    )
}

fn compatible<T: PartialEq>(field: &ReferenceItemField<T>, value: &T) -> Result<usize, String> {
    match field {
        ReferenceItemField::Unknown => Ok(1),
        ReferenceItemField::Known(old) if old == value => Ok(0),
        _ => Err("physical leaf blocked or conflicting".into()),
    }
}
fn validate(
    s: &ReferenceItemSemantics,
    class: ItemStackDocument,
    kind: Kind,
) -> Result<usize, String> {
    use ReferenceItemField::{Known, Unknown};
    match kind {
        Kind::PickupableTrue | Kind::MovableTrue => match &s.physical {
            Unknown => Ok(1),
            Known(p) => compatible(
                if kind == Kind::MovableTrue {
                    &p.movable
                } else {
                    &p.pickupable
                },
                &true,
            ),
            _ => Err("blocked physical group".into()),
        },
        Kind::RuneStack100 => {
            if class == ItemStackDocument::NonStackable {
                return Err("non-stackable identity conflict".into());
            }
            match &s.stack {
                Unknown => Ok(2),
                Known(p) => Ok(compatible(&p.stackable, &true)? + compatible(&p.stack_max, &100)?),
                _ => Err("blocked stack group".into()),
            }
        }
    }
}
pub(super) fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, ItemStackDocument, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
    expected: usize,
    expected_fields: usize,
) -> Result<PhysicalPromotion, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_PHYSICAL_PROMOTION/v1"
        || packet.counts.promotions != expected
        || packet.promotions.len() != expected
        || packet.counts.fields != expected_fields
    {
        return Err("physical packet shape/count drift".into());
    }
    let mut items: BTreeMap<_, _> = items.map(|(k, c, s)| (k, (c, s))).collect();
    let (mut seen, mut changed, mut fields) = (BTreeSet::new(), 0, 0);
    for row in &packet.promotions {
        if !seen.insert((&row.item_key, row.kind)) {
            return Err("duplicate physical row".into());
        }
        let (class, s) = items
            .get(row.item_key.as_str())
            .ok_or("missing physical Item")?;
        changed += validate(s, *class, row.kind)?;
        fields += if row.kind == Kind::RuneStack100 { 2 } else { 1 };
    }
    if fields != expected_fields {
        return Err("physical leaf count drift".into());
    }
    // The entire packet is validated before the first mutation. Preserve every sibling.
    for row in &packet.promotions {
        let (_, s) = items
            .get_mut(row.item_key.as_str())
            .ok_or("validated Item missing")?;
        match row.kind {
            Kind::PickupableTrue | Kind::MovableTrue => {
                if matches!(s.physical, ReferenceItemField::Unknown) {
                    s.physical = ReferenceItemField::Known(ReferenceItemPhysical {
                        weight: ReferenceItemField::Unknown,
                        movable: ReferenceItemField::Unknown,
                        pickupable: ReferenceItemField::Unknown,
                    });
                }
                if let ReferenceItemField::Known(p) = &mut s.physical {
                    if row.kind == Kind::MovableTrue {
                        p.movable = ReferenceItemField::Known(true);
                    } else {
                        p.pickupable = ReferenceItemField::Known(true);
                    }
                }
            }
            Kind::RuneStack100 => {
                s.stack = ReferenceItemField::Known(ReferenceItemStack {
                    stackable: ReferenceItemField::Known(true),
                    stack_max: ReferenceItemField::Known(100),
                })
            }
        }
    }
    Ok(PhysicalPromotion { fields, changed })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn packet(rows: serde_json::Value, fields: usize) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_PHYSICAL_PROMOTION/v1",
            "counts":{"promotions":rows.as_array().expect("rows").len(),"fields":fields},"promotions":rows})).expect("packet")
    }
    fn row(key: &str, kind: &str) -> serde_json::Value {
        serde_json::json!({"item_key":key,"kind":kind})
    }
    #[test]
    fn movable_true_preserves_siblings_and_rejects_conflicts_atomically() {
        let bytes = packet(serde_json::json!([row("i", "MOVABLE_TRUE")]), 1);
        let mut item = ReferenceItemSemantics {
            physical: Known(ReferenceItemPhysical {
                weight: Known(17),
                movable: Unknown,
                pickupable: Conflict,
            }),
            ..ReferenceItemSemantics::default()
        };
        for changed in [1, 0] {
            assert_eq!(
                apply_rows(
                    std::iter::once(("i", ItemStackDocument::Unknown, &mut item)),
                    &bytes,
                    1,
                    1
                ),
                Ok(PhysicalPromotion { fields: 1, changed })
            );
        }
        assert!(matches!(
            item.physical,
            Known(ReferenceItemPhysical {
                weight: Known(17),
                movable: Known(true),
                pickupable: Conflict
            })
        ));
        let bytes = packet(
            serde_json::json!([row("first", "MOVABLE_TRUE"), row("blocked", "MOVABLE_TRUE")]),
            2,
        );
        for movable in [Conflict, NotApplicable, Known(false)] {
            let mut first = ReferenceItemSemantics::default();
            let mut blocked = ReferenceItemSemantics {
                physical: Known(ReferenceItemPhysical {
                    weight: Unknown,
                    movable,
                    pickupable: Unknown,
                }),
                ..ReferenceItemSemantics::default()
            };
            let before = blocked.clone();
            assert!(
                apply_rows(
                    [
                        ("first", ItemStackDocument::Unknown, &mut first),
                        ("blocked", ItemStackDocument::Unknown, &mut blocked)
                    ]
                    .into_iter(),
                    &bytes,
                    2,
                    2
                )
                .is_err()
            );
            assert!(first.is_all_unknown());
            assert_eq!(blocked, before);
        }
    }

    #[test]
    fn preserves_siblings_and_identity_class_with_idempotent_positive_leaves() {
        let mut s = ReferenceItemSemantics {
            physical: Known(ReferenceItemPhysical {
                weight: Known(21),
                movable: Conflict,
                pickupable: Unknown,
            }),
            ..ReferenceItemSemantics::default()
        };
        let bytes = packet(
            serde_json::json!([row("i", "PICKUPABLE_TRUE"), row("i", "RUNE_STACK_100")]),
            3,
        );
        for changed in [3, 0] {
            assert_eq!(
                apply_rows(
                    std::iter::once(("i", ItemStackDocument::Unknown, &mut s)),
                    &bytes,
                    2,
                    3
                ),
                Ok(PhysicalPromotion { fields: 3, changed })
            );
        }
        assert!(matches!(
            s.physical,
            Known(ReferenceItemPhysical {
                weight: Known(21),
                movable: Conflict,
                ..
            })
        ));
    }
    #[test]
    fn rejects_blocked_groups_leaves_known_conflicts_and_class_conflicts() {
        for state in [Conflict, NotApplicable] {
            let s = ReferenceItemSemantics {
                physical: state,
                ..ReferenceItemSemantics::default()
            };
            assert!(validate(&s, ItemStackDocument::Unknown, Kind::PickupableTrue).is_err());
        }
        for pickupable in [Conflict, NotApplicable, Known(false)] {
            let s = ReferenceItemSemantics {
                physical: Known(ReferenceItemPhysical {
                    weight: Unknown,
                    movable: Unknown,
                    pickupable,
                }),
                ..ReferenceItemSemantics::default()
            };
            assert!(validate(&s, ItemStackDocument::Unknown, Kind::PickupableTrue).is_err());
        }
        let bytes = packet(serde_json::json!([row("i", "RUNE_STACK_100")]), 2);
        for state in [
            Conflict,
            NotApplicable,
            Known(ReferenceItemStack {
                stackable: Known(false),
                stack_max: Unknown,
            }),
            Known(ReferenceItemStack {
                stackable: Unknown,
                stack_max: Known(99),
            }),
            Known(ReferenceItemStack {
                stackable: Conflict,
                stack_max: Unknown,
            }),
        ] {
            let mut s = ReferenceItemSemantics {
                stack: state,
                ..ReferenceItemSemantics::default()
            };
            let before = s.clone();
            assert!(
                apply_rows(
                    std::iter::once(("i", ItemStackDocument::Unknown, &mut s)),
                    &bytes,
                    1,
                    2
                )
                .is_err()
            );
            assert_eq!(s, before);
        }
        let mut s = ReferenceItemSemantics::default();
        assert!(
            apply_rows(
                std::iter::once(("i", ItemStackDocument::NonStackable, &mut s)),
                &bytes,
                1,
                2
            )
            .is_err()
        );
        assert!(s.is_all_unknown());
    }
    #[test]
    fn invalid_duplicate_or_missing_late_row_rejects_without_partial_mutation() {
        for rows in [
            serde_json::json!([
                row("i", "PICKUPABLE_TRUE"),
                row("missing", "PICKUPABLE_TRUE")
            ]),
            serde_json::json!([row("i", "PICKUPABLE_TRUE"), row("i", "PICKUPABLE_TRUE")]),
            serde_json::json!([row("i", "PICKUPABLE_TRUE"), row("i", "INVALID")]),
        ] {
            let mut s = ReferenceItemSemantics::default();
            assert!(
                apply_rows(
                    std::iter::once(("i", ItemStackDocument::Unknown, &mut s)),
                    &packet(rows, 2),
                    2,
                    2
                )
                .is_err()
            );
            assert!(s.is_all_unknown());
        }
    }
}
