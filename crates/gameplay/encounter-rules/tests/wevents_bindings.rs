use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};

#[test]
fn wevents_flower_four_area_cycle_and_single_horestis_are_native_raids()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue = RaidCatalogue::embedded().map_err(|e| format!("{e:?}"))?;
    let flower = catalogue
        .get(&ProductionKey::new("oteryn:raid.raid.dryad_invasion")?)
        .ok_or("Flower")?;
    assert!(flower.active);
    assert!(matches!(
        flower.schedule,
        CatalogueSchedule::Calendar {
            interval_ms: Some(3_600_000),
            repeat_persistent: true,
            start_immediately: true,
            ..
        }
    ));
    let selected = flower
        .select([7; 32], [8; 16])
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(selected.definition.waves.len(), 4);
    for (index, wave) in selected.definition.waves.iter().enumerate() {
        assert_eq!(u64::from(wave.delay_ms), u64::try_from(index)? * 900_000);
        assert_eq!(wave.entries.len(), 1);
        assert_eq!(wave.entries[0].count, 25);
    }
    let mummy = catalogue
        .get(&ProductionKey::new("raid:world-events.horestis")?)
        .ok_or("Horestis")?;
    assert!(mummy.active);
    assert!(matches!(
        mummy.schedule,
        CatalogueSchedule::Calendar {
            start_immediately: true,
            ..
        }
    ));
    let selected = mummy
        .select([7; 32], [8; 16])
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(selected.definition.waves.len(), 1);
    assert_eq!(selected.definition.waves[0].entries[0].count, 1);
    Ok(())
}
