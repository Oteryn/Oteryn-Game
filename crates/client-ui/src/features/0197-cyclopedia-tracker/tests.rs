use super::*;
use oteryn_client_runtime::cyclopedia_tracker::{
    TrackerEntry, TrackerSnapshot, encode_domain_snapshot, encode_snapshot,
};
fn entry(key: &str, kills: u32, completed: bool) -> TrackerEntry {
    TrackerEntry {
        creature_key: key.into(),
        kill_count: kills,
        thresholds: [5, 10, 20],
        completed,
    }
}
#[test]
fn bestiary_widget_keeps_its_snapshot_when_a_boss_update_arrives_and_filters_sorted_rows()
-> Result<(), &'static str> {
    let mut p = CyclopediaTrackerPanel::default();
    let bestiary = TrackerSnapshot {
        boss: false,
        entries: vec![
            entry("oteryn:creature.rat", 19, false),
            entry("oteryn:creature.ant", 20, true),
            entry("oteryn:creature.bear", 2, false),
        ],
    };
    let bosses = TrackerSnapshot {
        boss: true,
        entries: vec![entry("oteryn:creature.boss", 1, false)],
    };
    assert!(apply_snapshot(
        &mut p,
        &encode_domain_snapshot(&bestiary, &bosses).ok_or("encode")?
    ));
    assert!(apply_snapshot(
        &mut p,
        &encode_snapshot(&bosses).ok_or("encode")?
    ));
    p.hide_completed = true;
    p.sort = TrackerSort::Remaining;
    let rows = bestiary_rows(&p);
    assert_eq!(
        rows.iter()
            .map(|e| e.creature_key.as_str())
            .collect::<Vec<_>>(),
        vec!["oteryn:creature.rat", "oteryn:creature.bear"]
    );
    p.search = "BEAR".into();
    assert_eq!(bestiary_rows(&p).len(), 1);
    assert_eq!(bestiary_rows(&p)[0].kill_count, 2);
    Ok(())
}
