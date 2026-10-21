/// A native tile position: `floor` is `-15..=0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TilePos {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}
