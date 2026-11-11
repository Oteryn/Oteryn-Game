use super::*;

#[test]
fn merged_registry_keeps_document_and_runtime_bounds() -> Result<(), String> {
    let callbacks = load()?;
    assert!(callbacks.len() > MAX_CALLBACKS);
    assert!(callbacks.len() <= MAX_RUNTIME_CALLBACKS);
    let smith = callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-callback/authored-smith-cache")
        })
        .collect::<Vec<_>>();
    assert_eq!(smith.len(), 2);
    let reward = smith
        .iter()
        .find(|callback| callback.key.ends_with("-reward"))
        .ok_or("native smith reward")?;
    assert_eq!(
        reward.selector.position,
        Some(Position {
            x: 32777,
            y: 32254,
            floor: -8
        })
    );
    let source = callbacks.first().ok_or("callback")?;
    let mut bounded = Vec::new();
    for index in 0..MAX_RUNTIME_CALLBACKS {
        let mut callback = source.clone();
        callback.key = format!("oteryn:quest-callback/runtime-bounded-{index}");
        callback.selector = Selector {
            position: Some(Position {
                x: index as u16,
                y: 1,
                floor: -1,
            }),
            ..callback.selector.clone()
        };
        bounded.push(callback);
    }
    validate_registry(&bounded)?;
    assert!(parse(&serde_json::json!({"schema":1,"callbacks":bounded}).to_string()).is_err());
    bounded.push(source.clone());
    assert_eq!(
        validate_registry(&bounded),
        Err("callback runtime registry bounds".into())
    );
    Ok(())
}

fn fixture() -> Result<serde_json::Value, String> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/questpc7-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut row = source["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|r| r["key"] == "oteryn:quest-callback/questpc7-grimvale-nightshade-0")
        .ok_or("source callback")?
        .clone();
    row["storage"] = serde_json::json!([{"track":"oteryn:quest-progress/fixture/bounded","minimum":0,"maximum":19,"set":20,"add":1}]);
    row["activation"] = serde_json::json!({"track":"oteryn:quest-progress/fixture/bounded","minimum":0,"maximum":19});
    row["teleport"] = serde_json::Value::Null;
    Ok(row)
}
fn document(row: serde_json::Value) -> String {
    serde_json::json!({"schema":1,"callbacks":[row]}).to_string()
}
#[test]
fn bounded_use_lowers_to_checked_add_and_refuses_after_exact_terminal() -> Result<(), String> {
    let rows = parse(&document(fixture()?))?;
    let catalogue = extend_callbacks(
        QuestStateCatalogue::empty("content-1").ok_or("catalogue")?,
        &rows,
    )?;
    let transition = catalogue.transition(&rows[0].key).ok_or("transition")?;
    assert_eq!(transition.effects[0].effect, QuestEffectKind::Add(1));
    let track = rows[0].storage[0].track.clone();
    assert_eq!(catalogue.track(&track).ok_or("track")?.max, 20);
    let mut stored = std::collections::BTreeMap::from([(track.clone(), 0)]);
    for i in 0..20 {
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!((changes[0].before, changes[0].after), (i, i + 1));
        stored.insert(track.clone(), changes[0].after);
    }
    assert_eq!(
        catalogue.evaluate(transition, &stored, 0),
        Err(crate::quest::QuestRefusal::StageMismatch)
    );
    assert_eq!(stored[&track], 20);
    Ok(())
}
#[test]
fn bounded_use_rejects_unbounded_ambiguous_or_mismatched_rules() -> Result<(), String> {
    for case in 0..15 {
        let mut row = fixture()?;
        match case {
            0 => row["storage"][0]["add"] = 0.into(),
            1 => row["storage"][0]["add"] = 2.into(),
            2 => row["storage"][0]["add"] = (-1).into(),
            3 => row["storage"][0]["minimum"] = (-1).into(),
            4 => row["storage"][0]["maximum"] = 10000.into(),
            5 => row["storage"][0]["set"] = 19.into(),
            6 => row["storage"][0]["preserve"] = true.into(),
            7 => row["storage"][0]["read_only"] = true.into(),
            8 => row["activation"] = serde_json::Value::Null,
            9 => row["activation"]["track"] = "oteryn:quest-progress/fixture/other".into(),
            10 => row["activation"]["maximum"] = 18.into(),
            11 => row["trigger"] = "step_in".into(),
            12 => row["reversible"] = true.into(),
            13 => row["storage"][0]["minimum"] = 21.into(),
            _ => {
                let extra = row["storage"][0].clone();
                row["storage"].as_array_mut().ok_or("storage")?.push(extra);
            }
        }
        assert!(parse(&document(row)).is_err(), "case {case}");
    }
    Ok(())
}
