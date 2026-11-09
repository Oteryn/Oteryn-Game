use super::Panel;
use std::fmt::Write;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub bounds: [i32; 4],
    pub color: u8,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drawing {
    pub caption: String,
    pub rects: Vec<Rect>,
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
    if !a.hide_geometry {
        for run in &a.runs {
            let [x, y] = point(run.begin, run.y);
            let right = ox + (u32::from(run.end - min_x) * numerator / denominator) as i32;
            let bottom = oy + ((u32::from(run.y - min_y) + 1) * numerator / denominator) as i32;
            draw.rects.push(Rect {
                bounds: [x, y, (right - x).max(1), (bottom - y).max(1)],
                color: (run.subarea % 6) as u8,
            });
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
