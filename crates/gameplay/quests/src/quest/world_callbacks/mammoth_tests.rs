use crate::quest::barbarian_mead;

#[test]
fn qm48_mammoth_binds_live_drunk_action_and_source_storage() -> Result<(), String> {
    let callbacks = super::load()?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/qm48/mammoth")
        .ok_or("missing live drunk mammoth action")?;
    assert!(callback.requires_live_drunk());
    assert_eq!(callback.quest, barbarian_mead::QUEST);
    assert_eq!(callback.selector.appearance, Some(7176));
    assert_eq!(callback.effects.transform, Some(7177));
    assert_eq!(callback.effects.reset_after_ms, Some(5_000));
    assert_eq!(callback.storage[0].track, barbarian_mead::TRACKS[0]);
    assert_eq!(
        (
            callback.storage[0].minimum,
            callback.storage[0].maximum,
            callback.storage[0].set
        ),
        (6, 6, 7)
    );
    assert_eq!(
        callback.storage[1].track,
        "oteryn:quest-progress/quest/u8_0/barbarian_test/mission03"
    );
    assert_eq!(callback.storage[1].set, 2);
    Ok(())
}
