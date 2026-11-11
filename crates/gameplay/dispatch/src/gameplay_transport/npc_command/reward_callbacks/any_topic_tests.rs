//! M39: source NPC offers may resume a conversation from any current topic.
use super::*;

#[test]
fn m39_any_topic_offer_is_accepted_by_the_production_parser()
-> Result<(), Box<dyn std::error::Error>> {
    let offer: Offer = serde_json::from_value(serde_json::json!({
        "npc": "oteryn:npc.cael",
        "keywords": ["tome"],
        "topic": 0,
        "next_topic": 1,
        "storage": [],
        "reply": ["May I have that tome?"],
        "any_topic": true
    }))?;
    assert_eq!(offer.next_topic, 1);
    for topic in [0, 1, 99, u16::MAX] {
        assert!(offer.matches_topic(topic));
    }
    Ok(())
}

#[test]
fn m39_legacy_offers_keep_exact_topic_matching() -> Result<(), Box<dyn std::error::Error>> {
    for any_topic in [None, Some(false)] {
        let mut value = serde_json::json!({
            "npc": "oteryn:npc.cael", "keywords": ["tome"],
            "topic": 7, "next_topic": 8, "storage": [], "reply": ["A tome?"]
        });
        if let Some(enabled) = any_topic {
            value["any_topic"] = enabled.into();
        }
        let offer: Offer = serde_json::from_value(value)?;
        assert!(offer.matches_topic(7));
        for topic in [0, 6, 8, u16::MAX] {
            assert!(!offer.matches_topic(topic));
        }
    }
    Ok(())
}

#[test]
fn rookie_embedded_offer_connects_only_eligible_first_yes_to_the_reward_topic()
-> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;

    let catalogue = oteryn_gameplay_quests::quest::QuestStateCatalogue::empty("rookie-offer")
        .ok_or("empty offer catalogue")?;
    let reward = reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/m09-rookie-mission11")
        .ok_or("source Rookie reward")?;
    let document = offers::embedded().ok_or("production offer catalogue")?;
    let mut candidates = document
        .offers
        .iter()
        .filter(|offer| offer.npc == reward.npc && offer.keywords.iter().any(|word| word == "yes"));
    let offer = candidates.next().ok_or("source Rookie first-yes offer")?;
    assert!(
        candidates.next().is_none(),
        "the source prompt must be unambiguous"
    );
    assert_eq!(offer.topic, 0);
    assert_eq!(offer.next_topic, reward.topic);
    assert_eq!(reward.topic, 91);
    assert_eq!(reward.keywords, ["yes"]);

    let mission10 = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission10";
    let mission11 = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission11";
    let ready = BTreeMap::from([(mission10.to_owned(), 3), (mission11.to_owned(), -1)]);
    assert!(offer.admits(&reward.npc, 0, "yes", &catalogue, &ready));
    for topic in [1, reward.topic, u16::MAX] {
        assert!(!offer.admits(&reward.npc, topic, "yes", &catalogue, &ready));
    }
    assert!(!offer.admits(&reward.npc, 0, "no", &catalogue, &ready));
    assert!(!offer.admits("oteryn:npc.another", 0, "yes", &catalogue, &ready));
    assert!(!offer.admits(&reward.npc, 0, "yes", &catalogue, &BTreeMap::new()));
    for (track, value) in [
        (mission10, 2),
        (mission10, 4),
        (mission11, 0),
        (mission11, 1),
    ] {
        let mut ineligible = ready.clone();
        ineligible.insert(track.to_owned(), value);
        assert!(!offer.admits(&reward.npc, 0, "yes", &catalogue, &ineligible));
    }
    Ok(())
}
