use super::*;
use oteryn_client_runtime::{
    boss_encounter::catalogue::{Entry, Page},
    raw_feature::{CommandStatus, GatedCommandResult},
};
#[test]
fn boss_gallery_refusal_bad_cursor_and_category_mismatch_preserve_confirmed_cards() {
    let mut p = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    let first = Page {
        next: Some(64),
        points: 50,
        entries: vec![Entry {
            key: "oteryn:creature.boss".into(),
            name: "Boss".into(),
            category: 1,
            kills: 25,
            thresholds: [25, 100, 300],
            points: [5, 20, 50],
            look: 21,
            object: false,
        }],
    };
    assert!(request(&mut p, Intent::Catalogue).is_some());
    let result = |status, payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    };
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert!(request(&mut p, Intent::CatalogueNext).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Rejected, vec![])
    ));
    assert_eq!(p.catalogue, Some(first.clone()));
    assert!(request(&mut p, Intent::CatalogueNext).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert_eq!(p.catalogue, Some(first.clone()));
    p.browser_category = BossCategory::Archfoe;
    assert!(request(&mut p, Intent::Catalogue).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert_eq!(p.catalogue, Some(first));
    assert_eq!(
        [
            BossCategory::Bane.server_filter(),
            BossCategory::Archfoe.server_filter(),
            BossCategory::Nemesis.server_filter()
        ],
        [1, 2, 3]
    );
}
