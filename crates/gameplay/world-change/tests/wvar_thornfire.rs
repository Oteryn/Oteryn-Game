#[test]
fn thornfire_recipe_preserves_all_homes_and_exact_station_operands()
-> Result<(), Box<dyn std::error::Error>> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../content/encounters/world-changes/variants.json"
    ))?;
    let d = source
        .as_array()
        .ok_or("definitions")?
        .iter()
        .find(|d| d["key"] == "thornfire")
        .ok_or("Thornfire definition absent")?;
    assert_eq!(d["slot"], 15);
    assert_eq!(
        d["reset_counter"]["stages"],
        serde_json::json!([[1, 1], [2, 0]])
    );
    let variants = d["variants"].as_array().ok_or("variants")?;
    assert_eq!(
        variants[0]["spawns"]
            .as_array()
            .ok_or("burning spawns")?
            .len(),
        24
    );
    let normal = variants[1]["spawns"].as_array().ok_or("normal spawns")?;
    assert_eq!(normal.len(), 54);
    let homes: std::collections::BTreeSet<_> =
        normal.iter().map(|s| s["position"].to_string()).collect();
    assert_eq!(homes.len(), 54);
    assert_eq!(
        normal
            .iter()
            .filter(|s| s["creature"] == "oteryn:creature.elf_overseer")
            .count(),
        3
    );
    let stations = d["stations"].as_array().ok_or("stations")?;
    assert_eq!(stations.len(), 666);
    let mut fires = 0;
    let mut bogs = 0;
    for station in stations {
        if station["resource"] == "thornfire_extinguished" {
            fires += 1;
            assert_eq!(station["material"], "oteryn:item.tibia.i12722");
            assert_eq!(station["yield_item"], 12819);
            assert_eq!(
                station["restore_after_save_ms"],
                serde_json::json!([28_800_000, 57_600_000])
            );
        } else {
            bogs += 1;
            assert!(station["resource"].is_null());
            assert_eq!(station["material"], "oteryn:item.tibia.i12819");
            assert_eq!(station["yield_item"], 12722);
            assert_eq!(station["after"], 12721);
        }
    }
    assert_eq!((fires, bogs), (610, 56));
    Ok(())
}
