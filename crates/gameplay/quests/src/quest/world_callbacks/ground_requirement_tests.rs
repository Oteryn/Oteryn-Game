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
    assert!(
        parse(&invalid.to_string()).is_err(),
        "generic Use cannot omit the immutable appearance"
    );
    invalid["callbacks"][0]["trigger"] = "step_in".into();
    // Personal StepIn Ground facts may omit an immutable appearance; neither
    // kind of predicate may be omitted altogether. Preserve that distinction.
    let ground_only = parse(&invalid.to_string())?.remove(0);
    assert!(gated_binding(&ground_only)?.is_some());
    invalid["callbacks"][0]["world_requirements"][0]
        .as_object_mut()
        .ok_or("requirement")?
        .remove("ground_stack");
    assert!(parse(&invalid.to_string()).is_err());
    Ok(())
}

fn dense_document() -> serde_json::Value {
    let mut doc = document();
    doc["callbacks"][0]["world_requirements"] = serde_json::Value::Array(
        (0..36)
            .map(|cell| {
                serde_json::json!({
                    "position":{"x":32818 + cell % 6,"y":32334 + cell / 6,"floor":-9},
                    "appearance":415,
                    "ground_stack":[{"item":"oteryn:item.tibia.i2394","count":1}]
                })
            })
            .collect(),
    );
    doc
}

#[test]
fn m07_dense_ground_callback_binds_every_cell_without_expanding_effect_budgets()
-> Result<(), String> {
    let doc = dense_document();
    let callback = parse(&doc.to_string())?.remove(0);
    let binding = gated_binding(&callback)?.ok_or("dense progress must be fenced")?;
    assert_eq!(callback.world_requirements.len(), 36);
    let mut changed = doc.clone();
    changed["callbacks"][0]["world_requirements"][35]["ground_stack"][0]["item"] =
        "oteryn:item.tibia.i2395".into();
    assert_ne!(
        gated_binding(&parse(&changed.to_string())?.remove(0))?,
        Some(binding)
    );
    for field in ["experience", "requires_world_admission", "reversible"] {
        let mut unsafe_doc = doc.clone();
        unsafe_doc["callbacks"][0][field] = match field {
            "experience" => 10.into(),
            "requires_world_admission" => false.into(),
            _ => true.into(),
        };
        assert!(
            parse(&unsafe_doc.to_string()).is_err(),
            "unsafe dense field {field}"
        );
    }
    Ok(())
}

#[test]
fn m07_dense_ground_callback_refuses_sparse_multifloor_and_stacked_readsets() -> Result<(), String>
{
    parse(&dense_document().to_string())?;
    for (field, value) in [("x", 32824), ("floor", -8)] {
        let mut doc = dense_document();
        doc["callbacks"][0]["world_requirements"][35]["position"][field] = value.into();
        assert!(
            parse(&doc.to_string()).is_err(),
            "invalid dense geometry {field}"
        );
    }
    let mut doc = dense_document();
    doc["callbacks"][0]["world_requirements"][0]["ground_stack"][0]["count"] = 2.into();
    assert!(parse(&doc.to_string()).is_err());
    let mut doc = dense_document();
    doc["callbacks"][0]["world_requirements"]
        .as_array_mut()
        .ok_or("requirements")?
        .pop();
    assert!(parse(&doc.to_string()).is_err());
    Ok(())
}

fn dense_teleport_document() -> serde_json::Value {
    let mut doc = dense_document();
    doc["callbacks"][0]["trigger"] = "step_in".into();
    doc["callbacks"][0]["teleport"] = serde_json::json!({"x":32766,"y":32275,"floor":-14});
    doc
}

#[test]
fn m07_dense_ground_step_teleport_binds_destination_and_all_cells() -> Result<(), String> {
    let doc = dense_teleport_document();
    let callback = parse(&doc.to_string())?.remove(0);
    assert!(callback.supports_ground_stack_guard());
    let binding = gated_binding(&callback)?.ok_or("physical teleport must be fenced")?;
    let mut changed = doc.clone();
    changed["callbacks"][0]["teleport"]["x"] = 32767.into();
    assert_ne!(
        gated_binding(&parse(&changed.to_string())?.remove(0))?,
        Some(binding)
    );
    let mut changed = doc;
    changed["callbacks"][0]["world_requirements"][35]["ground_stack"][0]["item"] =
        "oteryn:item.tibia.i2395".into();
    assert_ne!(
        gated_binding(&parse(&changed.to_string())?.remove(0))?,
        Some(binding)
    );
    Ok(())
}

#[test]
fn m07_dense_ground_step_teleport_refuses_unbounded_and_value_compositions() -> Result<(), String> {
    parse(&dense_teleport_document().to_string())?;
    for trigger in ["use", "step_out"] {
        let mut doc = dense_teleport_document();
        doc["callbacks"][0]["trigger"] = trigger.into();
        assert!(
            parse(&doc.to_string()).is_err(),
            "teleport trigger {trigger}"
        );
    }
    for cells in [8, 35] {
        let mut doc = dense_teleport_document();
        doc["callbacks"][0]["world_requirements"]
            .as_array_mut()
            .ok_or("requirements")?
            .truncate(cells);
        assert!(parse(&doc.to_string()).is_err());
    }
    let mut doc = dense_teleport_document();
    doc["callbacks"][0]["experience"] = 100.into();
    assert!(parse(&doc.to_string()).is_err());
    let mut doc = dense_teleport_document();
    doc["callbacks"][0]["requires_world_admission"] = false.into();
    assert!(parse(&doc.to_string()).is_err());
    let mut doc = dense_teleport_document();
    doc["callbacks"][0]["teleport"]["floor"] = 127.into();
    assert!(parse(&doc.to_string()).is_err());
    Ok(())
}
