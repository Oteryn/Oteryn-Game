use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    if panel.atlas.is_some() {
        return format!("Cyclopedia atlas\r\n{}", super::atlas_detail(panel));
    }
    format!(
        "Cyclopedia map\r\nWorld atlas: Ctrl+Alt+Shift+F9\r\n{}\r\n{}\r\n{}",
        panel.summary,
        map_text(panel),
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
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
        return "Query to load the exploration map".into();
    }
    let points = map_points(panel);
    let mut out = format!(
        "Visible POI overview — floor {} (north up; fitted to visible points)\r\n",
        panel.floor
    );
    if panel.hide_pois {
        out.push_str("POI layer hidden\r\n");
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
    out.push_str(
        "Only server-visible POIs are shown. Region, passage and NPC geometry is unavailable.\r\n",
    );
    out
}
