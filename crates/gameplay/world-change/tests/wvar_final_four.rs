use oteryn_gameplay_world_change::variants;

#[test]
fn final_four_variants_preserve_complete_membership_and_safe_rotation()
-> Result<(), Box<dyn std::error::Error>> {
    let all = variants::definitions()?;
    for (key, count) in [
        ("chakoya_iceberg", 2),
        ("spirit_grounds", 12),
        ("hive_outpost", 2),
        ("poacher_caves", 3),
    ] {
        let d = all
            .iter()
            .find(|d| d.key == key)
            .ok_or("final-four definition absent")?;
        assert_eq!(d.variants.len(), count);
        for v in &d.variants {
            for s in &v.spawns {
                assert!(
                    v.tiles.iter().any(|t| t.position == s.position),
                    "{} spawn has no admitted floor",
                    v.key
                );
            }
        }
        match key {
            "chakoya_iceberg" => {
                assert_eq!(d.variants[0].tiles.len(), 2118);
                assert_eq!(d.variants[0].spawns.len(), 63);
                assert!(d.variants[1].spawns.is_empty());
                assert_eq!(d.variants[1].tiles.len(), 2118);
            }
            "spirit_grounds" => {
                let mut entrances = std::collections::BTreeSet::new();
                let mut populations = std::collections::BTreeSet::new();
                for v in &d.variants {
                    assert_eq!(v.portals.len(), 2);
                    let entry = &v.portals[0];
                    let exit = &v.portals[1];
                    assert!(!entry.exit && exit.exit);
                    assert_eq!(entry.destination, (32857, 32425, 10));
                    assert_ne!(entry.position, exit.destination);
                    assert!(v.tiles.iter().any(|t| t.position == entry.destination));
                    assert!(
                        d.inactive_tiles
                            .iter()
                            .any(|t| t.position == exit.destination)
                    );
                    assert_eq!(v.spawns.len(), 4);
                    entrances.insert(entry.position);
                    populations.insert(v.spawns[0].creature.as_str());
                }
                assert_eq!(entrances.len(), 3);
                assert_eq!(populations.len(), 4);
            }
            "hive_outpost" => {
                assert_eq!(d.variants[0].spawns.len(), 6);
                assert_eq!(d.variants[1].spawns.len(), 7);
                for common in &d.variants[0].spawns {
                    assert!(d.variants[1].spawns.iter().any(|s| s.creature == common.creature && s.position == common.position));
                }
            }
            "poacher_caves" => {
                assert!(d.variants.iter().all(|v| v.spawns.len() == 12));
                assert!(d.variants.iter().all(|v| v.weight == 1));
            }
            _ => return Err("unexpected final-four key".into()),
        }
    }
    Ok(())
}
