//! RCAUTH: Willie, Lee'Delle and Wyda trade one carried item for one source reward.
use super::*;

fn exchange<'a>(callbacks: &'a [RewardCallback], key: &str) -> Result<&'a RewardCallback, String> {
    callbacks
        .iter()
        .find(|c| c.key == key)
        .ok_or_else(|| format!("missing {key}"))
}

type Terms<'a> = (Vec<(&'a str, u16)>, Option<(&'a str, u16)>);

fn terms(callback: &RewardCallback) -> Terms<'_> {
    (
        callback
            .take
            .iter()
            .map(|input| (input.item.as_str(), input.count))
            .collect(),
        callback
            .give
            .as_ref()
            .map(|give| (give.item.as_str(), give.count)),
    )
}

#[test]
fn source_item_exchanges_are_atomic_and_refuse_with_the_donor_line() -> Result<(), String> {
    let callbacks = load()?;
    for (key, npc, keyword, take, give, failure, repeatable) in [
        (
            "oteryn:quest-reward/rcauth/willie-banana",
            "oteryn:npc.willie",
            "banana",
            "oteryn:item.tibia.i3587",
            "oteryn:item.tibia.i3426",
            "Are you trying to mess with me?!",
            true,
        ),
        (
            "oteryn:quest-reward/rcauth/lee-delle-honey-flower",
            "oteryn:npc.lee_delle",
            "honey flower",
            "oteryn:item.tibia.i2984",
            "oteryn:item.tibia.i3362",
            "Honey flowers are my favourites <sighs>.",
            true,
        ),
        (
            "oteryn:quest-reward/rcauth/wyda-blood-herb",
            "oteryn:npc.wyda",
            "blood herb",
            "oteryn:item.tibia.i3734",
            "oteryn:item.tibia.i3454",
            "No, you don't have any...",
            false,
        ),
    ] {
        let callback = exchange(&callbacks, key)?;
        assert_eq!(callback.npc, npc);
        assert!(callback.keywords.iter().any(|word| word == keyword));
        assert_eq!(terms(callback), (vec![(take, 1)], Some((give, 1))));
        assert_eq!(callback.failure_reply, [failure]);
        assert_eq!(callback.repeatable, repeatable, "{key}");
        assert_eq!(callback.is_paid_exchange(), repeatable, "{key}");
    }
    Ok(())
}

#[test]
fn wyda_takes_one_blood_herb_once_after_the_herb_chest() -> Result<(), String> {
    let callbacks = load()?;
    let wyda = exchange(&callbacks, "oteryn:quest-reward/rcauth/wyda-blood-herb")?;
    assert_eq!(
        wyda.requires
            .iter()
            .map(|r| (r.track.as_str(), r.minimum, r.maximum))
            .collect::<Vec<_>>(),
        [("oteryn:quest-progress/kv/bloodherb/claimed", 1, 1)]
    );
    assert_eq!(
        wyda.storage
            .iter()
            .map(|s| (s.track.as_str(), s.minimum, s.maximum, s.set, s.preserve))
            .collect::<Vec<_>>(),
        [(
            "oteryn:quest-progress/rcauth/wyda/blood_herb",
            0,
            0,
            1,
            false
        )]
    );
    let loaded = super::super::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let track = loaded
        .catalogue()
        .track("oteryn:quest-progress/rcauth/wyda/blood_herb")
        .ok_or("declared Wyda track")?;
    assert_eq!(track.quest, wyda.quest);
    assert_eq!((track.initial, track.min, track.max), (0, 0, 1));
    for key in [
        "oteryn:quest-reward/rcauth/lee-delle-honey-flower",
        "oteryn:quest-reward/rcauth/willie-banana",
        "oteryn:quest-reward/rcauth/wyda-blood-herb",
    ] {
        assert!(loaded.catalogue().transition(key).is_some(), "{key}");
    }
    Ok(())
}
