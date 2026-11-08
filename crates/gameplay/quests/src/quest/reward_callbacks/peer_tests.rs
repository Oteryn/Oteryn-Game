use super::*;

#[test]
fn carried_peer_predicate_is_explicit_and_bounded() {
    let mut document: serde_json::Value = serde_json::from_str(DOCUMENTS[0]).unwrap();
    let row = document["callbacks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["carried_item"].is_string())
        .unwrap();
    row["peer_requires"] = serde_json::json!({
        "track": "oteryn:quest-progress/chosen/colours/colour", "minimum": 2, "maximum": 2
    });
    let before: RewardCallback = serde_json::from_value(row.clone()).unwrap();
    let binding = gated_binding(&before)
        .unwrap()
        .expect("peer guard pins reward terms");
    row["peer_requires"]["minimum"] = 1.into();
    let changed: RewardCallback = serde_json::from_value(row.clone()).unwrap();
    assert_ne!(gated_binding(&changed).unwrap(), Some(binding));
    row["peer_requires"]["minimum"] = 2.into();
    let encoded = document.to_string();
    assert!(
        parse(&encoded).is_ok(),
        "a carried callback can declare one peer predicate"
    );
    let row = document["callbacks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row.get("peer_requires").is_some())
        .unwrap();
    row["peer_requires"]["maximum"] = 1.into();
    assert!(
        parse(&document.to_string()).is_err(),
        "inverted ranges fail closed"
    );
}

#[test]
fn m43_data_attachment_waits_for_separately_packaged_opening() {
    let mut document = Document {
        schema: 1,
        tracks: Vec::new(),
        callbacks: Vec::new(),
    };
    colours_peer::append_after_opening(&mut document).unwrap();
    assert!(document.callbacks.is_empty());
    let mut data: Document = serde_json::from_str(colours_peer::DATA).unwrap();
    let mut marker = data.callbacks.pop().unwrap();
    marker.key = "oteryn:quest-reward/chosen/colours/mark-blue".into();
    marker.carried_item = Some("oteryn:item.tibia.i12208".into());
    marker.peer_requires = None;
    marker.take[0].item = "oteryn:item.tibia.i12208".into();
    document.callbacks.push(marker);
    colours_peer::append_after_opening(&mut document).unwrap();
    let paired = document
        .callbacks
        .iter()
        .find(|row| row.key == colours_peer::KEY)
        .unwrap();
    assert_eq!(
        paired.carried_item.as_deref(),
        Some("oteryn:item.tibia.i12228")
    );
    assert_eq!(paired.peer_requires.as_ref().unwrap().minimum, 2);
    assert_eq!(paired.peer_requires.as_ref().unwrap().maximum, 2);
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
    colours_peer::append_after_opening(&mut document).unwrap();
    assert_eq!(document.callbacks, before);
    validate_document(document).unwrap();
}
