use super::*;
use oteryn_world_bundle::subarea::{Area, Run};

#[test]
fn raid_warning_windows_use_full_area_discovery_and_ground_floor_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue = RaidCatalogue::embedded()?;
    let key = crate::content::ProductionKey::new("raid:libertybay.ferumbras")?;
    let raid = catalogue
        .get(&key)
        .ok_or("Ferumbras raid")?
        .select([1; 32], [2; 16])?;
    let wave = raid.definition.waves.first().ok_or("first wave")?;
    let spot = raid
        .areas
        .get(&wave.entries.first().ok_or("first entry")?.area_anchor)
        .ok_or("first wave location")?
        .from;
    let x = u16::try_from(spot.x)?;
    let y = u16::try_from(spot.y)?;
    let floor = i8::try_from(-spot.floor)?;
    let area = |id, parent| Area {
        id,
        key: String::new(),
        name: String::new(),
        parent,
        source_record_sha256: String::new(),
        town_id: None,
        hometown: false,
        temple: None,
    };
    let mut map = Table {
        areas: vec![area(1, None), area(2, Some(1)), area(3, Some(1))],
        runs: vec![Run {
            floor,
            y,
            begin: x,
            end: x + 1,
            subarea: 2,
        }],
        ..Table::default()
    };
    let fired_at = 2 * AREA_LEAD_MS;
    let start = fired_at + i64::from(wave.delay_ms);
    let region = Warning {
        area: 1,
        subarea: None,
    };
    let subarea = Warning {
        area: 1,
        subarea: Some(2),
    };
    let ground = Run {
        floor: GROUND_FLOOR,
        y,
        begin: x,
        end: x + 1,
        subarea: 2,
    };
    assert_eq!(
        raid_warnings(&map, &raid, fired_at, start - SUBAREA_LEAD_MS, |_| true),
        BTreeSet::from([region])
    );
    map.runs.push(ground);
    map.runs.sort_by_key(|run| (run.floor, run.y, run.begin));
    for (lead, expected) in [
        (AREA_LEAD_MS + 1, BTreeSet::new()),
        (AREA_LEAD_MS, BTreeSet::from([region])),
        (SUBAREA_LEAD_MS + 1, BTreeSet::from([region])),
        (SUBAREA_LEAD_MS, BTreeSet::from([region, subarea])),
        (1, BTreeSet::from([region, subarea])),
        (0, BTreeSet::new()),
    ] {
        assert_eq!(
            raid_warnings(&map, &raid, fired_at, start - lead, |_| true),
            expected,
            "lead {lead}"
        );
        assert!(raid_warnings(&map, &raid, fired_at, start - lead, |id| id == 2).is_empty());
    }
    Ok(())
}
