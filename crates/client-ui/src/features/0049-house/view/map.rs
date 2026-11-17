use super::Panel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHouse {
    pub label: String,
    pub position: (u16, u16, i8),
    pub pixel: (i32, i32),
    pub selected: bool,
}

/// North-up entrance map for the selected floor of the confirmed catalogue page.
pub fn map_houses(panel: &Panel) -> Vec<MapHouse> {
    let Some((_, _, floor)) = panel.page.rows.get(panel.index).and_then(|r| r.entrance) else {
        return Vec::new();
    };
    let entrances = || {
        panel
            .page
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| row.entrance.filter(|p| p.2 == floor).map(|p| (i, row, p)))
    };
    let min_x = entrances().map(|(_, _, p)| p.0).min().unwrap_or(0);
    let max_x = entrances().map(|(_, _, p)| p.0).max().unwrap_or(min_x);
    let min_y = entrances().map(|(_, _, p)| p.1).min().unwrap_or(0);
    let max_y = entrances().map(|(_, _, p)| p.1).max().unwrap_or(min_y);
    entrances()
        .map(|(i, row, position)| MapHouse {
            label: format!("{}: {}", i + 1, row.name),
            position,
            pixel: (
                20 + i32::from(position.0 - min_x) * 400 / i32::from((max_x - min_x).max(1)),
                50 + i32::from(position.1 - min_y) * 300 / i32::from((max_y - min_y).max(1)),
            ),
            selected: i == panel.index,
        })
        .collect()
}
