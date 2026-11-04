//! Crystal quest data regression tests using only the public Quest APIs.
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/the_ancient_tombs/actions_dipthrah_signs_doors.lua:14-41;
//! movements_enter_mahrdis_room.lua:9-17; movements_step_morguthis_blue_flames.lua:19-25.

use std::collections::BTreeMap;

use oteryn_gameplay_quests::quest::{
    QuestRefusal, loader::load_embedded_quest_state, world_callbacks,
};

const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";

#[test]
fn diprath_refuses_out_of_order_without_changing_any_track() -> Result<(), String> {
    let loaded = load_embedded_quest_state("quests5-ancient-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    for step in 2..=8 {
        let key = format!("oteryn:quest-callback/ancient-tombs-diprath-sign-{step}");
        let transition = catalogue.transition(&key).ok_or("source sign callback")?;
        for previous in [-1, 0] {
            let state = BTreeMap::from([
                (format!("{PREFIX}diprath_sign{}", step - 1), previous),
                (format!("{PREFIX}mahrdis_treasure"), 2),
            ]);
            let before = state.clone();
            assert_eq!(
                catalogue.evaluate(transition, &state, 0),
                Err(QuestRefusal::StageMismatch),
                "sign {step} must require its immediate predecessor"
            );
            assert_eq!(state, before);
        }
    }
    Ok(())
}

#[test]
fn diprath_sequence_advances_only_the_used_sign_and_refuses_reentry() -> Result<(), String> {
    let loaded = load_embedded_quest_state("quests5-ancient-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load().map_err(|error| format!("{error:?}"))?;
    let mut state = BTreeMap::from([(format!("{PREFIX}mahrdis_treasure"), 2)]);
    for step in 1..=8_u16 {
        let key = format!("oteryn:quest-callback/ancient-tombs-diprath-sign-{step}");
        let own_track = format!("{PREFIX}diprath_sign{step}");
        let callback = callbacks
            .iter()
            .find(|value| value.key == key)
            .ok_or_else(|| format!("missing callback {key}"))?;
        assert_eq!(callback.selector.action, Some(40031 + step));
        assert_eq!(callback.selector.unique, None);
        assert_eq!(callback.trigger, world_callbacks::Trigger::Use);
        let transition = catalogue
            .transition(&key)
            .ok_or_else(|| format!("missing transition {key}"))?;
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?;
        let advances = changes
            .iter()
            .filter(|change| change.before != change.after)
            .collect::<Vec<_>>();
        assert_eq!(advances.len(), 1);
        assert_eq!(advances[0].track, own_track);
        assert_eq!(advances[0].after, 1);
        for change in changes {
            state.insert(change.track, change.after);
        }
        assert_eq!(state[&format!("{PREFIX}mahrdis_treasure")], 2);
        assert_eq!(
            catalogue.evaluate(transition, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        for future in (step + 1)..=8 {
            assert!(!state.contains_key(&format!("{PREFIX}diprath_sign{future}")));
        }
    }
    Ok(())
}

#[test]
fn mahrdis_accepts_only_one_then_persists_two_and_refuses_reentry() -> Result<(), String> {
    let loaded = load_embedded_quest_state("quests5-ancient-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let key = "oteryn:quest-callback/ancient-tombs-mahrdis-room";
    let transition = catalogue
        .transition(key)
        .ok_or_else(|| format!("missing transition {key}"))?;
    let track = format!("{PREFIX}mahrdis_treasure");
    for value in [-1, 0, 2] {
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(track.clone(), value)]), 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    let changes = catalogue
        .evaluate(transition, &BTreeMap::from([(track.clone(), 1)]), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].track, track);
    assert_eq!((changes[0].before, changes[0].after), (1, 2));
    let callbacks = world_callbacks::load().map_err(|error| format!("{error:?}"))?;
    let callback = callbacks
        .iter()
        .find(|value| value.key == key)
        .ok_or_else(|| format!("missing callback {key}"))?;
    assert_eq!(callback.selector.unique, Some(40084));
    assert!(callback.return_on_failure);
    assert_eq!(
        callback.teleport,
        Some(world_callbacks::Position {
            x: 33190,
            y: 32947,
            floor: -15
        })
    );
    Ok(())
}

#[test]
fn morguthis_flames_accept_reentry_without_changing_other_flags() -> Result<(), String> {
    let loaded = load_embedded_quest_state("quests5-ancient-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load().map_err(|error| format!("{error:?}"))?;
    for flame in 1..=7_u16 {
        let key = format!("oteryn:quest-callback/ancient-tombs-morguthis-flame-{flame}");
        let track = format!("{PREFIX}morguthis_blue_flame_storage{flame}");
        let transition = catalogue
            .transition(&key)
            .ok_or_else(|| format!("missing transition {key}"))?;
        let callback = callbacks
            .iter()
            .find(|value| value.key == key)
            .ok_or_else(|| format!("missing callback {key}"))?;
        assert_eq!(callback.selector.unique, Some(50138 + flame));
        assert_eq!(callback.selector.action, None);
        assert!(!callback.return_on_failure);
        for before in [-1, 0, 1] {
            let state = BTreeMap::from([(track.clone(), before)]);
            let changes = catalogue
                .evaluate(transition, &state, 0)
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].track, track);
            assert_eq!((changes[0].before, changes[0].after), (before, 1));
        }
    }
    Ok(())
}

#[test]
fn morguthis_gate_requires_all_seven_flags_and_preserves_completed_treasure() -> Result<(), String>
{
    // Crystal movements_step_morguthis_blue_flames.lua:27-45,55;
    // data-global/lib/core/quests.lua:2819-2828 declares the terminal value2.
    let loaded = load_embedded_quest_state("quests5-ancient-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let key = "oteryn:quest-callback/ancient-tombs-morguthis-seven-flame-gate";
    let treasure = format!("{PREFIX}morguthis_treasure");
    let transition = catalogue
        .transition(key)
        .ok_or("eight-effect source gate")?;
    assert_eq!(transition.effects.len(), 8);
    let mut all_flags = BTreeMap::new();
    for flag in 1..=7 {
        all_flags.insert(format!("{PREFIX}morguthis_blue_flame_storage{flag}"), 1);
    }
    for missing in 1..=7 {
        let mut state = all_flags.clone();
        state.remove(&format!("{PREFIX}morguthis_blue_flame_storage{missing}"));
        state.insert(treasure.clone(), 1);
        let before = state.clone();
        assert_eq!(
            catalogue.evaluate(transition, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        assert_eq!(state, before);
    }
    for old in [-1, 0, 1, 2] {
        let mut state = all_flags.clone();
        state.insert(treasure.clone(), old);
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 8);
        for change in changes {
            if change.track == treasure {
                assert_eq!((change.before, change.after), (old, 2));
            } else {
                assert_eq!((change.before, change.after), (1, 1));
            }
        }
    }
    for unsupported in [3, i64::MAX] {
        let mut state = all_flags.clone();
        state.insert(treasure.clone(), unsupported);
        assert_eq!(
            catalogue.evaluate(transition, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        assert_eq!(state[&treasure], unsupported);
    }
    let callbacks = world_callbacks::load().map_err(|error| format!("{error:?}"))?;
    let callback = callbacks
        .iter()
        .find(|value| value.key == key)
        .ok_or_else(|| format!("missing callback {key}"))?;
    assert_eq!(callback.selector.unique, Some(50146));
    assert!(callback.return_on_failure);
    assert_eq!(
        callback.teleport,
        Some(world_callbacks::Position {
            x: 33163,
            y: 32694,
            floor: -14
        })
    );
    Ok(())
}
