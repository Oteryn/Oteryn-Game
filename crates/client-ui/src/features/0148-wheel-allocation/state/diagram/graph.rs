//! Original6x6 layout; every edge comes exclusively from server-provided unlock adjacency.
use super::*;
use std::collections::BTreeSet;
pub const SIZE: usize = 384;
pub fn center(index: usize) -> Option<(i32, i32)> {
    if index >= 36 {
        return None;
    }
    Some((32 + (index % 6) as i32 * 64, 32 + (index / 6) as i32 * 64))
}
pub fn hit(x: i32, y: i32) -> Option<usize> {
    if !(0..SIZE as i32).contains(&x) || !(0..SIZE as i32).contains(&y) {
        return None;
    }
    (0..36)
        .find(|i| center(*i).is_some_and(|(a, b)| (x - a) * (x - a) + (y - b) * (y - b) <= 20 * 20))
}
pub fn edges(nodes: &[wire::Node]) -> Option<Vec<(usize, usize)>> {
    if nodes.len() != 36 {
        return None;
    }
    let mut edges = BTreeSet::new();
    for (index, n) in nodes.iter().enumerate() {
        for other in &n.unlock_from {
            let other = usize::from(*other);
            if other >= 36 || index == other {
                return None;
            }
            edges.insert((index.min(other), index.max(other)));
        }
    }
    Some(edges.into_iter().collect())
}
fn pixel(p: &mut [u8], x: i32, y: i32, c: [u8; 4]) {
    if (0..SIZE as i32).contains(&x) && (0..SIZE as i32).contains(&y) {
        let i = (y as usize * SIZE + x as usize) * 4;
        p[i..i + 4].copy_from_slice(&c);
    }
}
fn line(p: &mut [u8], (mut x, mut y): (i32, i32), (b, c): (i32, i32)) {
    let dx = (b - x).abs();
    let sx = if x < b { 1 } else { -1 };
    let dy = -(c - y).abs();
    let sy = if y < c { 1 } else { -1 };
    let mut e = dx + dy;
    loop {
        pixel(p, x, y, [85, 89, 103, 255]);
        if x == b && y == c {
            break;
        }
        let twice = 2 * e;
        if twice >= dy {
            e += dy;
            x += sx;
        }
        if twice <= dx {
            e += dx;
            y += sy;
        }
    }
}
pub fn rgba(p: &WheelPanel) -> Option<Vec<u8>> {
    let r = metadata(p)?;
    let mut image = vec![0; SIZE * SIZE * 4];
    for pixel in image.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[15, 18, 27, 255]);
    }
    for (a, b) in edges(&r.nodes)? {
        line(&mut image, center(a)?, center(b)?);
    }
    let colors = [
        [57, 176, 101, 255],
        [208, 75, 67, 255],
        [158, 103, 212, 255],
        [64, 144, 212, 255],
    ];
    for (index, n) in r.nodes.iter().enumerate() {
        let (x, y) = center(index)?;
        let color = *colors.get(usize::from(n.domain))?;
        let fill = i32::from(p.points[index].min(n.capacity)) * 40 / i32::from(n.capacity);
        for dy in -22..=22 {
            for dx in -22..=22 {
                let radius = dx * dx + dy * dy;
                if radius <= 400 {
                    let c = if dy >= 20 - fill {
                        color
                    } else {
                        color.map(|v| v / 3)
                    };
                    pixel(&mut image, x + dx, y + dy, [c[0], c[1], c[2], 255]);
                } else if index == p.slot && radius <= 484 {
                    pixel(&mut image, x + dx, y + dy, [245, 224, 134, 255]);
                }
            }
        }
    }
    Some(image)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheel_graph_hit_testing_selects_exactly_thirty_six_slots() -> Result<(), &'static str> {
        for index in 0..36 {
            let (x, y) = center(index).ok_or("slot")?;
            assert_eq!(hit(x, y), Some(index));
        }
        assert_eq!(hit(0, 0), None);
        assert_eq!(center(36), None);
        assert_eq!(center(usize::MAX), None);
        assert_eq!(hit(i32::MIN, i32::MIN), None);
        Ok(())
    }
}
