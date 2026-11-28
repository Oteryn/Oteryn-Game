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
fn quest_transcripts_match_the_published_offer_and_settlement_topics() -> Result<(), String> {
    let offers = offers::embedded().ok_or("quest offers")?;
    let callbacks = reward_callbacks::load()?;
    for (npc, word, next_topic) in [
        ("gnomilly", "report", 1),
        ("gnombold", "report", 1),
        ("a_sweaty_cyclops", "bast skirt", 4),
        ("cael", "tomes of knowledge", 1),
        ("sven", "barbarians", 1),
    ] {
        let name = format!("oteryn:npc.{npc}");
        assert!(
            offers.offers.iter().any(|offer| offer.npc == name
                && offer.matches_topic(0)
                && offer.matches_keyword(word)
                && offer.next_topic == next_topic),
            "{npc} {word}"
        );
    }
    for (npc, topic, word) in [
        ("gnomilly", 1, "kill"),
        ("gnombold", 1, "kill"),
        ("dedoras", 3, "epaulette"),
        ("vanys", 8, "pomegranate"),
        ("undal", 8, "shield"),
    ] {
        let name = format!("oteryn:npc.{npc}");
        assert!(
            callbacks.iter().any(|row| row.npc == name
                && row.topic == topic
                && matches_reward_keyword(row, word)),
            "{npc} {word}"
        );
    }
    Ok(())
}
