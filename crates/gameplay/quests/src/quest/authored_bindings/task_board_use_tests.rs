use super::Bindings;

#[test]
fn task_board_use_acceptance_is_source_bound_and_noncompleting() -> Result<(), String> {
    let bindings = Bindings::parse(include_bytes!("fixtures/task_board_use.json"))?;
    let catalogue = bindings
        .attach(&crate::quest::QuestStateCatalogue::empty("board-fixture").ok_or("catalogue")?)?;
    let transition = catalogue
        .transition("oteryn:quest-task-board-use/m01c-fixture")
        .ok_or("transition")?;
    let changes = catalogue
        .evaluate(transition, &Default::default(), 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].after, 1);
    assert!(!catalogue.completes_after(transition, &changes));
    for (field, value) in [
        ("programme", serde_json::json!("query")),
        ("storage", serde_json::json!([])),
        ("sources", serde_json::json!([])),
    ] {
        let mut doc: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/task_board_use.json"))
                .map_err(|e| e.to_string())?;
        doc["quests"][0]["stages"][0]["trigger"]["usage"][field] = value;
        assert!(Bindings::parse(&serde_json::to_vec(&doc).map_err(|e| e.to_string())?).is_err());
    }
    Ok(())
}
