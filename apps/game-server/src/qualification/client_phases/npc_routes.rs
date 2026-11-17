// Candidate evidence comes from the compiled NPC table, before asking the walk
// planner to solve a geometry problem. An eligible trade declaration alone does
// not supply an NPC placement.
fn rune_seller_keys(id: u32) -> TestResult<std::collections::BTreeSet<String>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    Ok(trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| {
                    offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key
                })
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()
                .and_then(|key| key.strip_prefix("oteryn:service.trade."))
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect())
}

#[derive(Debug)]
struct NpcRouteInventory<'a> {
    placements: Vec<(&'a str, TilePos)>,
    in_town_window: usize,
}
fn npc_route_failure<'a>(
    from: TilePos,
    target: &str,
    npcs: &'a oteryn_world_bundle::npc::Table,
    keys: &std::collections::BTreeSet<String>,
) -> NpcRouteInventory<'a> {
    let placements: Vec<_> = npcs
        .npcs
        .iter()
        .filter(|npc| keys.contains(&npc.key))
        .flat_map(|npc| {
            npc.placements.iter().map(|p| {
                (
                    npc.key.as_str(),
                    TilePos {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                    },
                )
            })
        })
        .collect();
    let in_town_window = placements
        .iter()
        .filter(|(_, p)| {
            p.x.abs_diff(from.x) <= 160
                && p.y.abs_diff(from.y) <= 160
                && p.floor.abs_diff(from.floor) <= 2
        })
        .count();
    let inventory = NpcRouteInventory {
        placements,
        in_town_window,
    };
    let reason = if inventory.placements.is_empty() {
        "no-compiled-placement"
    } else {
        "no-walk-route"
    };
    evidence(&format!(
        "route_failed start={from:?} target={target} reason={reason} eligible_npcs={keys:?} compiled_placements={} placements_in_160_tile_window={} placements={:?}",
        inventory.placements.len(),
        inventory.in_town_window,
        inventory.placements
    ));
    inventory
}
