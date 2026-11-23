use crate::quest::barbarian_mead;

#[test]
fn barbarian_loader_binds_bare_use_on_the_awake_bear() -> Result<(), String> {
    let callbacks = super::load()?;
    let awake = callbacks
        .iter()
        .find(|callback| {
            callback.trigger == super::Trigger::Use
                && callback.selector.position
                    == Some(super::Position {
                        x: 32253,
                        y: 31049,
                        floor: -5,
                    })
                && callback.selector.appearance == Some(7174)
                && callback.item_on_target.is_none()
        })
        .ok_or("awake bear has no bare USE callback")?;
    assert!(awake.storage.is_empty());
    Ok(())
}

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
    assert_eq!(callback.effects.magic_repeat_after_ms, Some(60_000));
    let stun = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/barbarian-test/stun-bear")
        .ok_or("missing bear stun action")?;
    assert_eq!(stun.effects.reset_after_ms, Some(5_000));
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

#[test]
fn barbarian_horn_loader_binds_donor_effects_at_the_targets() -> Result<(), String> {
    let callbacks = super::load()?;
    for (key, effect) in [
        ("qm48/mammoth", 33),
        ("barbarian-test/fill-horn", 13),
        ("barbarian-test/stun-bear", 32),
        ("barbarian-test/hug-bear", 33),
    ] {
        let key = format!("oteryn:quest-callback/{key}");
        let callback = callbacks.iter().find(|row| row.key == key).ok_or(&key)?;
        let magic = callback.effects.magic.as_ref().ok_or(&key)?;
        assert_eq!(Some(magic.position), callback.selector.position, "{key}");
        assert_eq!(magic.effect, effect, "{key}");
    }
    Ok(())
}

#[test]
fn sven_links_debits_bear_and_mammoth_to_barbarian_completion() -> Result<(), String> {
    use crate::quest::{loader, reward_callbacks};
    let loaded =
        loader::load_embedded_quest_state("barbarian-test").map_err(|err| format!("{err:?}"))?;
    let rewards = reward_callbacks::offered_for(loaded.catalogue())?;
    let sven = |suffix: &str| {
        rewards
            .iter()
            .find(|r| r.npc == "oteryn:npc.sven" && r.key.ends_with(suffix))
            .ok_or_else(|| format!("missing Sven {suffix} callback"))
    };
    assert_eq!(
        sven("/sven-honeycomb")?.take[0].item,
        "oteryn:item.tibia.i5902"
    );
    assert_eq!(
        sven("/sven-mead-report")?
            .give
            .as_ref()
            .map(|gift| gift.item.as_str()),
        Some("oteryn:item.tibia.i7140")
    );
    assert!(sven("/sven-bear-report")?.storage.iter().any(|s| s.track
        == barbarian_mead::TRACKS[0]
        && (s.minimum, s.maximum, s.set) == (5, 5, 6)));
    let done = sven("/sven-mammoth-report")?;
    assert!(done.completes);
    assert_eq!(
        done.achievement.as_deref(),
        Some("oteryn:achievement/honorary_barbarian")
    );
    assert!(
        done.participants
            .iter()
            .flat_map(|p| &p.storage)
            .any(|s| s.track == barbarian_mead::TRACKS[0]
                && (s.minimum, s.maximum, s.set) == (7, 7, 8))
    );
    for suffix in ["fill-horn", "stun-bear", "hug-bear"] {
        let key = format!("oteryn:quest-callback/barbarian-test/{suffix}");
        assert!(
            super::load_for(loaded.catalogue())?
                .iter()
                .any(|c| c.key == key),
            "{key}"
        );
    }
    Ok(())
}
