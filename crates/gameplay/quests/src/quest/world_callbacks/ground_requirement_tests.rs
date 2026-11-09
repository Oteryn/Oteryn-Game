use super::*;

fn document() -> serde_json::Value {
    serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-transition/ground_fixture/window", "quest":"oteryn:quest.ground_fixture",
        "trigger":"use","selector":{"position":{"x":1,"y":1,"floor":-1}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[{"track":"oteryn:quest-progress/ground_fixture/stage","minimum":0,"maximum":0,"set":1,"increment":false}],
        "offerings":[],"return_on_failure":false,"failure_message":null,"sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13"],
        "requires_world_admission":true,"world_requirements":[{"position":{"x":2,"y":1,"floor":-1},"appearance":1739,"ground_stack":[{"item":"oteryn:item.fixture_crate","count":1},
                     {"item":"oteryn:item.fixture_amphora","count":1}]}]
    }]})
}

#[test]
fn ground_stack_guards_bind_progress_without_admitting_value_free_actions() -> Result<(), String> {
    let doc = document();
    let callback = parse(&doc.to_string())?.remove(0);
    let binding =
        gated_binding(&callback)?.ok_or("ground readset must bind the progress operation")?;
    let mut changed = doc.clone();
    changed["callbacks"][0]["world_requirements"][0]["ground_stack"][0]["count"] = 2.into();
    let changed = parse(&changed.to_string())?.remove(0);
    assert_ne!(gated_binding(&changed)?, Some(binding));
    let mut changed = doc.clone();
    changed["callbacks"][0]["sources"] = serde_json::json!([
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:14"
    ]);
    changed["callbacks"][0]["success_message"] = "Different wording".into();
    assert_eq!(
        gated_binding(&parse(&changed.to_string())?.remove(0))?,
        Some(binding)
    );
    let mut value_free = doc.clone();
    value_free["callbacks"][0]["storage"] = serde_json::json!([]);
    value_free["callbacks"][0]["requires_world_admission"] = false.into();
    assert!(
        !parse(&value_free.to_string())?
            .remove(0)
            .supports_ground_stack_guard()
    );
    let mut reward = doc.clone();
    reward["callbacks"][0]["experience"] = 100.into();
    assert!(parse(&reward.to_string()).is_err());
    let mut invalid = doc;
    invalid["callbacks"][0]["world_requirements"][0]["appearance"] = 0.into();
    assert!(parse(&invalid.to_string()).is_err());
    Ok(())
}
