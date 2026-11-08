use super::*;
use oteryn_client_runtime::boss_encounter::Snapshot;
#[test]
fn boss_browser_search_categories_and_detail_selection_use_server_records() {
    let mut p = BossEncounterPanel {
        selected: true,
        snapshot: Some(Snapshot {
            now: 100,
            progress: vec![("oteryn:dragon".into(), 7), ("oteryn:rat".into(), 1)],
            cooldowns: vec![("oteryn:dragon".into(), 200)],
            ..Default::default()
        }),
        ..Default::default()
    };
    p.browser_search = " DRAGON ".into();
    assert_eq!(boss_records(&p), ["oteryn:dragon"]);
    assert!(select_boss(&mut p, true));
    assert_eq!(p.fields[0], "oteryn:dragon");
    assert!(browser_text(&p).contains("7 kills"));
    assert!(browser_text(&p).contains("100 ms"));
    p.browser_search.clear();
    p.browser_category = BossCategory::Cooldown;
    assert_eq!(boss_records(&p).len(), 1);
    p.pending = true;
    assert!(!select_boss(&mut p, false));
}
