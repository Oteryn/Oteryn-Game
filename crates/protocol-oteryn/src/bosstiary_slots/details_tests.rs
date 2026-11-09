use super::*;
#[test]
fn detailed_boss_slots_are_opt_in_and_preserve_legacy_receipts() {
    let request = BossSlotRequest::Detailed(Box::new(BossSlotRequest::Equip {
        slot: 1,
        boss: "oteryn:creature.oberon".into(),
    }));
    let bytes = request.encode().unwrap_or_default();
    assert_eq!(bytes.get(..3), Some([3, 1, 1].as_slice()));
    assert_eq!(BossSlotRequest::decode(&bytes), Some(request));
    assert!(BossSlotRequest::decode(&[3, 3, 0]).is_none());
    let mut reply = BossSlotReply {
        status: 0,
        unlocked: 3,
        bonus: 106,
        points: 1500,
        remove_fee: 100000,
        ready_at_ms: 1490,
        bosses: ["oteryn:creature.oberon".into(), String::new()],
        details: None,
    };
    let legacy = reply.encode().unwrap_or_default();
    assert_eq!(BossSlotReply::decode(&legacy), Some(reply.clone()));
    reply.details = Some(SlotDetails {
        base_bonus: 106,
        bonuses: [131, 0],
        boosted: "oteryn:creature.drume".into(),
        now: 1500,
    });
    let bytes = reply.encode().unwrap_or_default();
    assert_eq!(&bytes[..legacy.len()], legacy);
    assert_eq!(BossSlotReply::decode(&bytes), Some(reply));
    for end in legacy.len() + 1..bytes.len() {
        assert!(BossSlotReply::decode(&bytes[..end]).is_none());
    }
}
