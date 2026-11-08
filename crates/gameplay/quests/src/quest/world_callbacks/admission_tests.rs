use super::*;

#[test]
fn world_admission_predicates_require_personal_fenced_player_callbacks() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| !c.storage.is_empty() && !c.reversible)
        .ok_or("personal callback")?;
    let mut value = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
    value["admission"] =
        serde_json::json!({"regular_blessings":true,"promotion":true,"premium":true});
    let document =
        |value: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[value]}).to_string();
    assert!(
        parse(&document(&value)).is_ok(),
        "live admission predicates must be authorable"
    );
    value["storage"] = serde_json::json!([]);
    value["state_ops"] = serde_json::json!([]);
    assert!(
        parse(&document(&value)).is_err(),
        "admission cannot bypass the personal writer"
    );
    Ok(())
}

#[test]
fn qm03_entrances_bind_live_guards_to_native_objects_and_personal_progress() -> Result<(), String> {
    let callbacks = load()?;
    for (name, quest, trigger, level, promotion, blessings) in [
        (
            "anniversary",
            "25_years_of_tibia_quest",
            Trigger::Use,
            20,
            true,
            true,
        ),
        (
            "pits",
            "the_pits_of_inferno_quest",
            Trigger::StepIn,
            80,
            false,
            false,
        ),
    ] {
        let callback = callbacks
            .iter()
            .find(|c| c.key == format!("oteryn:quest-callback/qm03/{name}"))
            .ok_or_else(|| format!("missing {name} entrance"))?;
        assert_eq!(callback.quest, format!("oteryn:quest.{quest}"));
        assert_eq!(callback.trigger, trigger);
        assert_eq!(callback.minimum_level, Some(level));
        assert!(callback.requires_world_admission);
        assert!(callback.selector.native.is_some());
        assert_eq!(
            callback.admission,
            Some(Admission {
                premium: true,
                promotion,
                regular_blessings: blessings
            })
        );
        assert_eq!(
            callback.storage[0].track,
            format!("oteryn:quest-progress/chosen-source/{quest}/s1")
        );
        assert!(callback.storage[0].at_least);
        let binding = gated_binding(callback)?.ok_or("physical binding")?;
        let mut changed = callback.clone();
        changed.admission = None;
        assert_ne!(
            gated_binding(&changed)?,
            Some(binding),
            "live guards are part of the physical admission binding"
        );
    }
    Ok(())
}
