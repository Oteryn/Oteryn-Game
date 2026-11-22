use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, reward_callbacks,
};
use sha2::{Digest, Sha256};

#[test]
fn zebron_dice_bindings_allow_paid_loss_at_zero_streak() -> Result<(), String> {
    let mut loss = serde_json::json!({
        "key": "oteryn:quest-reward/kissing-a-pig/zebron-loss",
        "quest": "oteryn:quest.authored.kissing_a_pig_quest",
        "npc": "oteryn:npc.zebron",
        "keywords": ["yes"],
        "topic": 6301,
        "next_topic": 6301,
        "storage": [{
            "track": "oteryn:quest-progress/kissing-a-pig/zebron-wins",
            "minimum": 0,
            "maximum": 2,
            "set": 0
        }],
        "take": [],
        "fee": {"gold": 25, "include_bank": false},
        "dice_outcome": "loss",
        "reply": ["You lose with |PLAYER_ROLL| against |NPC_ROLL|."],
        "sources": ["https://tibia.fandom.com/wiki/Kissing_a_Pig_Quest/Spoiler"]
    });
    let validate = |loss: &serde_json::Value| {
        let source = loss["sources"][0].as_str().ok_or("source")?;
        let recipe_sha256: String = Sha256::digest(source)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let packet = serde_json::json!({"schema": 1, "quests": [{
            "quest": loss["quest"],
            "recipe_sha256": recipe_sha256,
            "sources": [source],
            "tracks": [{
                "key": loss["storage"][0]["track"],
                "minimum": 0, "maximum": 3, "initial": 0
            }],
            "stages": [{
                "key": "zebron-loss",
                "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": loss}
            }]
        }]});
        let bindings = Bindings::parse(packet.to_string().as_bytes())?;
        let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
        reward_callbacks::load_for(&bindings.attach(&base)?)
    };
    assert!(
        validate(&loss)?.iter().any(|callback| {
            callback.key == "oteryn:quest-reward/kissing-a-pig/zebron-loss"
        })
    );
    loss["fee"] = serde_json::Value::Null;
    assert!(validate(&loss).is_err());
    loss["fee"] = serde_json::json!({"gold": 25, "include_bank": false});
    loss["dice_outcome"] = serde_json::Value::Null;
    assert!(validate(&loss).is_err());
    Ok(())
}
