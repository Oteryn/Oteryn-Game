//! MP05's real authored quotes must agree with Crystal 201189f8.
//! Donor data-global/npc/sam.lua:176 (club 5/1), :238 (throwing knife 25/2).
#[test]
fn mp05_sam_quotes_match_crystal() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let document: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let sam = document["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .map(|record| &record["declaration"])
        .find(|declaration| declaration["identity"]["key"] == "oteryn:service.trade.sam")
        .ok_or("Sam trade declaration absent")?;
    let offers = sam["offers"].as_array().ok_or("Sam offers absent")?;
    for (item, direction, price) in [
        (3270, "BuyFromPlayer", 1),
        (3270, "SellToPlayer", 5),
        (3298, "BuyFromPlayer", 2),
        (3298, "SellToPlayer", 25),
    ] {
        let key = format!("oteryn:item.tibia.i{item}");
        let matching: Vec<_> = offers
            .iter()
            .filter(|offer| offer["item"]["key"] == key && offer["direction"] == direction)
            .collect();
        assert_eq!(matching.len(), 1, "Sam {key} {direction}");
        assert_eq!(matching[0]["unit_price"], price, "Sam {key} {direction}");
    }
    Ok(())
}
