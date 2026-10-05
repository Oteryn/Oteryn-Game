use std::fmt::{self, Display, Formatter};

/// Why an asset could not be loaded or resolved. Always local to one file or one lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    /// The file could not be read.
    Io { file: String, message: String },
    /// The manifest has no entry for the file name.
    NotInManifest { file: String },
    /// The file bytes do not match the manifest `sha256`.
    HashMismatch { file: String },
    /// The file is larger than its bound.
    TooLarge { file: String, limit: u64 },
    /// The file name could escape the asset directory.
    UnsafeFileName { file: String },
    /// The content does not parse or is out of its declared bounds.
    Malformed { file: String, reason: String },
    /// The appearance id is outside `1..=MAX_APPEARANCE_ID`.
    InvalidAppearanceId { id: u32 },
    /// The id is in range but the pinned appearances do not contain it.
    UnknownAppearance { id: u32 },
    /// The native floor is outside `-15..=0`.
    InvalidFloor { floor: i32 },
    /// The sprite id is absent from the sprite catalogue.
    UnknownSprite { sprite_id: u32 },
    /// The entry would draw more than `MAX_ENTRY_CELLS` cells.
    TooManyCells { cells: usize },
}

impl Display for AssetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { file, message } => write!(formatter, "read {file}: {message}"),
            Self::NotInManifest { file } => write!(formatter, "{file} has no manifest entry"),
            Self::HashMismatch { file } => {
                write!(formatter, "{file} does not match its manifest sha256")
            }
            Self::TooLarge { file, limit } => {
                write!(formatter, "{file} exceeds its bound of {limit} bytes")
            }
            Self::UnsafeFileName { file } => write!(formatter, "unsafe file name {file:?}"),
            Self::Malformed { file, reason } => write!(formatter, "{file} is malformed: {reason}"),
            Self::InvalidAppearanceId { id } => {
                write!(formatter, "appearance id {id} is outside the valid range")
            }
            Self::UnknownAppearance { id } => write!(formatter, "unknown appearance id {id}"),
            Self::InvalidFloor { floor } => {
                write!(formatter, "floor {floor} is outside -15..=0")
            }
            Self::UnknownSprite { sprite_id } => {
                write!(formatter, "sprite id {sprite_id} is not in the catalogue")
            }
            Self::TooManyCells { cells } => {
                write!(formatter, "entry resolves to {cells} cells, over the bound")
            }
        }
    }
}

impl std::error::Error for AssetError {}
