use super::*;
#[test]
fn retained_tool_source_shape_rejects_consumptive_or_multiple_participants() -> Result<(), String> {
    let raw = include_str!("../../../../../../content/quests/hidden-threats-callbacks.json");
    let valid = parse(raw)?;
    assert_eq!(valid.len(), 3);
    assert!(valid.iter().all(Callback::is_progress_only_tool));
    for case in 0..4 {
        let mut document: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let row = &mut document["callbacks"][0];
        match case {
            0 => row["item_on_target"]["count"] = 2.into(),
            1 => row["item_on_target"]["replacement"] = "oteryn:item.tibia.i27268".into(),
            2 => row["item_on_target"]["target_replacement"] = 1643.into(),
            _ => {
                row["offerings"] = serde_json::json!([{"position":{"x":33024,"y":32008,"floor":-12},"item":"oteryn:item.tibia.i27269","count":1}])
            }
        }
        assert!(parse(&document.to_string()).is_err());
    }
    Ok(())
}
#[test]
fn conditional_retained_tool_success_message_requires_an_actual_flag_change() -> Result<(), String>
{
    let callback = parse(include_str!(
        "../../../../../../content/quests/hidden-threats-callbacks.json"
    ))?
    .remove(0);
    let track = callback.storage[0].track.clone();
    let unchanged = super::super::QuestTrackChange {
        track: track.clone(),
        before: 1,
        after: 1,
    };
    assert!(callback.committed_message(&[unchanged]).is_none());
    let changed = super::super::QuestTrackChange {
        track,
        before: -1,
        after: 1,
    };
    assert_eq!(
        callback.committed_message(&[changed]),
        Some("The forged key unlocks the door.")
    );
    Ok(())
}
