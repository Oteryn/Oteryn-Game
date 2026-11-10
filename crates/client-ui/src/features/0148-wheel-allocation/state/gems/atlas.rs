//! Original geometric glyphs. Quality changes cut; domain changes color; lock/vessel are overlays.
use super::*;
pub const SIZE: usize = 64;
pub fn glyph(quality: u8, domain: u8, locked: bool, vessel: bool) -> Vec<u8> {
    let mut rgba = vec![0; SIZE * SIZE * 4];
    let colors = [
        [63, 188, 110],
        [215, 81, 75],
        [164, 109, 221],
        [74, 150, 221],
    ];
    let color = colors
        .get(usize::from(domain))
        .copied()
        .unwrap_or([110, 110, 110]);
    let radius = 18 + 3 * i32::from(quality.min(2));
    for y in 0..64 {
        for x in 0..64 {
            let dx = (x - 32_i32).abs();
            let dy = (y - 32_i32).abs();
            let inside = dx + dy <= radius;
            let ring = vessel && (29..=31).contains(&(dx + dy));
            let lock = locked && ((44..59).contains(&x) && (45..59).contains(&y));
            let rgb = if lock {
                Some([245, 200, 65])
            } else if ring {
                Some([230, 230, 240])
            } else if inside {
                Some(if x < 32 { color } else { color.map(|c| c / 2) })
            } else {
                None
            };
            if let Some(rgb) = rgb {
                let i = (y as usize * SIZE + x as usize) * 4;
                rgba[i..i + 3].copy_from_slice(&rgb);
                rgba[i + 3] = 255;
            }
        }
    }
    rgba
}
fn empty_slot() -> Vec<u8> {
    let mut pixels = vec![0; SIZE * SIZE * 4];
    for y in 8..56 {
        for x in 8..56 {
            if x == 8 || x == 55 || y == 8 || y == 55 {
                let i = (y * SIZE + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[105, 105, 115, 255]);
            }
        }
    }
    pixels
}
pub fn panel_glyphs(p: &WheelPanel) -> Vec<u8> {
    let mut cells = Vec::new();
    for index in 0..5 {
        let id = if index == 0 {
            p.gem
        } else {
            p.vessels[index - 1]
        };
        let gem = id.and_then(|id| {
            p.confirmed_atelier
                .as_ref()?
                .gems
                .iter()
                .find(|g| g.id == id)
        });
        cells.push(gem.map_or_else(empty_slot, |g| {
            glyph(g.quality, g.domain, g.locked, g.vessel)
        }));
    }
    let mut pixels = vec![0; SIZE * SIZE * 5 * 4];
    for y in 0..SIZE {
        for (column, cell) in cells.iter().enumerate() {
            let source = &cell[y * SIZE * 4..(y + 1) * SIZE * 4];
            let start = (y * SIZE * 5 + column * SIZE) * 4;
            pixels[start..start + SIZE * 4].copy_from_slice(source);
        }
    }
    pixels
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gem_shapes_distinguish_quality_domain_lock_and_vessel() -> Result<(), &'static str> {
        let base = glyph(0, 0, false, false);
        assert!(base != glyph(2, 0, false, false), "quality changes the cut");
        assert!(
            base != glyph(0, 1, false, false),
            "domain changes the color"
        );
        let lock = glyph(0, 0, true, false);
        assert_eq!(
            &lock[(50 * 64 + 50) * 4..(50 * 64 + 50) * 4 + 3],
            &[245, 200, 65]
        );
        let vessel = glyph(0, 0, false, true);
        assert_eq!(
            &vessel[(32 * 64 + 2) * 4..(32 * 64 + 2) * 4 + 3],
            &[230, 230, 240]
        );
        Ok(())
    }
}
