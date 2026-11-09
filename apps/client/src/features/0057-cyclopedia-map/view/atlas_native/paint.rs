use super::*;
use oteryn_client_ui::cyclopedia_map::atlas_color;
pub(super) unsafe fn draw(hwnd: HWND, p: &Panel) {
    unsafe {
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return;
        }
        let mut brushes = std::collections::BTreeMap::new();
        for row in 0..40 {
            for col in 0..64 {
                let id = p.atlas.as_ref().map_or(0, |a| a.cells[row * 64 + col]);
                let color = atlas_color(p, id);
                let brush = *brushes
                    .entry(color)
                    .or_insert_with(|| CreateSolidBrush(color));
                let r = RECT {
                    left: 10 + col as i32 * 12,
                    top: 90 + row as i32 * 12,
                    right: 22 + col as i32 * 12,
                    bottom: 102 + row as i32 * 12,
                };
                FillRect(dc, &r, brush);
            }
        }
        if let Some(a) = &p.atlas {
            let step = 1u32 << a.query.zoom;
            for marker in &a.markers {
                if p.atlas_layers & (1 << (marker.kind - 1)) == 0 {
                    continue;
                }
                let Some(col) = u32::from(marker.x)
                    .checked_sub(u32::from(a.query.x))
                    .map(|x| x / step)
                    .filter(|c| *c < 64)
                else {
                    continue;
                };
                let Some(row) = u32::from(marker.y)
                    .checked_sub(u32::from(a.query.y))
                    .map(|y| y / step)
                    .filter(|r| *r < 40)
                else {
                    continue;
                };
                let color = match marker.kind {
                    1 => 0xffaa44,
                    2 => 0xee66bb,
                    _ => 0x44eeee,
                };
                let brush = CreateSolidBrush(color);
                let r = RECT {
                    left: 12 + col as i32 * 12,
                    top: 92 + row as i32 * 12,
                    right: 20 + col as i32 * 12,
                    bottom: 100 + row as i32 * 12,
                };
                FillRect(dc, &r, brush);
                DeleteObject(brush);
            }
        }
        for (_, brush) in brushes {
            DeleteObject(brush);
        }
        ReleaseDC(hwnd, dc);
    }
}
pub(super) fn legend(p: &Panel) -> String {
    use std::fmt::Write;
    let Some(a) = &p.atlas else {
        return "No confirmed atlas. Refresh to load.\r\nDark cells: no map coverage.\r\nBlue: NPC\r\nPurple: passage\r\nYellow: visible exploration POI".into();
    };
    let mut s = String::from("Regions and subareas\r\n");
    for area in &a.areas {
        let _ = write!(
            s,
            "{}{} — {}\r\n",
            if area.parent == 0 {
                "Region "
            } else {
                "  Subarea "
            },
            area.id,
            area.name
        );
    }
    s.push_str("\r\nMarkers (current page)\r\n");
    for marker in &a.markers {
        if p.atlas_layers & (1 << (marker.kind - 1)) != 0 {
            let _ = write!(
                s,
                "{} {},{} — {}\r\n",
                match marker.kind {
                    1 => "NPC",
                    2 => "Passage",
                    _ => "POI",
                },
                marker.x,
                marker.y,
                marker.label
            );
        }
    }
    s.push_str("\r\nBlue: NPC | Purple: passage | Yellow: POI\r\nDark: unknown membership. Click the map for its region. Overview cells group nearby map tiles; this map does not show a walking route.");
    s
}
