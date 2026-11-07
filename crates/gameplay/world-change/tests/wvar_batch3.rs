use oteryn_gameplay_world_change::variants;

#[test]
fn wvar_batch3_uses_admitted_camps_bounded_kill_gate_and_safe_spider_exit()
-> Result<(), Box<dyn std::error::Error>> {
    let definitions = variants::definitions()?;
    for key in [
        "jungle_camp",
        "warpath",
        "spider_nest",
        "lumberjack",
        "bored",
    ] {
        assert!(
            definitions.iter().any(|definition| definition.key == key),
            "{key}"
        );
    }
    let jungle = definitions
        .iter()
        .find(|d| d.key == "jungle_camp")
        .ok_or("jungle")?;
    assert_eq!(
        jungle.variants.iter().map(|v| v.weight).collect::<Vec<_>>(),
        [49, 1, 50]
    );
    let warpath = definitions
        .iter()
        .find(|d| d.key == "warpath")
        .ok_or("warpath")?;
    let camp = &warpath.variants[0];
    assert_eq!(
        camp.spawns
            .iter()
            .filter(|s| s.after_kills.is_none())
            .count(),
        6
    );
    assert_eq!(
        camp.spawns
            .iter()
            .find(|s| s.after_kills == Some(6))
            .map(|s| s.creature.as_str()),
        Some("oteryn:creature.bibby_bloodbath")
    );
    let spider = definitions
        .iter()
        .find(|d| d.key == "spider_nest")
        .ok_or("spider")?;
    let limit = spider.entry_limit.as_ref().ok_or("three-kill admission")?;
    assert_eq!(limit.maximum, 3);
    let entry = &spider.variants[0].portals[0];
    assert_ne!(
        entry.destination, spider.variants[0].spawns[0].position,
        "entry cannot land on the live boss"
    );
    let exit = variants::Selected::default()
        .passage((32818, 32280, 8))
        .ok_or("permanent exit")?;
    assert!(exit.exit);
    assert_ne!(
        exit.destination, entry.position,
        "closed web cannot be the return landing"
    );
    let bored = definitions
        .iter()
        .find(|d| d.key == "bored")
        .ok_or("bored")?;
    assert_eq!(bored.variants[0].spawns.len(), 3);
    assert!(
        bored.variants[0]
            .spawns
            .iter()
            .all(|s| s.creature == "oteryn:creature.giant_spider_wyda")
    );
    Ok(())
}
