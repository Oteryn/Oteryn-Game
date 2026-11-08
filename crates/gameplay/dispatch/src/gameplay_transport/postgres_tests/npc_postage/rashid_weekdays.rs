//! Reuse the existing UTC selector against every frozen retail weekday anchor.
use super::*;

#[test]
fn npcfix_rashid_all_weekdays_match_frozen_anchors_and_refuse_stale_positions() -> TestResult {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/npc-authoring/samples/npcfix-retail-placements.json"));
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(path)?)?;
    let row = rows.iter().find(|r| r["npc"] == "oteryn:npc.rashid").ok_or("Rashid source")?;
    let points = row["positions"].as_array().ok_or("seven source positions")?;
    assert_eq!(points.len(), 7);
    let placements = points.iter().map(|p| Ok(NpcPlacement {
        x: u16::try_from(p["x"].as_u64().ok_or("source x")?)?,
        y: u16::try_from(p["y"].as_u64().ok_or("source y")?)?,
        floor: -i8::try_from(p["z"].as_u64().ok_or("source z")?)?,
        direction: MovementFacing::South,
    })).collect::<TestResult<Vec<_>>>()?;
    for (day, point) in placements.iter().enumerate() {
        let at = i64::try_from(day + 4)? * 86_400_000; // 1970-01-05 was Monday.
        let expected = oteryn_gameplay_entities::rashid_schedule::destination_at(at);
        assert_eq!((expected.placement.x, expected.placement.y, expected.placement.floor),
            (point.x, point.y, point.floor), "frozen weekday {day}");
        let mut table = NpcPlacementTable { npcs: vec![NpcPlacements {
            key: "oteryn:npc.rashid".into(), placements: placements.clone(),
        }] };
        oteryn_gameplay_entities::rashid_schedule::rotate_placements(&mut table, at);
        assert_eq!(table.npcs.len(), 1);
        assert_eq!(table.npcs[0].placements.len(), 1);
        assert_eq!((table.npcs[0].placements[0].x, table.npcs[0].placements[0].y),
            (point.x, point.y), "one exact placement on weekday {day}");
        let position = crate::foundation::MovementLocalPosition {
            x: i32::from(point.x), y: i32::from(point.y), floor: -i16::from(point.floor),
        };
        assert!(oteryn_gameplay_entities::rashid_schedule::trade_available(
            "oteryn:npc.rashid", position, at));
        assert!(!oteryn_gameplay_entities::rashid_schedule::trade_available(
            "oteryn:npc.rashid", position, at + 86_400_000), "stale weekday {day}");
    }
    Ok(())
}
