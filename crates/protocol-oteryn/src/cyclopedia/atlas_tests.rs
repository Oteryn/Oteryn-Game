#![allow(clippy::expect_used)]
use super::*;
#[test]
fn atlas_pages_bound_geometry_and_reject_truncation_and_duplicate_areas() {
    let page = Page {
        next: Some(128),
        areas: vec![
            Area {
                id: 23,
                parent: 0,
                name: "Hrodmir".into(),
            },
            Area {
                id: 109,
                parent: 23,
                name: "Nibelor".into(),
            },
        ],
        runs: vec![Run {
            area: 109,
            x: 20,
            end: 24,
            y: 10,
            floor: -7,
        }],
    };
    let bytes = encode(&page).expect("valid atlas");
    assert_eq!(decode(&bytes), Ok(page.clone()));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut invalid = page.clone();
    invalid.areas.push(invalid.areas[0].clone());
    assert!(encode(&invalid).is_err());
    let mut invalid = page.clone();
    invalid.runs[0].end = 20;
    assert!(encode(&invalid).is_err());
    let mut invalid = page.clone();
    invalid.runs[0].area = 999;
    assert!(encode(&invalid).is_err());
    let mut invalid = page;
    invalid.runs = vec![invalid.runs[0]; RUNS_PER_PAGE + 1];
    assert!(encode(&invalid).is_err());
}
