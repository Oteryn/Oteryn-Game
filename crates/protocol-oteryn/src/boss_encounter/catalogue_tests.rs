#![allow(clippy::expect_used)]
use super::*;
#[test]
fn boss_catalogue_bounds_filters_stages_and_portrait_identity() {
    let query = Query {
        after: 64,
        category: 2,
        search: "Oberon".into(),
    };
    let bytes = query.encode().expect("query");
    assert_eq!(Query::decode(&bytes), Some(query));
    let entry = Entry {
        key: "oteryn:creature.grand_master_oberon".into(),
        name: "Grand Master Oberon".into(),
        category: 2,
        kills: 20,
        thresholds: [5, 20, 60],
        points: [10, 40, 100],
        look: 1250,
        object: false,
    };
    let page = Page {
        next: Some(128),
        points: 140,
        entries: vec![entry],
    };
    let bytes = page.encode().expect("page");
    assert_eq!(Page::decode(&bytes), Some(page.clone()));
    for end in 0..bytes.len() {
        assert!(Page::decode(&bytes[..end]).is_none());
    }
    let mut bad = page.clone();
    bad.entries[0].thresholds = [5, 5, 60];
    assert!(bad.encode().is_none());
    let mut bad = page.clone();
    bad.entries[0].category = 0;
    assert!(bad.encode().is_none());
    let mut bad = page.clone();
    bad.entries = vec![bad.entries[0].clone(); PER_PAGE + 1];
    assert!(bad.encode().is_none());
    assert!(
        Query {
            after: 0,
            category: 4,
            search: String::new()
        }
        .encode()
        .is_none()
    );
    assert!(
        Query {
            after: 0,
            category: 0,
            search: "x".repeat(49)
        }
        .encode()
        .is_none()
    );
}
