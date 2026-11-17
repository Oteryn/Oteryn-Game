use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    if panel.atlas.is_some() {
        return format!(
            "Cyclopedia atlas\r\n{}\r\n{}",
            super::atlas_detail(panel),
            discovery_speed_tooltip(panel)
        );
    }
    format!(
        "Cyclopedia map\r\nWorld atlas: Ctrl+Alt+Shift+F9\r\n{}\r\n{}\r\n{}\r\n{}\r\n{}",
        discovery_speed_tooltip(panel),
        panel.summary,
        super::region_atlas_view::text(panel),
        map_text(panel),
        if panel.pending || panel.speed_pending {
            tr!("Waiting for the server")
        } else if !panel.selected && panel.feedback.is_empty() {
            tr!("Unavailable in this session")
        } else {
            &panel.feedback
        }
    )
}

/// Registration target for the Cyclopedia map navigation entry.
pub fn map_page() {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPoint {
    pub column: u32,
    pub row: u32,
    pub position: (u16, u16, i8),
}
/// Project the server's visible POIs; unknown region geometry is never invented.
pub fn map_points(panel: &Panel) -> Vec<MapPoint> {
    if panel.hide_pois {
        return Vec::new();
    }
    let points: Vec<_> = panel
        .map
        .iter()
        .flat_map(|map| &map.visible)
        .filter(|p| p.2 == panel.floor)
        .copied()
        .collect();
    let min_x = points.iter().map(|p| p.0).min().unwrap_or(0);
    let min_y = points.iter().map(|p| p.1).min().unwrap_or(0);
    let width = u32::from(points.iter().map(|p| p.0).max().unwrap_or(min_x) - min_x).max(1);
    let height = u32::from(points.iter().map(|p| p.1).max().unwrap_or(min_y) - min_y).max(1);
    points
        .into_iter()
        .map(|position| MapPoint {
            column: u32::from(position.0 - min_x) * 31 / width,
            row: u32::from(position.1 - min_y) * 11 / height,
            position,
        })
        .collect()
}
pub fn map_text(panel: &Panel) -> String {
    use std::fmt::Write;
    if panel.map.is_none() {
        return tr!("Query to load the exploration map").into();
    }
    let points = map_points(panel);
    let mut out = lfmt!(
        "Visible POI overview — floor {} (north up; fitted to visible points)\r\n",
        panel.floor
    );
    if panel.hide_pois {
        out.push_str(tr!("POI layer hidden\r\n"));
    }
    for row in 0..12 {
        for column in 0..32 {
            out.push(
                if points.iter().any(|p| p.column == column && p.row == row) {
                    '◆'
                } else {
                    '·'
                },
            );
        }
        out.push_str("\r\n");
    }
    for point in points {
        let (x, y, z) = point.position;
        let _ = write!(out, "◆ {x},{y},{z}\r\n");
    }
    out.push_str(tr!(
        "Only server-visible POIs are shown. Region, passage and NPC geometry is unavailable.\r\n"
    ));
    out
}

/// Reference manual §3.2.2 and the official 8834 source capture: range only.
pub fn discovery_speed_tooltip(panel: &Panel) -> String {
    format!(
        "{}\r\n{} +{}",
        tr!(
            "Discovery speed bonus: +100–195 in fully discovered areas, depending on the number of fully discovered areas. Resetting a subarea removes its discovery credit."
        ),
        tr!("Current discovery speed bonus:"),
        panel.speed_bonus
    )
}
