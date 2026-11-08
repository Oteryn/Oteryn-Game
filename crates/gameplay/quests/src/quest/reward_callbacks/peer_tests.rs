use super::*;

fn fail<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

#[test]
fn carried_peer_predicate_is_explicit_and_bounded() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/l10-brotherhood-use-follower")
        .ok_or("carried source callback")?;
    let mut document = serde_json::json!({"schema": 1, "callbacks": [callback]});
    let row = document["callbacks"]
        .as_array_mut()
        .ok_or("callbacks array")?
        .iter_mut()
        .find(|row| row["carried_item"].is_string())
        .ok_or("carried callback")?;
    row["peer_requires"] = serde_json::json!({
        "track": "oteryn:quest-progress/chosen/colours/colour", "minimum": 2, "maximum": 2
    });
    let before: RewardCallback = serde_json::from_value(row.clone()).map_err(fail)?;
    let binding = gated_binding(&before)
        .map_err(fail)?
        .ok_or("peer guard pins reward terms")?;
    row["peer_requires"]["minimum"] = 1.into();
    let changed: RewardCallback = serde_json::from_value(row.clone()).map_err(fail)?;
    assert_ne!(gated_binding(&changed).map_err(fail)?, Some(binding));
    row["peer_requires"]["minimum"] = 2.into();
    let encoded = document.to_string();
    assert!(
        parse(&encoded).is_ok(),
        "a carried callback can declare one peer predicate"
    );
    let row = document["callbacks"]
        .as_array_mut()
        .ok_or("callbacks array")?
        .iter_mut()
        .find(|row| row.get("peer_requires").is_some())
        .ok_or("peer callback")?;
    row["peer_requires"]["maximum"] = 1.into();
    assert!(
        parse(&document.to_string()).is_err(),
        "inverted ranges fail closed"
    );
    Ok(())
}

#[test]
fn m43_data_attachment_waits_for_separately_packaged_opening() -> Result<(), String> {
    let mut document = Document {
        schema: 1,
        tracks: Vec::new(),
        callbacks: Vec::new(),
    };
    colours_peer::append_after_opening(&mut document).map_err(fail)?;
    assert!(document.callbacks.is_empty());
    let mut data: Document = serde_json::from_str(colours_peer::DATA).map_err(fail)?;
    let mut marker = data.callbacks.pop().ok_or("M43 data callback")?;
    marker.key = "oteryn:quest-reward/chosen/colours/mark-blue".into();
    marker.carried_item = Some("oteryn:item.tibia.i12208".into());
    marker.peer_requires = None;
    marker.take.first_mut().ok_or("take row")?.item = "oteryn:item.tibia.i12208".into();
    document.callbacks.push(marker);
    colours_peer::append_after_opening(&mut document).map_err(fail)?;
    let paired = document
        .callbacks
        .iter()
        .find(|row| row.key == colours_peer::KEY)
        .ok_or("paired M43 callback")?;
    assert_eq!(
        paired.carried_item.as_deref(),
        Some("oteryn:item.tibia.i12228")
    );
    let peer = paired.peer_requires.as_ref().ok_or("peer requirement")?;
    assert_eq!(peer.minimum, 2);
    assert_eq!(peer.maximum, 2);
    assert!(
        paired
            .storage
            .iter()
            .any(|row| row.track.ends_with("/colour")
                && row.minimum == 1
                && row.maximum == 1
                && row.set == 3)
    );
    let before = document.callbacks.clone();
    colours_peer::append_after_opening(&mut document).map_err(fail)?;
    assert_eq!(document.callbacks, before);
    validate_document(document).map_err(fail)?;
    Ok(())
}

#[test]
fn carried_creature_requires_exact_native_identity() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/l10-brotherhood-use-follower")
        .ok_or("carried source callback")?;
    let mut document = serde_json::json!({"schema": 1, "callbacks": [callback]});
    let row = document["callbacks"]
        .as_array_mut()
        .ok_or("callbacks")?
        .iter_mut()
        .find(|row| row["carried_item"].is_string())
        .ok_or("carried callback")?;
    row["creature_target"] = "oteryn:creature.rotworm".into();
    assert!(parse(&document.to_string()).is_ok());
    let row = document["callbacks"]
        .as_array_mut()
        .ok_or("callbacks")?
        .iter_mut()
        .find(|row| row["creature_target"].is_string())
        .ok_or("creature callback")?;
    row["creature_target"] = "rotworm".into();
    assert!(parse(&document.to_string()).is_err());
    Ok(())
}

#[test]
fn goblin_rotworm_sample_binds_progress_tool_and_output() -> Result<(), String> {
    let callbacks = load()?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/rotworm-sample")
        .ok_or("Rotworm sample callback")?;
    assert_eq!(
        callback.creature_target.as_deref(),
        Some("oteryn:creature.rotworm")
    );
    assert_eq!(
        callback.carried_item.as_deref(),
        Some("oteryn:item.tibia.i17673")
    );
    assert_eq!(
        callback.give.as_ref().ok_or("transformed tool")?.item,
        "oteryn:item.tibia.i17674"
    );
    assert_eq!(callback.storage[0].minimum, 0);
    assert_eq!(callback.storage[0].maximum, 0);
    assert_eq!(callback.storage[0].set, 1);
    assert!(gated_binding(callback)?.is_some());
    Ok(())
}
