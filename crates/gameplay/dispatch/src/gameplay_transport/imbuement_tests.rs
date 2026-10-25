#![allow(clippy::expect_used)]
use super::*;
use crate::content::{
    ReferenceImbuementAllowance, ReferenceImbuementFamily, ReferenceImbuementTier,
};

fn content(
    slots: ReferenceItemField<u8>,
    allowed: ReferenceItemField<Vec<ReferenceImbuementAllowance>>,
    excluded: ReferenceItemField<Vec<ReferenceImbuementFamily>>,
) -> ReferenceItemField<ReferenceItemImbuement> {
    ReferenceItemField::Known(ReferenceItemImbuement {
        slot_count: slots,
        allowed_family_tiers: allowed,
        excluded_families: excluded,
    })
}

fn allowed(
    families: &[ReferenceImbuementFamily],
) -> ReferenceItemField<Vec<ReferenceImbuementAllowance>> {
    ReferenceItemField::Known(
        families
            .iter()
            .map(|&family| ReferenceImbuementAllowance {
                family,
                tier: ReferenceImbuementTier::Three,
            })
            .collect(),
    )
}

/// The reply of a decision; an item-gated write that only shrine access still decides is
/// `REQUIREMENT_NOT_MET` without a shrine in reach.
fn result(decision: Decision) -> ImbuementResult {
    match decision {
        Decision::Reply(reply) => reply.result,
        Decision::Apply { .. } | Decision::Clear { .. } => ImbuementResult::RequirementNotMet,
    }
}

fn row(slot: u8, category: u8) -> StoredImbuement {
    StoredImbuement {
        slot,
        category,
        tier: 1,
        remaining_ms: 1_000,
    }
}

/// Two slots offering Critical Hit (family 1) and Life Leech (family 11) at every tier.
fn two_slots() -> Allowance {
    allowance(&content(
        ReferenceItemField::Known(2),
        allowed(&[
            ReferenceImbuementFamily::LifeLeech,
            ReferenceImbuementFamily::CriticalHit,
            ReferenceImbuementFamily::LifeLeech,
        ]),
        ReferenceItemField::NotApplicable,
    ))
    .expect("slot-bearing")
}

#[test]
fn imbuement_allowance_fails_closed_on_unknown_content() {
    assert_eq!(allowance(&ReferenceItemField::Unknown), None);
    assert_eq!(allowance(&ReferenceItemField::NotApplicable), None);
    for slots in [
        ReferenceItemField::Unknown,
        ReferenceItemField::Known(0),
        ReferenceItemField::Known(4),
    ] {
        assert_eq!(
            allowance(&content(
                slots,
                ReferenceItemField::Unknown,
                ReferenceItemField::Unknown
            )),
            None
        );
    }
    // The shipped content: known slots, unknown families. The slots show, nothing is offered.
    let shipped = allowance(&content(
        ReferenceItemField::Known(3),
        ReferenceItemField::Unknown,
        ReferenceItemField::Unknown,
    ))
    .expect("slots are known");
    assert_eq!((shipped.slot_count, shipped.offers.len()), (3, 0));
    // Known allowances with unknown exclusions still offer nothing.
    let unknown_exclusions = allowance(&content(
        ReferenceItemField::Known(1),
        allowed(&[ReferenceImbuementFamily::CriticalHit]),
        ReferenceItemField::Unknown,
    ))
    .expect("slots are known");
    assert!(unknown_exclusions.offers.is_empty());
    let excluded = allowance(&content(
        ReferenceItemField::Known(1),
        allowed(&[
            ReferenceImbuementFamily::CriticalHit,
            ReferenceImbuementFamily::SkillSword,
        ]),
        ReferenceItemField::Known(vec![ReferenceImbuementFamily::CriticalHit]),
    ))
    .expect("slots are known");
    assert!(excluded.offers.iter().all(|offer| offer.family == 20));
}

#[test]
fn imbuement_offers_every_tier_of_each_admitted_family_at_canary_prices() {
    let allowance = two_slots();
    let offers: Vec<_> = allowance
        .offers
        .iter()
        .map(|o| (o.family, o.tier, o.gold, o.requires_premium))
        .collect();
    assert_eq!(
        offers,
        [
            (1, 1, 5_000, false),
            (1, 2, 30_000, true),
            (1, 3, 200_000, true),
            (11, 1, 5_000, false),
            (11, 2, 30_000, true),
            (11, 3, 200_000, true),
        ]
    );
    let Decision::Reply(reply) = decide(ImbuementAction::Query, Some(&allowance), &[row(1, 10)])
    else {
        panic!("a query is answered at once");
    };
    assert_eq!(reply.result, ImbuementResult::Ok);
    let window = reply.window.expect("query window");
    assert_eq!((window.slot_count, window.clear_gold), (2, 15_000));
    assert_eq!(window.applied[0].family, 11);
    assert_eq!(window.offers, allowance.offers);
    wire::encode_imbuement_reply(&wire::ImbuementReply {
        result: ImbuementResult::Ok,
        window: Some(window),
    })
    .expect("a decided window is always encodable");
}

#[test]
fn imbuement_apply_gates_slot_offer_occupancy_category_then_access() {
    let allowance = two_slots();
    let apply = |slot, imbuement, applied: &[StoredImbuement]| {
        result(decide(
            ImbuementAction::Apply { slot, imbuement },
            Some(&allowance),
            applied,
        ))
    };
    // Slot beyond the item's slots and an index beyond its offers.
    assert_eq!(apply(2, 0, &[]), ImbuementResult::NotAllowed);
    assert_eq!(apply(0, 6, &[]), ImbuementResult::NotAllowed);
    // An occupied slot, then a second imbuement of an applied category on another slot.
    assert_eq!(apply(0, 0, &[row(0, 10)]), ImbuementResult::SlotOccupied);
    assert_eq!(apply(1, 4, &[row(0, 10)]), ImbuementResult::NotAllowed);
    // Every item gate passed: the write carries the slot and the chosen offer.
    assert_eq!(
        decide(
            ImbuementAction::Apply {
                slot: 1,
                imbuement: 0
            },
            Some(&allowance),
            &[row(0, 10)]
        ),
        Decision::Apply {
            slot: 1,
            offer: allowance.offers[0]
        }
    );
    assert_eq!(apply(1, 5, &[]), ImbuementResult::RequirementNotMet);
    // No eligible item at all.
    assert_eq!(
        result(decide(
            ImbuementAction::Apply {
                slot: 0,
                imbuement: 0
            },
            None,
            &[]
        )),
        ImbuementResult::NotAllowed
    );
}

#[test]
fn imbuement_clear_needs_an_occupied_slot_inside_the_item_then_access() {
    let allowance = two_slots();
    let clear = |slot, applied: &[StoredImbuement]| {
        result(decide(
            ImbuementAction::Clear { slot },
            Some(&allowance),
            applied,
        ))
    };
    assert_eq!(clear(0, &[]), ImbuementResult::NotAllowed);
    assert_eq!(clear(2, &[row(0, 0)]), ImbuementResult::NotAllowed);
    assert_eq!(clear(1, &[row(0, 0)]), ImbuementResult::NotAllowed);
    assert_eq!(
        decide(ImbuementAction::Clear { slot: 0 }, Some(&allowance), &[row(0, 0)]),
        Decision::Clear { slot: 0 }
    );
    assert_eq!(
        result(decide(ImbuementAction::Query, None, &[])),
        ImbuementResult::NotAllowed
    );
}

#[test]
fn imbuement_shrine_keys_are_sorted_for_binary_search() {
    assert!(SHRINE_KEYS.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(SHRINE_KEYS.binary_search(&"oteryn:item.tibia.i25060").is_ok());
}

#[test]
fn imbuement_allowance_of_activated_native_content_follows_generated_slots() {
    use crate::content::native_gameplay::test_support::{
        activated_with_item_keys, activated_with_loot_tables,
    };
    let world = activated_with_item_keys(None)
        .active()
        .expect("active")
        .identity()
        .world_id();
    let (controller, _) = activated_with_loot_tables(world, None);
    let active = controller.active().expect("active");
    let native = active.native_gameplay().expect("native");
    let policy = |key: &str| {
        let record = native
            .item_policy(key, "definition-r1")
            .expect(key)
            .record();
        assert!(record.admission.materializable, "{key}");
        allowance(&record.semantics.imbuement)
    };
    // The sword and the backpack carry their Canary `imbuementslot` facts.
    let sword = policy("oteryn:item.tibia.i3264").expect("sword slots");
    assert_eq!(sword.slot_count, 2);
    let families: Vec<_> = sword.offers.iter().map(|offer| offer.family).collect();
    assert_eq!(families, [1, 1, 1, 2, 2, 2, 11, 11, 11, 12, 12, 12, 20, 20, 20]);
    let backpack = policy("oteryn:item.tibia.i2854").expect("backpack slot");
    assert_eq!(backpack.slot_count, 1);
    // A wooden shield and gold coins have no slot: every action is the same NOT_ALLOWED.
    for key in ["oteryn:item.tibia.i3412", "oteryn:item.tibia.i3031"] {
        let allowance = policy(key);
        assert_eq!(allowance, None, "{key}");
        assert_eq!(
            decide(ImbuementAction::Query, allowance.as_ref(), &[]),
            Decision::Reply(wire::ImbuementReply {
                result: ImbuementResult::NotAllowed,
                window: None,
            }),
            "{key}"
        );
    }
}
