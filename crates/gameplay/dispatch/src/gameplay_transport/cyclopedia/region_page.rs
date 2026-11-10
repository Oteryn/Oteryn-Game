//! Exact region geometry, bounded independently from character exploration POIs.
use oteryn_protocol_oteryn::cyclopedia::region_atlas::{
    self as atlas, Area, Page, RUNS_PER_PAGE, Run,
};
use oteryn_world_bundle::subarea::Table;

pub(super) fn page(table: &Table, after: u32) -> Option<Vec<u8>> {
    let start = usize::try_from(after).ok()?;
    if start > table.runs.len() {
        return None;
    }
    let end = start.saturating_add(RUNS_PER_PAGE).min(table.runs.len());
    let runs: Vec<_> = table.runs[start..end]
        .iter()
        .map(|r| Run {
            area: r.subarea,
            x: r.begin,
            end: r.end,
            y: r.y,
            floor: r.floor,
        })
        .collect();
    let mut ids = std::collections::BTreeSet::new();
    for run in &runs {
        let a = table.areas.iter().find(|a| a.id == run.area)?;
        ids.insert(a.id);
        ids.insert(a.parent?);
    }
    let areas = table
        .areas
        .iter()
        .filter(|a| ids.contains(&a.id))
        .map(|a| Area {
            id: a.id,
            parent: a.parent.unwrap_or(0),
            name: a.name.clone(),
        })
        .collect();
    atlas::encode(&Page {
        next: (end < table.runs.len()).then_some(u32::try_from(end).ok()?),
        areas,
        runs,
    })
    .ok()
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn atlas_pages_preserve_native_region_names_and_exact_sparse_coverage() {
        let table = Table {
            areas: vec![
                oteryn_world_bundle::subarea::Area {
                    id: 23,
                    key: String::new(),
                    name: "Hrodmir".into(),
                    parent: None,
                    source_record_sha256: String::new(),
                    town_id: None,
                    hometown: false,
                    temple: None,
                },
                oteryn_world_bundle::subarea::Area {
                    id: 109,
                    key: String::new(),
                    name: "Nibelor".into(),
                    parent: Some(23),
                    source_record_sha256: String::new(),
                    town_id: None,
                    hometown: false,
                    temple: None,
                },
            ],
            runs: (0..130)
                .map(|y| oteryn_world_bundle::subarea::Run {
                    floor: -7,
                    y,
                    begin: 20,
                    end: 24,
                    subarea: 109,
                })
                .collect(),
            ..Default::default()
        };
        let first = atlas::decode(&page(&table, 0).expect("first")).expect("wire");
        assert_eq!(first.next, Some(64));
        assert_eq!(first.runs.len(), 64);
        assert_eq!(first.areas[1].name, "Nibelor");
        assert_eq!(first.areas[1].parent, 23);
        let last = atlas::decode(&page(&table, 128).expect("last")).expect("wire");
        assert_eq!(last.runs[0].y, 128);
        assert_eq!(last.next, None);
        assert!(page(&table, 131).is_none());
    }
}
