use super::*;
#[test]
fn player_trade_book_capacity_and_identity_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    use super::super::capabilities::{ITEM_VIEW_OFFERED_CAPABILITIES, SelectedCapabilities};
    for supported in [&[4, 6][..], &[6, 60][..]] {
        let legacy = SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, supported)
            .ok_or("selection")?;
        assert!(!legacy.command_selected(wire::COMMAND_TYPE_PLAYER_TRADE_INTENT));
        assert!(!legacy.domain_selected(wire::STATE_DOMAIN_PLAYER_TRADE));
    }
    let modern = SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &[4, 6, 60])
        .ok_or("trade selection")?;
    assert!(modern.command_selected(wire::COMMAND_TYPE_PLAYER_TRADE_INTENT));
    assert!(modern.domain_selected(wire::STATE_DOMAIN_PLAYER_TRADE));
    let mut rate = Rate::default();
    for _ in 0..COMMANDS_PER_SECOND {
        assert!(rate.admit(0));
    }
    assert!(!rate.admit(999));
    assert!(rate.admit(1000));
    assert!(!rate.admit(999));
    assert!(has_capacity(MAX_TRADES - 1, false));
    assert!(!has_capacity(MAX_TRADES, false));
    assert!(has_capacity(MAX_TRADES, true));
    let session =
        GameSessionId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16])?;
    let command = CommandRef::new(session, CommandId::new(u64::MAX)?);
    let tags: [&[u8]; 4] = [b"pair", b"occurrence", b"transaction", b"event"];
    let ids = tags.map(|tag| identity(command, tag));
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id[6] >> 4, 7);
        assert_eq!(id[8] & 0xc0, 0x80);
        assert_eq!(*id, identity(command, tags[i]));
        for other in ids.iter().skip(i + 1) {
            assert_ne!(id, other);
        }
    }
    eprintln!(
        "TRADE-WIRE-1 book bound: {} inline bytes for {} records (leaf JSON allocations additionally bounded by two durable leaves per record)",
        std::mem::size_of::<Record>() * MAX_TRADES,
        MAX_TRADES
    );
    Ok(())
}
