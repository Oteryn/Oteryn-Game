use super::*;
#[test]
fn source_portals_require_read_only_retained_tool_admission() -> Result<(), String> {
    validate_callbacks(&[crate::quest::source_world_objects::dreamer_entrance()])?;
    let mut callback = crate::quest::source_world_objects::uneasy_crystal();
    callback.key = "oteryn:quest-callback/m44-dreamer-entrance".into();
    callback.quest = "oteryn:quest.dreamers_challenge_quest".into();
    callback.sources=vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_fire_bug.lua:110-128".into()];
    callback.storage.clear();
    callback.requires_world_admission = true;
    callback.selector.native = Some(NativeSelector {
        position: Position {
            x: 32849,
            y: 32233,
            floor: -9,
        },
        ordinal: 1,
        appearance: 3134,
    });
    callback.selector.action = None;
    callback.selector.unique = Some(2243);
    callback.selector.appearance = Some(3134);
    callback.item_on_target = Some(ItemOnTarget {
        item: "oteryn:item.tibia.i5467".into(),
        alternatives: Vec::new(),
        count: 1,
        replacement: None,
        target_replacement: None,
        reusable: true,
        selected_instance: false,
        target_remove: false,
        ingredients: Vec::new(),
    });
    let mut value = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    value["effects"] = serde_json::json!({"source_portal":{
        "position":{"x":32849,"y":32233,"floor":-9},
        "destination":{"x":32857,"y":32234,"floor":-11},
        "appearance":1949,"lifetime_us":300000000,"restore_source":true,
        "group":"oteryn:quest-source/dreamer-entrance"
    }});
    let document =
        |v: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[v]}).to_string();
    parse(&document(&value))?;
    for mutation in 0..5 {
        let mut invalid = value.clone();
        match mutation {
            0 => invalid["item_on_target"]["reusable"] = false.into(),
            1 => invalid["effects"]["source_portal"]["lifetime_us"] = 0.into(),
            2 => invalid["effects"]["source_portal"]["position"]["x"] = 32850.into(),
            3 => {
                invalid["effects"]["item_reward"] = serde_json::json!({"premium":false,"item":"oteryn:item.tibia.i3031","count":1,"contents":[],"additional":[],"text":null})
            }
            _ => invalid["requires_world_admission"] = false.into(),
        }
        assert!(parse(&document(&invalid)).is_err());
    }
    Ok(())
}
