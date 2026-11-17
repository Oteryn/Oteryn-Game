//! The Koshei source payment reuses the established fee and quest-clock participants.
use super::*;

#[test]
fn koshei_paid_hand_in_and_delayed_collection_are_source_bound() -> Result<(), String> {
    let callbacks = load()?;
    let start = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-start")
        .ok_or("missing Koshei atomic payment binding")?;
    assert_eq!(
        start.fee,
        Some(MonetaryFee {
            gold: 5000,
            include_bank: true
        })
    );
    assert_eq!(
        start
            .take
            .iter()
            .map(|i| (i.item.as_str(), i.count))
            .collect::<Vec<_>>(),
        (7528..=7531)
            .map(|n| (format!("oteryn:item.tibia.i{n}"), 1))
            .collect::<Vec<_>>()
            .iter()
            .map(|(s, n)| (s.as_str(), *n))
            .collect::<Vec<_>>()
    );
    assert!(
        start.give.is_none(),
        "the source starts work, it does not grant immediately"
    );
    assert_eq!(start.state_ops[0].effect, StateMutation::SetNow);
    let collect = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-collect")
        .ok_or("missing Koshei collection binding")?;
    assert_eq!(
        collect.state_ops[0].from,
        StateCondition::ElapsedAtLeast { seconds: 86400 }
    );
    assert_eq!(
        collect.give.as_ref().map(|g| (g.item.as_str(), g.count)),
        Some(("oteryn:item.tibia.i7532", 1))
    );
    assert!(collect.fee.is_none());
    assert!(
        !collect.completes,
        "amulet assembly does not complete the Koshei fight"
    );
    Ok(())
}
