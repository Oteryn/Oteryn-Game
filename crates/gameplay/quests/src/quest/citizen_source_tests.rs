//! Mainland Citizen source costs and timing must never alias Rookgaard progress.
use super::reward_callbacks;

#[test]
fn citizen_regular_source_costs_clock_and_addons_are_distinct_from_rook() -> Result<(), String> {
    let callbacks = reward_callbacks::load()?;
    let find = |suffix: &str| {
        callbacks
            .iter()
            .find(|c| c.key == format!("oteryn:quest-reward/questpc-citizen-{suffix}"))
            .ok_or_else(|| format!("missing mainland Citizen source callback: {suffix}"))
    };
    let materials = find("backpack-materials")?;
    assert_eq!(materials.npc, "oteryn:npc.lubo");
    assert_eq!(materials.take.len(), 1);
    assert_eq!(materials.take[0].item, "oteryn:item.tibia.i5878");
    assert_eq!(materials.take[0].count, 100);
    let backpack = find("backpack")?;
    assert_eq!(backpack.outfit.as_ref().ok_or("backpack addon")?.addons, 1);
    let clock = serde_json::to_value(&backpack.state_ops).map_err(|e| e.to_string())?;
    assert!(
        clock
            .as_array()
            .ok_or("clock operations")?
            .iter()
            .any(|op| {
                op["from"]["kind"] == "elapsed_at_least" && op["from"]["seconds"] == 7201
            })
    );
    let hat = find("hat")?;
    assert_eq!(hat.npc, "oteryn:npc.hanna");
    assert_eq!(
        hat.failure_next_topic,
        Some(0),
        "source refusal ends the material question"
    );
    assert_eq!(hat.outfit.as_ref().ok_or("hat addon")?.addons, 2);
    let amounts: std::collections::BTreeMap<_, _> = hat
        .take
        .iter()
        .map(|input| (input.item.as_str(), input.count))
        .collect();
    assert_eq!(
        amounts,
        std::collections::BTreeMap::from([
            ("oteryn:item.tibia.i3374", 1),
            ("oteryn:item.tibia.i5890", 100),
            ("oteryn:item.tibia.i5902", 50),
        ])
    );
    assert!(
        callbacks
            .iter()
            .filter(|c| c.key.starts_with("oteryn:quest-reward/questpc-citizen-"))
            .flat_map(|c| &c.storage)
            .all(|s| !s.track.contains("citizen_outfits_rook"))
    );
    Ok(())
}
