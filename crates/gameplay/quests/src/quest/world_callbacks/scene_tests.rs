use super::*;

#[test]
fn source_scenes_refuse_ambiguous_unbounded_and_value_bearing_edits() -> Result<(), String> {
    let source = include_str!("../../../../../../content/quests/native-scene-callbacks.json");
    let valid: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let callbacks = parse(source)?;
    let scene = callbacks[0].effects.scene.as_ref().ok_or("scene")?;
    assert_eq!(
        scene.open.iter().filter(|e| e.remove == Some(1271)).count(),
        9
    );
    assert_eq!(
        scene.open.iter().filter(|e| e.create == Some(1270)).count(),
        2
    );
    for case in 0..8 {
        let mut bad = valid.clone();
        match case {
            0 => {
                bad["callbacks"][0]["effects"]["scene"]["open"][1]["position"] =
                    bad["callbacks"][0]["effects"]["scene"]["open"][0]["position"].clone()
            }
            1 => {
                bad["callbacks"][0]["effects"]["item_reward"] =
                    serde_json::json!({"item":"oteryn:item.tibia.i3031","count":1,"contents":[]})
            }
            2 => {
                bad["callbacks"][0]["effects"]["scene"]["open"] = serde_json::json!([]);
                bad["callbacks"][0]["effects"]["scene"]["remove_self"] = false.into();
            }
            3 => bad["callbacks"][0]["reversible"] = true.into(),
            4 => {
                let prototype = bad["callbacks"][0]["effects"]["scene"]["open"][0].clone();
                bad["callbacks"][0]["effects"]["scene"]["open"] = (0..33)
                    .map(|x| {
                        let mut edit = prototype.clone();
                        edit["position"]["x"] = (32000 + x).into();
                        edit
                    })
                    .collect::<Vec<_>>()
                    .into();
            }
            5 => {
                bad["callbacks"][0]["effects"]["scene"]["open"][0]["passable"] =
                    serde_json::Value::Null
            }
            6 => {
                bad["callbacks"][0]["effects"]["scene"]["open"][0]["position"] =
                    bad["callbacks"][0]["selector"]["position"].clone()
            }
            _ => bad["callbacks"][0]["effects"]["scene"]["unexpected"] = true.into(),
        }
        assert!(
            parse(&bad.to_string()).is_err(),
            "invalid source scene {case}"
        );
    }
    // The new owner does not expand the older passage primitive's eight-effect bound.
    let mut legacy = valid;
    legacy["callbacks"][0]["effects"] = serde_json::json!({});
    legacy["callbacks"][0]["passages"] = (32593..=32601)
        .map(|x| {
            serde_json::json!({
                "position":{"x":x,"y":32104,"floor":-14},"appearance":1271,"replacement":null
            })
        })
        .collect::<Vec<_>>()
        .into();
    assert!(parse(&legacy.to_string()).is_err());
    Ok(())
}
