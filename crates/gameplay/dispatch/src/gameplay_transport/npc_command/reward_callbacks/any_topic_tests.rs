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
