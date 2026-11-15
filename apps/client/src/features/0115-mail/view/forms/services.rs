use super::*;
use crate::play::{feature_0049_house, feature_0057_cyclopedia_map};
pub(super) fn world_event() -> Form {
    Form {
        title: "World events",
        labels: &[],
        buttons: &["Refresh"],
        snapshot: |m| {
            (
                m.world_event.open,
                m.world_event.pending,
                vec![],
                oteryn_client_ui::world_event::panel_text(&m.world_event),
            )
        },
        set_open: |m, value| m.world_event.open = value,
        submit: |m, commands, _, _| {
            if let Some(command) = oteryn_client_ui::world_event::request(&mut m.world_event)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                m.world_event.pending = false;
                m.world_event.feedback = Some("The session has ended");
            }
        },
    }
}
pub(super) fn atlas() -> Form {
    Form {
        title: "World atlas",
        labels: &[],
        buttons: &[
            "Regions",
            "Previous area",
            "Next area",
            "Subareas",
            "Top-level regions",
            "Load map",
            "Floor up",
            "Floor down",
            "Region layer",
            "POI layer",
            "Refresh visible POIs",
            "Show graphical map",
            "Next area page",
            "Previous area page",
        ],
        snapshot: |m| {
            let p = &m.properties.cyclopedia_map;
            (
                p.geometry.open,
                p.pending || p.speed_pending,
                vec![],
                oteryn_client_ui::cyclopedia_map::atlas_view::text(p),
            )
        },
        set_open: |m, value| m.properties.cyclopedia_map.geometry.open = value,
        submit: |m, commands, action, _| {
            if action == 11 {
                m.properties.cyclopedia_map.geometry.show_map = true;
            } else {
                feature_0057_cyclopedia_map::input::atlas::submit(m, commands, action);
            }
        },
    }
}
pub(super) fn ownership() -> Form {
    Form {
        title: "House ownership and move-out",
        labels: &[
            "House row (1 onwards)",
            "Recipient",
            "Date (UTC)",
            "Gold price",
        ],
        buttons: &[
            "Refresh",
            "Previous page",
            "Next page",
            "Select house",
            "Review move-out",
            "Review cancel move-out",
            "Review transfer",
            "Review cancel transfer",
            "Review accept transfer",
            "Review reject transfer",
            "Confirm reviewed action",
            "Cancel review",
        ],
        snapshot: |m| {
            let p = &m.properties.house.ownership;
            let mut text = oteryn_client_ui::house::ownership_text(&p.model);
            if let Some(review) = &p.form_review {
                text.push_str("\n\n");
                text.push_str(review.text());
            }
            (
                p.open && p.model.available(),
                p.model.pending(),
                std::iter::once((p.model.index() + 1).to_string())
                    .chain(p.fields.iter().cloned())
                    .collect(),
                text,
            )
        },
        set_open: |m, value| {
            m.properties.house.ownership.open = value;
            if !value {
                m.properties.house.ownership.form_review = None;
                m.properties.house.ownership.model.cancel_review();
            }
        },
        submit: |m, commands, action, values| {
            use feature_0049_house::input::ownership as input;
            if action > 11 {
                return;
            }
            match action {
                0..=2 => input::page(
                    m,
                    commands,
                    match action {
                        1 => -1,
                        2 => 1,
                        _ => 0,
                    },
                ),
                3 => {
                    let result = values
                        .first()
                        .and_then(|v| v.parse::<usize>().ok())
                        .and_then(|i| i.checked_sub(1))
                        .ok_or("Select a house")
                        .and_then(|i| m.properties.house.ownership.select(i));
                    if let Err(error) = result {
                        m.properties.house.ownership.model.feedback = error.into();
                    }
                }
                4..=9 => {
                    let fields =
                        std::array::from_fn(|i| values.get(i + 1).cloned().unwrap_or_default());
                    match input::review(m, (action - 3) as u8, fields) {
                        Ok(review) => m.properties.house.ownership.form_review = Some(review),
                        Err(error) => m.properties.house.ownership.model.feedback = error,
                    }
                }
                _ => {
                    if let Some(review) = m.properties.house.ownership.form_review.take() {
                        input::confirm(m, commands, review, action == 10);
                    }
                }
            }
        },
    }
}
pub(super) fn house_map() -> Form {
    Form {
        title: "House entrances",
        labels: &[],
        buttons: &["Refresh"],
        snapshot: |m| {
            let p = &m.properties.house;
            let text = crate::play::feature_0049_house::state::map_projection(m)
                .iter()
                .map(|h| {
                    format!(
                        "{} {}: {},{},{}",
                        if h.selected { "◆" } else { "⌂" },
                        h.label,
                        h.position.0,
                        h.position.1,
                        h.position.2
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            (p.map_open, false, vec![], text)
        },
        set_open: |m, value| m.properties.house.map_open = value,
        submit: |m, commands, _, _| feature_0049_house::input::submit(m, commands, 0),
    }
}
#[cfg(any(test, feature = "bevy-shell"))]
pub(super) fn drawing(
    title: &str,
    m: &Mailbox,
) -> Option<oteryn_client_ui::cyclopedia_map::atlas_view::Drawing> {
    use oteryn_client_ui::cyclopedia_map::atlas_view::Drawing;
    if title == "World atlas" && m.properties.cyclopedia_map.geometry.show_map {
        Some(oteryn_client_ui::cyclopedia_map::atlas_view::project(
            &m.properties.cyclopedia_map,
        ))
    } else if title == "Map overview" {
        Some(overview_drawing(&m.properties.cyclopedia_map))
    } else if title == "House entrances" {
        Some(Drawing {
            caption: "House entrances — north up".into(),
            points: crate::play::feature_0049_house::state::map_projection(m)
                .iter()
                .map(|h| [h.pixel.0, h.pixel.1])
                .collect(),
            ..Default::default()
        })
    } else {
        None
    }
}

pub(super) fn overview() -> Form {
    Form {
        title: "Map overview",
        labels: &[],
        buttons: &[
            "Refresh",
            "Centre on player",
            "Floor up",
            "Floor down",
            "Zoom in",
            "Zoom out",
            "North",
            "South",
            "West",
            "East",
            "NPC layer",
            "Passage layer",
            "POI layer",
            "Region layer",
            "Previous marker page",
            "Next marker page",
            "Back to Cyclopedia",
        ],
        snapshot: |m| {
            let p = &m.properties.cyclopedia_map;
            let mut text = oteryn_client_ui::cyclopedia_map::atlas_detail(p);
            for marker in p.atlas.iter().flat_map(|a| &a.markers) {
                if p.atlas_layers & (1 << (marker.kind - 1)) != 0 {
                    text.push_str(&format!(
                        "\n{} {},{} — {}",
                        match marker.kind {
                            1 => "NPC",
                            2 => "Passage",
                            _ => "POI",
                        },
                        marker.x,
                        marker.y,
                        marker.label
                    ));
                }
            }
            (p.atlas_open, p.pending || p.speed_pending, vec![], text)
        },
        set_open: |m, value| m.properties.cyclopedia_map.atlas_open = value,
        submit: |m, commands, action, _| {
            let p = &mut m.properties.cyclopedia_map;
            if action == 16 {
                p.atlas_open = false;
                p.open = true;
            } else if oteryn_client_ui::cyclopedia_map::atlas_action(p, action) {
                feature_0057_cyclopedia_map::input::submit(
                    m,
                    commands,
                    oteryn_client_ui::cyclopedia_map::Intent::Atlas,
                );
            }
        },
    }
}
fn overview_drawing(
    p: &oteryn_client_ui::cyclopedia_map::Panel,
) -> oteryn_client_ui::cyclopedia_map::atlas_view::Drawing {
    use oteryn_client_ui::cyclopedia_map::{
        atlas_player_cell,
        atlas_view::{Drawing, Rect},
    };
    let mut drawing = Drawing {
        caption: "North up · white cross: you · gold: visible markers".into(),
        ..Default::default()
    };
    if let Some(atlas) = &p.atlas {
        if p.atlas_layers & 8 != 0 {
            for (i, id) in atlas.cells.iter().enumerate().filter(|(_, id)| **id != 0) {
                drawing.rects.push(Rect {
                    bounds: [(i % 64) as i32 * 8, (i / 64) as i32 * 8, 8, 8],
                    color: (id % 6) as u8,
                });
            }
        }
        let step = 1u32 << atlas.query.zoom;
        for marker in &atlas.markers {
            if p.atlas_layers & (1 << (marker.kind - 1)) == 0 {
                continue;
            }
            if let (Some(x), Some(y)) = (
                u32::from(marker.x).checked_sub(u32::from(atlas.query.x)),
                u32::from(marker.y).checked_sub(u32::from(atlas.query.y)),
            ) && x / step < 64
                && y / step < 40
            {
                drawing
                    .points
                    .push([(x / step) as i32 * 8 + 4, (y / step) as i32 * 8 + 4]);
            }
        }
        drawing.player = atlas_player_cell(p).map(|[x, y]| [x * 8 + 4, y * 8 + 4]);
    }
    drawing
}
