use super::*;

fn document() -> serde_json::Value {
    serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/m01c-paid-step-fixture",
        "quest":"oteryn:quest.m01c_paid_step_fixture","trigger":"step_in",
        "admission":{"premium":true},"requires_world_admission":true,
        "selector":{"position":null,"unique":2241,"action":null,"appearance":null,
                    "native":{"position":{"x":32861,"y":32239,"floor":-9},"ordinal":0,"appearance":353}},
        "lever":null,"reversible":false,"passages":[],
        "teleport":{"x":32861,"y":32235,"floor":-9},
        "storage":[{"track":"oteryn:quest-progress/m01c-paid-step-fixture","minimum":0,"maximum":1,"set":1}],
        "offerings":[],"item_on_target":{"item":"oteryn:item.tibia.i3595","count":1,
            "replacement":null,"target_replacement":null,"on_step":true},
        "return_on_failure":true,"failure_message":null,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_carrot.lua:9-13"]
    }]})
}

#[test]
fn automatic_step_input_requires_explicit_fenced_native_admission() -> Result<(), String> {
    let document = document();
    let callbacks = parse(&document.to_string())?;
    assert!(callbacks[0].item_on_target.as_ref().ok_or("input")?.on_step);
    for mutation in 0..6 {
        let mut invalid = document.clone();
        let callback = &mut invalid["callbacks"][0];
        match mutation {
            0 => callback["item_on_target"]["on_step"] = false.into(),
            1 => callback["trigger"] = "use".into(),
            2 => callback["requires_world_admission"] = false.into(),
            3 => callback["selector"]["native"] = serde_json::Value::Null,
            4 => callback["item_on_target"]["target_replacement"] = 1949.into(),
            _ => callback["item_on_target"]["reusable"] = true.into(),
        }
        assert!(
            parse(&invalid.to_string()).is_err(),
            "accepted mutation {mutation}"
        );
    }
    Ok(())
}
