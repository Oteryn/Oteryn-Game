//! Pin the authored precondition to the already admitted raid and native map anchor.
#[test]
fn worldchg_stampede_content_qualifies_native_wave_region_and_achievement()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_content::content::ProductionKey;
    use oteryn_gameplay_encounter_rules::raid::{RaidCatalogue, RaidEndPolicy, RaidPolicy};
    use sha2::{Digest, Sha256};
    let catalogue = RaidCatalogue::embedded()?;
    let key = ProductionKey::new("raid:porthope.stampede")?;
    let raid = catalogue.get(&key).ok_or("Stampede")?;
    let selected = raid.select([7; 32], [8; 16])?;
    assert_eq!(
        selected.definition.policy,
        RaidPolicy::Random {
            mean_interval_s: 86400,
            min_gap_s: 0
        }
    );
    assert!(matches!(
        raid.schedule,
        oteryn_gameplay_encounter_rules::raid::CatalogueSchedule::LegacyXml {
            margin_ms: 21600000,
            ..
        }
    ));
    assert_eq!(selected.end_policy, RaidEndPolicy::UntilReset);
    assert_eq!(selected.definition.announcements[0].delay_ms, 1000);
    assert_eq!(selected.definition.waves[0].delay_ms, 2000);
    let entry = &selected.definition.waves[0].entries[0];
    assert_eq!(entry.count, 16);
    assert_eq!(
        entry.creature.as_str(),
        "oteryn:creature.terrified_elephant"
    );
    let area = selected
        .areas
        .get(&entry.area_anchor)
        .ok_or("native area")?;
    assert_eq!(
        (area.from.x, area.from.y, area.from.floor),
        (32825, 32667, 7)
    );
    assert_eq!((area.to.x, area.to.y, area.to.floor), (32888, 32675, 7));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for region in &raid.variants[0].waves[0].entries[0].area.regions {
        let bytes = std::fs::read(root.join(&region.path))?;
        assert_eq!(
            Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            region.sha256
        );
    }
    let keys = |count| {
        oteryn_gameplay_progression::bestiary_achievement::kill_count_keys(
            "oteryn:creature.terrified_elephant",
            count,
        )
        .collect::<Vec<_>>()
    };
    assert!(keys(4).is_empty());
    assert_eq!(keys(5), ["oteryn:achievement/trail_of_the_ape_god"]);
    let requirements = crate::durability::world_event::changes::raid_requirements()?;
    assert_eq!(
        requirements
            .get("raid:porthope.stampede")
            .map(String::as_str),
        Some("stampede")
    );
    assert_eq!(
        requirements
            .get("raid:furyfate.furyosa")
            .map(String::as_str),
        Some("fury_gates")
    );
    assert!(!requirements.contains_key("raid:badger"));
    Ok(())
}
