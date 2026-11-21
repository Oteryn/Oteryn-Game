//! M39's donor collection removes the clicked bones after filling the vessel.
use super::*;

#[test]
fn m39_carried_sample_can_request_exact_clicked_target_removal()
-> Result<(), Box<dyn std::error::Error>> {
    let callback = load()?.into_iter().next().ok_or("world callback")?;
    let mut source = serde_json::to_value(callback)?;
    source["item_on_target"] = serde_json::json!({
        "item":"oteryn:item.tibia.i4852", "count":1,
        "replacement":"oteryn:item.tibia.i131", "target_replacement":null,
        "target_remove":true
    });
    let parsed: Callback = serde_json::from_value(source)?;
    assert_eq!(
        serde_json::to_value(parsed)?["item_on_target"]["target_remove"],
        true
    );
    Ok(())
}
