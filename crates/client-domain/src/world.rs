//! World cell values, independent of asset formats, protocol and rendering backends.

/// Edge of one drawn source cell in pixels.
pub const CELL_PX: u32 = 32;

/// One atlas cell drawn at a pixel offset from a tile's top-left, in 32 px source pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    pub cell: u16,
    pub offset: [i32; 2],
}

/// What one map tile draws: `under` before a creature standing on it, `over` after it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapTile {
    pub under: Vec<Draw>,
    /// How far a creature on this tile is lifted, in source pixels.
    pub elevation: i32,
    pub over: Vec<Draw>,
}

/// The tutorial islet temple, where the offline view starts.
pub const START: (i32, i32) = (32534, 32513);
pub const START_FLOOR: u8 = 7;
