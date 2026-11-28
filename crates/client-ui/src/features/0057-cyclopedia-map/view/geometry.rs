use super::Panel;
use std::fmt::Write;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub bounds: [i32; 4],
    pub color: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    Area,
    Subarea,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drawing {
    pub caption: String,
    pub rects: Vec<Rect>,
    pub raid_warnings: Vec<([i32; 4], WarningLevel)>,
    pub points: Vec<[i32; 2]>,
}
pub fn text(panel: &Panel) -> String {
    let a = &panel.geometry;
    let mut text = format!("World atlas — floor {}\r\n{}\r\n", a.floor, a.feedback);
    for area in &a.areas {
        let _ = writeln!(
            text,
            "{}{}: {}\r",
            if Some(area.id) == a.selected {
                "> "
            } else {
                ""
            },
            area.id,
            area.name
        );
    }
    let _ = write!(
        text,
        "Loaded map spans: {}/{}\r\nRegion layer: {} | POI layer: {}\r\n",
        a.runs.len(),
        a.total,
        !a.hide_geometry,
        !panel.hide_pois
    );
    if panel.pending {
        text.push_str("Waiting for the server\r\n");
    }
    text.push_str("Current-location exploration (not the selected atlas region):\r\n");
    text.push_str(&panel.summary);
    text
}
pub fn project(panel: &Panel) -> Drawing {
    let a = &panel.geometry;
    let mut draw = Drawing {
        caption: format!(
            "{} — floor {} · north up · {}/{} map spans",
            a.area().map_or("Choose an area", |a| a.name.as_str()),
            a.floor,
            a.runs.len(),
            a.total
        ),
        ..Drawing::default()
    };
    let Some([min_x, min_y, max_x, max_y]) = a.bounds else {
        draw.caption.push_str(" — Load area");
        return draw;
    };
    let width = u32::from(max_x - min_x) + 1;
    let height = u32::from(max_y - min_y) + 1;
    let (numerator, denominator) = if width * 400 >= height * 600 {
        (600u32, width)
    } else {
        (400u32, height)
    };
    let ox = 16 + (600 - width * numerator / denominator) as i32 / 2;
    let oy = 66 + (400 - height * numerator / denominator) as i32 / 2;
    let point = |x: u16, y: u16| {
        [
            ox + (u32::from(x - min_x) * numerator / denominator) as i32,
            oy + (u32::from(y - min_y) * numerator / denominator) as i32,
        ]
    };
    let warned_area = a
        .area()
        .map(|area| {
            if area.parent == 0 {
                area.id
            } else {
                area.parent
            }
        })
        .filter(|&area| panel.raid_warnings.area_warned(area));
    if !a.hide_geometry || warned_area.is_some() {
        for run in &a.runs {
            let [x, y] = point(run.begin, run.y);
            let right = ox + (u32::from(run.end - min_x) * numerator / denominator) as i32;
            let bottom = oy + ((u32::from(run.y - min_y) + 1) * numerator / denominator) as i32;
            let bounds = [x, y, (right - x).max(1), (bottom - y).max(1)];
            if !a.hide_geometry {
                draw.rects.push(Rect {
                    bounds,
                    color: (run.subarea % 6) as u8,
                });
            }
            if let Some(area) = warned_area {
                draw.raid_warnings.push((bounds, WarningLevel::Area));
                if panel.raid_warnings.subarea_warned(area, run.subarea) {
                    draw.raid_warnings.push((bounds, WarningLevel::Subarea));
                }
            }
        }
    }
    if !panel.hide_pois {
        for &(x, y, floor) in panel.map.iter().flat_map(|m| &m.visible) {
            if floor == a.floor && (min_x..=max_x).contains(&x) && (min_y..=max_y).contains(&y) {
                draw.points.push(point(x, y));
            }
        }
    }
    draw
}
