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
    let reply = decide(ImbuementAction::Query, Some(&allowance), &[row(1, 10)]);
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
        let reply = decide(
            ImbuementAction::Apply { slot, imbuement },
            Some(&allowance),
            applied,
        );
        assert!(reply.window.is_none());
        reply.result
    };
    // Slot beyond the item's slots and an index beyond its offers.
    assert_eq!(apply(2, 0, &[]), ImbuementResult::NotAllowed);
    assert_eq!(apply(0, 6, &[]), ImbuementResult::NotAllowed);
    // An occupied slot, then a second imbuement of an applied category on another slot.
    assert_eq!(apply(0, 0, &[row(0, 10)]), ImbuementResult::SlotOccupied);
    assert_eq!(apply(1, 4, &[row(0, 10)]), ImbuementResult::NotAllowed);
    // Every item gate passed: shrine access fails closed before any transaction.
    assert_eq!(
        apply(1, 0, &[row(0, 10)]),
        ImbuementResult::RequirementNotMet
    );
    assert_eq!(apply(1, 5, &[]), ImbuementResult::RequirementNotMet);
    // No eligible item at all.
    assert_eq!(
        decide(
            ImbuementAction::Apply {
                slot: 0,
                imbuement: 0
            },
            None,
            &[]
        )
        .result,
        ImbuementResult::NotAllowed
    );
}

#[test]
fn imbuement_clear_needs_an_occupied_slot_inside_the_item_then_access() {
    let allowance = two_slots();
    let clear = |slot, applied: &[StoredImbuement]| {
        decide(ImbuementAction::Clear { slot }, Some(&allowance), applied).result
    };
    assert_eq!(clear(0, &[]), ImbuementResult::NotAllowed);
    assert_eq!(clear(2, &[row(0, 0)]), ImbuementResult::NotAllowed);
    assert_eq!(clear(1, &[row(0, 0)]), ImbuementResult::NotAllowed);
    assert_eq!(clear(0, &[row(0, 0)]), ImbuementResult::RequirementNotMet);
    assert_eq!(
        decide(ImbuementAction::Query, None, &[]).result,
        ImbuementResult::NotAllowed
    );
}

#[test]
fn imbuement_allowance_of_activated_native_content_fails_closed() {
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
    // Sword, wooden shield, backpack and gold coins: materializable, but the qualified
    // native item policies do not declare imbuement semantics yet, so no item shows slots
    // and every action is the same NOT_ALLOWED.
    for key in [
        "oteryn:item.tibia.i3264",
        "oteryn:item.tibia.i3412",
        "oteryn:item.tibia.i2854",
        "oteryn:item.tibia.i3031",
    ] {
        let record = native
            .item_policy(key, "definition-r1")
            .expect(key)
            .record();
        assert!(record.admission.materializable, "{key}");
        assert_eq!(
            record.semantics.imbuement,
            ReferenceItemField::Unknown,
            "{key}"
        );
        let allowance = allowance(&record.semantics.imbuement);
        assert_eq!(allowance, None, "{key}");
        assert_eq!(
            decide(ImbuementAction::Query, allowance.as_ref(), &[]),
            wire::ImbuementReply {
                result: ImbuementResult::NotAllowed,
                window: None,
            },
            "{key}"
        );
    }
}
