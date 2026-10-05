//! Exact cached Canary47df/Crystal00ce AreaCombat kernels; offsets only, no world authority.
use crate::content::ProjectV2AbilityArea;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Facing {
    North,
    East,
    South,
    West,
    NorthWest,
    NorthEast,
    SouthEast,
    SouthWest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeometryError {
    Bounds,
    InvalidMatrix,
    MissingDiagonal,
}
const RADIUS: [[u8; 13]; 13] = [
    [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0],
    [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0],
    [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0],
    [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0],
    [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0],
    [8, 7, 6, 5, 4, 2, 1, 2, 4, 5, 6, 7, 8],
    [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0],
    [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0],
    [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0],
    [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0],
    [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0],
];
pub(crate) fn offsets(
    area: &ProjectV2AbilityArea,
    facing: Facing,
) -> Result<Vec<(i32, i32)>, GeometryError> {
    let (rows, diagonal) = match area {
        ProjectV2AbilityArea::Circle { radius_tiles } => {
            return Ok(RADIUS
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter().enumerate().filter_map(move |(x, v)| {
                        (*v == 1 || *v > 0 && u16::from(*v) <= *radius_tiles)
                            .then_some((x as i32 - 6, y as i32 - 6))
                    })
                })
                .collect());
        }
        ProjectV2AbilityArea::Beam {
            length_tiles,
            spread_tiles,
        } => {
            let l = usize::from(*length_tiles);
            let s = usize::from(*spread_tiles);
            if l == 0 || l > 64 || s > 64 {
                return Err(GeometryError::Bounds);
            }
            let cols = if s == 0 { 1 } else { ((l - l % s) / s) * 2 + 1 };
            if cols > 129 {
                return Err(GeometryError::Bounds);
            }
            let mut width = cols;
            let mut rows = Vec::new();
            for y in 1..=l {
                let mut row = String::new();
                for x in 1..=cols {
                    row.push(if y == l && x == cols / 2 + 1 {
                        'C'
                    } else if x > cols - width && x <= width {
                        'x'
                    } else {
                        '.'
                    });
                }
                rows.push(row);
                if s > 0 && y % s == 0 {
                    width = width.saturating_sub(1);
                }
            }
            (rows, false)
        }
        ProjectV2AbilityArea::Matrix { north, diagonal } => {
            let diag = matches!(
                facing,
                Facing::NorthWest | Facing::NorthEast | Facing::SouthEast | Facing::SouthWest
            );
            if diag && diagonal.is_empty() {
                return Err(GeometryError::MissingDiagonal);
            }
            (
                if diag {
                    diagonal.clone()
                } else {
                    north.clone()
                },
                diag,
            )
        }
    };
    if rows.is_empty()
        || rows.len() > 129
        || rows[0].is_empty()
        || rows[0].len() > 129
        || rows
            .iter()
            .any(|r| r.len() != rows[0].len() || !r.is_ascii())
    {
        return Err(GeometryError::InvalidMatrix);
    }
    let mut center = None;
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            match c {
                b'c' | b'C' => {
                    if center.replace((x as i32, y as i32)).is_some() {
                        return Err(GeometryError::InvalidMatrix);
                    }
                }
                b'x' | b'.' => {}
                _ => return Err(GeometryError::InvalidMatrix),
            }
        }
    }
    let (cx, cy) = center.ok_or(GeometryError::InvalidMatrix)?;
    let mut result = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            if c != b'x' && c != b'C' {
                continue;
            }
            let (x, y) = (x as i32 - cx, y as i32 - cy);
            let p = if diagonal {
                match facing {
                    Facing::NorthWest => (x, y),
                    Facing::NorthEast => (-x, y),
                    Facing::SouthWest => (x, -y),
                    Facing::SouthEast => (-x, -y),
                    _ => unreachable!(),
                }
            } else {
                match facing {
                    Facing::North => (x, y),
                    Facing::East => (-y, x),
                    Facing::South => (-x, -y),
                    Facing::West => (y, -x),
                    _ => return Err(GeometryError::MissingDiagonal),
                }
            };
            result.push(p);
        }
    }
    Ok(result)
}
pub(crate) fn direction(dx: i64, dy: i64, diagonal: bool) -> Facing {
    if diagonal && dx != 0 && dy != 0 {
        match (dx < 0, dy < 0) {
            (true, true) => Facing::NorthWest,
            (false, true) => Facing::NorthEast,
            (true, false) => Facing::SouthWest,
            (false, false) => Facing::SouthEast,
        }
    } else if dx < 0 {
        Facing::West
    } else if dx > 0 {
        Facing::East
    } else if dy < 0 {
        Facing::North
    } else {
        Facing::South
    }
}
pub(crate) fn step(f: Facing) -> (i32, i32) {
    match f {
        Facing::North => (0, -1),
        Facing::East => (1, 0),
        Facing::South => (0, 1),
        Facing::West => (-1, 0),
        Facing::NorthWest => (-1, -1),
        Facing::NorthEast => (1, -1),
        Facing::SouthEast => (1, 1),
        Facing::SouthWest => (-1, 1),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_numeric_circle_not_chebyshev() {
        let zero = offsets(
            &ProjectV2AbilityArea::Circle { radius_tiles: 0 },
            Facing::North,
        )
        .unwrap();
        assert_eq!(zero, vec![(0, 0)]);
        let one = offsets(
            &ProjectV2AbilityArea::Circle { radius_tiles: 1 },
            Facing::North,
        )
        .unwrap();
        assert_eq!(one, vec![(0, 0)]);
        let two = offsets(
            &ProjectV2AbilityArea::Circle { radius_tiles: 2 },
            Facing::North,
        )
        .unwrap();
        assert_eq!(two.len(), 5);
        assert!(!two.contains(&(1, 1)));
        assert_eq!(
            offsets(
                &ProjectV2AbilityArea::Circle { radius_tiles: 99 },
                Facing::East
            )
            .unwrap()
            .len(),
            101
        );
    }
    #[test]
    fn beam_rotation_and_spread() {
        let a = ProjectV2AbilityArea::Beam {
            length_tiles: 3,
            spread_tiles: 0,
        };
        assert_eq!(
            offsets(&a, Facing::North).unwrap(),
            vec![(0, -2), (0, -1), (0, 0)]
        );
        assert_eq!(
            offsets(&a, Facing::East).unwrap(),
            vec![(2, 0), (1, 0), (0, 0)]
        );
        assert_eq!(
            offsets(
                &ProjectV2AbilityArea::Beam {
                    length_tiles: 3,
                    spread_tiles: 1
                },
                Facing::North
            )
            .unwrap()
            .len(),
            15
        );
    }
    #[test]
    fn matrix_center_exclusion_and_diagonal_refusal() {
        let a = ProjectV2AbilityArea::Matrix {
            north: vec![".x.".into(), "xcx".into()],
            diagonal: vec![],
        };
        assert_eq!(
            offsets(&a, Facing::North).unwrap(),
            vec![(0, -1), (-1, 0), (1, 0)]
        );
        assert_eq!(
            offsets(&a, Facing::NorthWest),
            Err(GeometryError::MissingDiagonal)
        );
        assert_eq!(
            offsets(
                &ProjectV2AbilityArea::Matrix {
                    north: vec!["CC".into()],
                    diagonal: vec![]
                },
                Facing::North
            ),
            Err(GeometryError::InvalidMatrix)
        );
    }
}
