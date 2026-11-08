//! A duplicate introduced by one programme must never silence unrelated contacts.
use super::*;

#[test]
fn questpc5_cook_contacts_keep_shared_reward_greetings_live() -> Result<(), &'static str> {
    for key in ["oteryn:npc.cheaty", "oteryn:npc.the_draccoon_npc"] {
        assert!(reward_callback_npc(key), "{key}");
        let messages = reward_messages(key).ok_or("Cook greeting book unavailable")?;
        assert_eq!(messages.greet, "Welcome. Ask about your mission.");
    }
    let king = reward_messages("oteryn:npc.king_tibianus").ok_or("King source greeting")?;
    assert_eq!(king.greet, "Greetings, |PLAYERNAME|.");
    assert_eq!(king.farewell, "Good bye, |PLAYERNAME|.");
    Ok(())
}

#[test]
fn questpc5_shared_and_story_greeting_identities_are_globally_unique() -> Result<(), String> {
    let mut identities = std::collections::BTreeSet::new();
    for raw in REWARD_GREETINGS {
        let document: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        for row in document["npcs"].as_array().ok_or("greetings")? {
            let key = row["npc"].as_str().ok_or("NPC key")?;
            assert!(
                identities.insert(key.to_owned()),
                "duplicate greeting {key}"
            );
        }
    }
    for key in [
        "oteryn:npc.cheaty",
        "oteryn:npc.the_draccoon_npc",
        "oteryn:npc.king_tibianus",
    ] {
        assert!(identities.contains(key), "missing caller {key}");
    }
    Ok(())
}
