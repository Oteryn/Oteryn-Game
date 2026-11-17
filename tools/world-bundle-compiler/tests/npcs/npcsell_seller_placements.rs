use std::{fs, path::Path};

use oteryn_world_bundle_compiler::source;
use serde_json::Value;

use super::*;

#[test]
fn requested_items_have_sellers_at_their_promotion_cells() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = source::project_regions(root, &[(126, 125), (131, 124)])?;
    let (resolver, palette) = source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;

    let trade: Value = serde_json::from_slice(&fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let services = trade["records"].as_array().ok_or("trade records")?;
    let sellers = [
        ("frodo", (32358, 32209, -7), &[3577][..]),
        (
            "mordecai",
            (33618, 31884, -5),
            &[3180, 3165, 3174, 3152, 3177, 3178][..],
        ),
    ];

    let mut missing = Vec::new();
    for (name, cell, items) in sellers {
        let npc_key = format!("oteryn:npc.{name}");
        let service_key = format!("oteryn:service.trade.{name}");
        assert!(
            project.families.npcs[&npc_key]
                .services
                .iter()
                .any(|service| service.key == service_key),
            "{npc_key} must reference {service_key}"
        );
        let offers = services
            .iter()
            .map(|record| &record["declaration"])
            .find(|declaration| declaration["identity"]["key"] == service_key)
            .and_then(|declaration| declaration["offers"].as_array())
            .ok_or("seller trade offers")?;
        let placed = table.npcs.iter().any(|npc| {
            npc.key == npc_key && npc.placements.iter().any(|p| (p.x, p.y, p.floor) == cell)
        });
        for item in items {
            let item_key = format!("oteryn:item.tibia.i{item}");
            assert!(
                offers.iter().any(|offer| {
                    offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key
                }),
                "{service_key} must sell {item_key}"
            );
            if !placed {
                missing.push((item_key, name, cell));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing seller placements (item, seller, cell): {missing:?}; holds: {:?}",
        report
            .held
            .iter()
            .filter(|hold| matches!(
                hold.npc.as_str(),
                "oteryn:npc.frodo" | "oteryn:npc.mordecai"
            ))
            .collect::<Vec<_>>()
    );
    Ok(())
}
