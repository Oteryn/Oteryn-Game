#![allow(clippy::expect_used)]
use super::*;
use oteryn_protocol_oteryn::charm_progression::CharmRemoveDisposition;

#[test]
fn canonical_bestiary_binds_every_race_with_its_charm_points() {
    let bestiary = bestiary().expect("canonical Bestiary source");
    assert_eq!(bestiary.races.len(), 823);
    assert!(bestiary.races.iter().all(|race| race.final_kill_threshold() > 0));
}

#[test]
fn offer_inserts_capability_67_in_order_only_when_available() {
    let base = [
        OfferedCapability {
            id: 4,
            requires: &[],
        },
        OfferedCapability {
            id: 70,
            requires: &[],
        },
    ];
    let ids = |offers: Vec<OfferedCapability>| offers.iter().map(|c| c.id).collect::<Vec<_>>();
    assert_eq!(ids(offered(&base, true)), [4, 67, 70]);
    assert_eq!(ids(offered(&base, false)), [4, 70]);
    let twice = offered(&offered(&base, true), true);
    assert_eq!(ids(twice), [4, 67, 70]);
}

#[test]
fn reply_names_a_fee_only_for_a_completed_remove() {
    let reply = |outcome, fee_gold| {
        wire::encode_charm_progression_reply(&CharmProgressionReply {
            outcome,
            fee_gold,
            bestiary: Vec::new(),
            charms: Default::default(),
        })
    };
    assert!(reply(CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed), 100).is_ok());
    assert!(reply(CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed), 0).is_err());
    assert!(reply(CharmProgressionOutcome::Query, 100).is_err());
}
