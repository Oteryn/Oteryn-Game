//! Projection of confirmed personal marks onto the current-floor native minimap.
pub use oteryn_client_runtime::personal_map_marks::{PersonalMapMark, PersonalMapMarks};
use oteryn_renderer::TileCoord;

/// Visible true locations; an off-window mark is never clamped onto a false tile.
pub fn project(
    marks: &PersonalMapMarks,
    own: TileCoord,
    anchor: TileCoord,
    floor: i16,
    zoom: i32,
) -> Vec<TileCoord> {
    if zoom <= 0 {
        return Vec::new();
    }
    marks
        .0
        .iter()
        .filter(|mark| i16::from(mark.floor) == floor)
        .filter_map(|mark| {
            let x = i32::from(mark.x)
                .checked_sub(anchor.x)?
                .checked_sub(own.x)?
                .div_euclid(zoom)
                .checked_add((crate::minimap::COLUMNS / 2) as i32)?;
            let y = i32::from(mark.y)
                .checked_sub(anchor.y)?
                .checked_sub(own.y)?
                .div_euclid(zoom)
                .checked_add((crate::minimap::ROWS / 2) as i32)?;
            (x >= 0
                && x < crate::minimap::COLUMNS as i32
                && y >= 0
                && y < crate::minimap::ROWS as i32)
                .then_some(TileCoord::new(x, y))
        })
        .collect()
}
/// One nearby label and its real coordinates remain useful while the mark is outside the window.
pub fn caption(
    marks: &PersonalMapMarks,
    own: TileCoord,
    anchor: TileCoord,
    floor: i16,
) -> Vec<String> {
    let x = i64::from(own.x) + i64::from(anchor.x);
    let y = i64::from(own.y) + i64::from(anchor.y);
    let Some(mark) = marks
        .0
        .iter()
        .filter(|mark| i16::from(mark.floor) == floor)
        .min_by_key(|mark| {
            let dx = i64::from(mark.x) - x;
            let dy = i64::from(mark.y) - y;
            i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)
        })
    else {
        return Vec::new();
    };
    let chars: Vec<_> = mark.text.chars().take(128).collect();
    let mut lines: Vec<String> = chars.chunks(20).map(|line| line.iter().collect()).collect();
    lines.push(format!(
        "{}, {} / {}",
        mark.x,
        mark.y,
        mark.floor.unsigned_abs()
    ));
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn personal_marks_use_true_world_coordinates_floor_and_anchor() {
        let mark = PersonalMapMark {
            key: "sterling".into(),
            x: 31948,
            y: 31924,
            floor: -7,
            text: "Treasure".into(),
        };
        let marks = PersonalMapMarks(vec![mark]);
        let own = TileCoord::new(20, 30);
        let anchor = TileCoord::new(31928, 31894);
        assert_eq!(
            project(&marks, own, anchor, -7, 1),
            vec![TileCoord::new(15, 12)]
        );
        assert!(project(&marks, own, anchor, -8, 1).is_empty());
        assert!(project(&marks, own, anchor, -7, 0).is_empty());
        assert_eq!(
            project(&marks, TileCoord::new(10, 10), anchor, -7, 2),
            vec![TileCoord::new(20, 22)]
        );
        assert!(project(&marks, TileCoord::new(0, 0), anchor, -7, 1).is_empty());
        assert!(project(&marks, TileCoord::new(i32::MIN, 0), anchor, -7, 1).is_empty());
        assert_eq!(
            caption(&marks, own, anchor, -7),
            vec!["Treasure", "31948, 31924 / 7"]
        );
        assert!(caption(&marks, own, anchor, -8).is_empty());
    }
}
