#![allow(clippy::expect_used)]
use super::super::house_ownership::eligibility::Candidate;
use super::*;
use crate::foundation::WorldId;
use oteryn_gameplay_housing::house_auction::value::{plan_release_value, plan_winner_value};
use oteryn_gameplay_housing::house_auction::{Bid, eligibility::Eligibility};

#[test]
fn house_ownership_audit_uses_four_value_lines_and_the_registered_envelope() {
    let id = [0x01, 0x9a, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
    let plan = plan_winner_value(
        300,
        700,
        600,
        201,
        100,
        super::super::bank_audit::BANK_BALANCE_MAX,
    )
    .expect("valid auction fixture");
    let facts = Eligibility {
        active: true,
        account_matches: true,
        world_matches: true,
        level: 20,
        junior: false,
        slot_occupied: false,
        banned: false,
        premium_allowed: true,
    };
    let mut other = id;
    other[15] = 2;
    let eligible = [
        Candidate {
            bid: Bid {
                account: id,
                maximum: 600,
                maximum_reached_order: 1,
            },
            character: id,
            facts,
        },
        Candidate {
            bid: Bid {
                account: other,
                maximum: 200,
                maximum_reached_order: 2,
            },
            character: other,
            facts,
        },
    ];
    let write = ValueWrite {
        operation: id,
        auction: id,
        world: WorldId::decode(&id).expect("valid auction fixture"),
        house: "oteryn:content.house.test",
        account: id,
        character: id,
        kind: 2,
        binding: [7; 32],
        command: None,
        fence: None,
        maximum: 600,
        first_rent: 100,
        price: 201,
        reached: 1,
        ends_at_us: 1,
        now_us: 1000,
        plan: &plan,
        eligibility: &eligible,
        ownership_before: 1,
    };
    let wire = encode(&write, id).expect("valid auction fixture");
    assert!(wire.len() <= ENVELOPE_BYTES_MAX);
    let envelope = EventEnvelopeV1::decode(wire.as_slice()).expect("valid auction fixture");
    assert_eq!(envelope.event_type_id, 8);
    assert_eq!(envelope.retention_profile_id, "ECONOMY_LEDGER_RETENTION_V1");
    assert_eq!(
        envelope.payload_sha256,
        Sha256::digest(&envelope.payload).as_slice()
    );
    assert!(envelope.payload.len() <= PAYLOAD_BYTES_MAX);
    let payload =
        HouseOperation::decode(envelope.payload.as_slice()).expect("valid auction fixture");
    assert_eq!(
        payload.lines.len() + usize::from(payload.escrow_before != payload.escrow_after),
        4
    );
    assert_eq!(payload.price, 201);
    assert_eq!(payload.first_rent, 100);
    assert_eq!(payload.ownership_before, Some(1));
    assert_eq!(payload.ownership_after, Some(2));
    assert_eq!(payload.slot_before, Some(false));
    assert_eq!(payload.slot_after, Some(true));
    assert_eq!(
        payload.paid_until_unix_us,
        Some(1000 + oteryn_gameplay_housing::house_auction::FIRST_RENT_US)
    );
    let mut census: Vec<_> = (0_u16..256)
        .map(|n| {
            let mut account = id;
            account[14..].copy_from_slice(&n.to_be_bytes());
            super::super::house_ownership::eligibility::Candidate {
                bid: oteryn_gameplay_housing::house_auction::Bid {
                    account,
                    maximum: super::super::bank_audit::BANK_BALANCE_MAX,
                    maximum_reached_order: u64::from(n) + 1,
                },
                character: id,
                facts: oteryn_gameplay_housing::house_auction::eligibility::Eligibility {
                    active: true,
                    account_matches: false,
                    world_matches: true,
                    level: u32::MAX,
                    junior: false,
                    slot_occupied: false,
                    banned: false,
                    premium_allowed: true,
                },
            }
        })
        .collect();
    let full = ValueWrite {
        eligibility: &census,
        kind: 3,
        price: 0,
        plan: &plan_release_value(300, 700, super::super::bank_audit::BANK_BALANCE_MAX)
            .expect("captured release"),
        ..write
    };
    assert!(encode(&full, id).expect("bounded full census").len() <= ENVELOPE_BYTES_MAX);
    census.push(super::super::house_ownership::eligibility::Candidate {
        bid: census[0].bid,
        character: census[0].character,
        facts: census[0].facts,
    });
    let full = ValueWrite {
        eligibility: &census,
        ..write
    };
    assert!(matches!(
        encode(&full, id),
        Err(HouseOwnershipError::AuctionFull)
    ));
    let mut over = plan.clone();
    over.lines.push(over.lines[0]);
    let over = ValueWrite {
        plan: &over,
        ..write
    };
    assert!(matches!(
        encode(&over, id),
        Err(HouseOwnershipError::InvalidInput)
    ));
}
