use super::*;

fn fixture() -> serde_json::Value {
    serde_json::json!({"key":"oteryn:quest-reward/tool-fixture","quest":"oteryn:quest.tool_fixture",
        "npc":"oteryn:npc.fixture","carried_item":"oteryn:item.tibia.i17669",
        "quest_tool":{"target":{"kind":"creature","creature":"oteryn:creature.adventurer","capture":true},"reusable":true,"replacement":null},
        "keywords":["tool"],"topic":0,"next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/tool/programme","minimum":0,"maximum":0,"set":0,"preserve":true}],
        "state_ops":[{"track":"oteryn:quest-progress/tool/count","from":{"kind":"between","minimum":0,"maximum":2},"effect":{"kind":"add","value":1}}],
        "take":[],"present":[{"item":"oteryn:item.tibia.i17669","count":1}],
        "give":null,"outfit":null,"achievement":null,"reply":["Captured."],"sources":["engine fixture"]})
}

#[test]
fn carried_creature_tools_require_exact_custody_and_a_bounded_advancing_counter()
-> Result<(), String> {
    let document = |c| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    let original = fixture();
    let accepted = parse(&document(original.clone()))?;
    assert_eq!(accepted.len(), 1);
    for mutation in 0..5 {
        let mut value = original.clone();
        match mutation {
            0 => value["quest_tool"]["replacement"] = "oteryn:item.tibia.i17672".into(),
            1 => value["quest_tool"]["target"]["creature"] = "oteryn:npc.adventurer".into(),
            2 => value["present"] = serde_json::json!([]),
            3 => value["state_ops"] = serde_json::json!([]),
            _ => {
                value["peer_requires"] = serde_json::json!({"track":"oteryn:quest-progress/tool/peer","minimum":0,"maximum":0})
            }
        }
        assert!(
            parse(&document(value)).is_err(),
            "unsafe tool configuration {mutation}"
        );
    }
    Ok(())
}

#[test]
fn changing_a_tool_target_changes_the_owning_counter_transition() -> Result<(), String> {
    let document = |c| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    let mut value = fixture();
    let first = parse(&document(value.clone()))?.remove(0);
    let transition = primary_transition(&first)?;
    assert!(matches!(
        transition.effects[1].effect,
        QuestEffectKind::AddWithBinding { delta: 1, .. }
    ));
    value["quest_tool"]["target"]["creature"] = "oteryn:creature.rotworm".into();
    let second = parse(&document(value))?.remove(0);
    assert_ne!(gated_binding(&first)?, gated_binding(&second)?);
    assert_ne!(
        transition, primary_transition(&second)?
    );
    Ok(())
}
