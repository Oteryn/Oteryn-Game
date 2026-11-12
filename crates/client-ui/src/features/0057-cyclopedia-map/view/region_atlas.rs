use super::Panel;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRun {
    pub area: u32,
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}
/// Fit exact half-open coverage to a large map viewport; gaps remain uncovered.
pub fn atlas_runs(panel: &Panel, width: u32, height: u32) -> Vec<AtlasRun> {
    let runs: Vec<_> = panel
        .region_atlas
        .iter()
        .flat_map(|a| &a.runs)
        .filter(|r| r.floor == panel.floor)
        .collect();
    let Some(min_x) = runs.iter().map(|r| r.x).min() else {
        return vec![];
    };
    let min_y = runs.iter().map(|r| r.y).min().unwrap_or(0);
    let span_x = u32::from(runs.iter().map(|r| r.end).max().unwrap_or(min_x)) - u32::from(min_x);
    let span_y = u32::from(runs.iter().map(|r| r.y).max().unwrap_or(min_y)) - u32::from(min_y) + 1;
    let scale = |n: u32, size: u32, span: u32| -> u32 {
        (u64::from(n) * u64::from(size) / u64::from(span.max(1))) as u32
    };
    runs.into_iter()
        .map(|r| AtlasRun {
            area: r.area,
            left: scale(u32::from(r.x - min_x), width, span_x),
            right: scale(u32::from(r.end - min_x), width, span_x),
            top: scale(u32::from(r.y - min_y), height, span_y),
            bottom: scale(u32::from(r.y - min_y) + 1, height, span_y),
        })
        .collect()
}
/// Visible exploration points use the same atlas transform as region geometry.
pub fn atlas_pois(panel: &Panel, width: u32, height: u32) -> Vec<super::MapPoint> {
    if panel.hide_pois {
        return vec![];
    }
    let runs: Vec<_> = panel
        .region_atlas
        .iter()
        .flat_map(|a| &a.runs)
        .filter(|r| r.floor == panel.floor)
        .collect();
    let Some(min_x) = runs.iter().map(|r| r.x).min() else {
        return vec![];
    };
    let min_y = runs.iter().map(|r| r.y).min().unwrap_or(0);
    let max_x = runs.iter().map(|r| r.end).max().unwrap_or(min_x);
    let max_y = runs.iter().map(|r| r.y).max().unwrap_or(min_y);
    panel
        .map
        .iter()
        .flat_map(|m| &m.visible)
        .filter(|p| {
            p.2 == panel.floor && p.0 >= min_x && p.0 < max_x && p.1 >= min_y && p.1 <= max_y
        })
        .map(|&position| super::MapPoint {
            position,
            column: (u64::from(position.0 - min_x) * u64::from(width)
                / u64::from((max_x - min_x).max(1))) as u32,
            row: (u64::from(position.1 - min_y) * u64::from(height)
                / u64::from(u32::from(max_y - min_y) + 1)) as u32,
        })
        .collect()
}
pub(super) fn text(panel: &Panel) -> String {
    use std::fmt::Write;
    let Some(atlas) = &panel.region_atlas else {
        return "Load region atlas to browse admitted regions".into();
    };
    let mut text = format!(
        "Region atlas — floor {} — {} coverage runs loaded{}\r\n",
        panel.floor,
        atlas.runs.len(),
        if atlas.next.is_some() {
            " (more pages)"
        } else {
            " (complete)"
        }
    );
    let ids: std::collections::BTreeSet<_> = atlas
        .runs
        .iter()
        .filter(|r| r.floor == panel.floor)
        .map(|r| r.area)
        .collect();
    for area in atlas.areas.iter().filter(|a| ids.contains(&a.id)) {
        if let Some(parent) = atlas.areas.iter().find(|a| a.id == area.parent) {
            let _ = write!(text, "{} > {} [#{}]\r\n", parent.name, area.name, area.id);
        }
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_projection_preserves_gaps_and_selected_floor_at_coordinate_limits() {
        use oteryn_client_runtime::cyclopedia_map::region_atlas::{Page, Run};
        let mut panel = Panel {
            floor: -7,
            region_atlas: Some(Page {
                next: None,
                areas: vec![],
                runs: vec![
                    Run {
                        area: 1,
                        x: 65520,
                        end: 65525,
                        y: 65534,
                        floor: -7,
                    },
                    Run {
                        area: 2,
                        x: 65530,
                        end: 65535,
                        y: 65534,
                        floor: -7,
                    },
                    Run {
                        area: 3,
                        x: 0,
                        end: 10,
                        y: 1,
                        floor: -8,
                    },
                ],
            }),
            ..Default::default()
        };
        let runs = atlas_runs(&panel, 900, 600);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].left, 0);
        assert!(runs[0].right < runs[1].left);
        assert_eq!(runs[1].right, 900);
        assert_eq!(runs[0].bottom, 600);
        panel.map = Some(oteryn_client_runtime::cyclopedia_map::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: vec![(65530, 65534, -7), (65530, 65534, -8), (0, 0, -7)],
        });
        let points = atlas_pois(&panel, 900, 600);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].column, 600);
        assert_eq!(points[0].row, 0);
        panel.hide_pois = true;
        assert!(atlas_pois(&panel, 900, 600).is_empty());
    }
}
