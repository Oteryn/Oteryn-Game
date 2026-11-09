//! Atlas controls consume confirmed server observations; clicks never move the character.
use super::Panel;
use oteryn_client_runtime::cyclopedia_map::atlas::{HEIGHT, Query, WIDTH};
pub fn atlas_action(p: &mut Panel, action: usize) -> bool {
    if p.pending || !p.selected {
        return false;
    }
    if (10..=13).contains(&action) {
        p.atlas_layers ^= 1 << (action - 10);
        return false;
    }
    let mut q = p.atlas.as_ref().map_or(p.atlas_query, |a| a.query);
    match action {
        0 => {}
        1 => q = Query::default(),
        2 | 3 => {
            let Some(a) = &p.atlas else {
                return false;
            };
            let floors: Vec<_> = (-15i8..=0)
                .filter(|f| a.floors & (1u16 << (-*f as u8)) != 0)
                .collect();
            let Some(i) = floors.iter().position(|f| *f == q.floor) else {
                return false;
            };
            q.floor = floors[if action == 2 {
                (i + 1) % floors.len()
            } else {
                (i + floors.len() - 1) % floors.len()
            }];
            q.page = 0;
        }
        4 | 5 => {
            let old = q.zoom;
            q.zoom = if action == 4 {
                q.zoom.saturating_sub(1)
            } else {
                q.zoom.saturating_add(1).min(8)
            };
            q.x = resize(q.x, WIDTH, old, q.zoom);
            q.y = resize(q.y, HEIGHT, old, q.zoom);
            q.page = 0;
        }
        6..=9 => {
            if p.atlas.is_none() {
                return false;
            }
            let step = 1u16 << q.zoom;
            let dx = step.saturating_mul(WIDTH as u16 / 2);
            let dy = step.saturating_mul(HEIGHT as u16 / 2);
            match action {
                6 => q.y = q.y.saturating_sub(dy),
                7 => q.y = q.y.saturating_add(dy).min(65534),
                8 => q.x = q.x.saturating_sub(dx),
                _ => q.x = q.x.saturating_add(dx).min(65534),
            }
            q.page = 0;
        }
        14 => q.page = q.page.saturating_sub(1),
        15 => {
            if p.atlas
                .as_ref()
                .is_none_or(|a| (u32::from(q.page) + 1) * 64 >= a.marker_total)
            {
                return false;
            }
            q.page = q.page.saturating_add(1);
        }
        _ => return false,
    }
    p.atlas_query = q;
    true
}
fn resize(origin: u16, cells: usize, old: u8, new: u8) -> u16 {
    if origin == 65535 {
        return origin;
    }
    let center = u32::from(origin) + ((cells as u32) << old) / 2;
    center
        .saturating_sub(((cells as u32) << new) / 2)
        .min(65534) as u16
}
pub fn atlas_detail(p: &Panel) -> String {
    if !p.selected {
        return "Atlas unavailable in this session".into();
    }
    let Some(a) = &p.atlas else {
        return if p.pending {
            "Loading map".into()
        } else {
            format!("Refresh to load the atlas. {}", p.feedback)
        };
    };
    let index = p.atlas_cell.min(WIDTH * HEIGHT - 1);
    let id = a.cells[index];
    let sub = a.areas.iter().find(|a| a.id == id);
    let region = sub.and_then(|s| a.areas.iter().find(|a| a.id == s.parent));
    let step = 1u32 << a.query.zoom;
    format!(
        "Floor {} | {} tiles/cell | origin {},{} | marker page {} ({} total)\r\nRegion: {} | Subarea: {}\r\nSelected map cell {},{} (region overview at this zoom).\r\nNPC {} | Passages {} | POI {} | Regions {}\r\n{}\r\n{}",
        a.query.floor,
        step,
        a.query.x,
        a.query.y,
        a.query.page + 1,
        a.marker_total,
        region.map_or("Unknown", |r| r.name.as_str()),
        sub.map_or("No membership coverage", |r| r.name.as_str()),
        u32::from(a.query.x) + (index % WIDTH) as u32 * step,
        u32::from(a.query.y) + (index / WIDTH) as u32 * step,
        layer(p, 0),
        layer(p, 1),
        layer(p, 2),
        layer(p, 3),
        if a.progress.is_some() {
            "Exploration progress available"
        } else {
            "Exploration unavailable at the current position"
        },
        if p.pending {
            "Waiting for the server; showing the previous confirmed atlas"
        } else {
            p.feedback.as_str()
        }
    )
}
fn layer(p: &Panel, bit: u8) -> &'static str {
    if p.atlas_layers & (1 << bit) != 0 {
        "shown"
    } else {
        "hidden"
    }
}
/// Original colour palette, stable by region; no proprietary map image is used.
pub fn atlas_color(p: &Panel, id: u32) -> u32 {
    if id == 0 || p.atlas_layers & 8 == 0 {
        return 0x282522;
    }
    let parent = p
        .atlas
        .as_ref()
        .and_then(|a| a.areas.iter().find(|a| a.id == id))
        .map_or(id, |a| if a.parent == 0 { a.id } else { a.parent });
    let seed = parent.wrapping_mul(2654435761);
    let r = 64 + (seed & 127);
    let g = 64 + ((seed >> 8) & 127);
    let b = 64 + ((seed >> 16) & 127);
    r | g << 8 | b << 16
}
