use crate::error::AssetError;
use crate::store::{AssetStore, MAX_CATALOG_BYTES, is_plain_file_name};
use serde::Deserialize;

const CATALOG_FILE: &str = "catalog-content.json";

/// Sprite sheet cell layouts, from the catalogue `spritetype` 0-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteLayout {
    Size32x32,
    Size32x64,
    Size64x32,
    Size64x64,
}

impl SpriteLayout {
    /// Sprite size in pixels.
    #[must_use]
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Size32x32 => (32, 32),
            Self::Size32x64 => (32, 64),
            Self::Size64x32 => (64, 32),
            Self::Size64x64 => (64, 64),
        }
    }

    /// Sprite size in 32-pixel cells.
    #[must_use]
    pub const fn cells(self) -> (u32, u32) {
        let (width, height) = self.dimensions();
        (width / 32, height / 32)
    }

    fn from_spritetype(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Size32x32),
            1 => Some(Self::Size32x64),
            2 => Some(Self::Size64x32),
            3 => Some(Self::Size64x64),
            _ => None,
        }
    }
}

/// One catalogue sprite sheet: the sprite id range it holds and its layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetDescriptor {
    pub first_sprite_id: u32,
    pub last_sprite_id: u32,
    pub layout: SpriteLayout,
    pub file: String,
}

#[derive(Deserialize)]
struct RawEntry {
    #[serde(rename = "type")]
    kind: Option<String>,
    file: Option<String>,
    firstspriteid: Option<u32>,
    lastspriteid: Option<u32>,
    spritetype: Option<u32>,
}

/// The verified `catalog-content.json`: the appearances file name and the sprite sheets.
#[derive(Debug, Clone)]
pub struct Catalog {
    appearances_file: String,
    sheets: Vec<SpriteSheetDescriptor>,
}

impl Catalog {
    pub fn load(store: &AssetStore) -> Result<Self, AssetError> {
        let bytes = store.read_verified(CATALOG_FILE, MAX_CATALOG_BYTES)?;
        let malformed = |reason: String| AssetError::Malformed {
            file: CATALOG_FILE.to_owned(),
            reason,
        };
        let raw: Vec<RawEntry> =
            serde_json::from_slice(&bytes).map_err(|error| malformed(error.to_string()))?;
        let mut appearances_file = None;
        let mut sheets = Vec::new();
        for entry in raw {
            match entry.kind.as_deref() {
                Some("appearances") => {
                    let file = entry
                        .file
                        .filter(|file| is_plain_file_name(file))
                        .ok_or_else(|| malformed("appearances entry has no usable file".into()))?;
                    appearances_file = Some(file);
                }
                Some("sprite") => sheets.push(sheet_descriptor(entry, &malformed)?),
                _ => {}
            }
        }
        sheets.sort_by_key(|sheet| (sheet.first_sprite_id, sheet.last_sprite_id));
        for pair in sheets.windows(2) {
            if pair[1].first_sprite_id <= pair[0].last_sprite_id {
                return Err(malformed(format!(
                    "overlapping sprite ranges {}..={} and {}..={}",
                    pair[0].first_sprite_id,
                    pair[0].last_sprite_id,
                    pair[1].first_sprite_id,
                    pair[1].last_sprite_id
                )));
            }
        }
        Ok(Self {
            appearances_file: appearances_file
                .ok_or_else(|| malformed("no appearances entry".into()))?,
            sheets,
        })
    }

    /// File name of the `appearances` file the catalogue names.
    #[must_use]
    pub fn appearances_file(&self) -> &str {
        &self.appearances_file
    }

    #[must_use]
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// The sheet that holds `sprite_id`.
    #[must_use]
    pub fn locate(&self, sprite_id: u32) -> Option<&SpriteSheetDescriptor> {
        let upper = self
            .sheets
            .partition_point(|sheet| sheet.first_sprite_id <= sprite_id);
        let candidate = self.sheets.get(upper.checked_sub(1)?)?;
        (sprite_id <= candidate.last_sprite_id).then_some(candidate)
    }
}

fn sheet_descriptor(
    entry: RawEntry,
    malformed: &impl Fn(String) -> AssetError,
) -> Result<SpriteSheetDescriptor, AssetError> {
    let (Some(first), Some(last), Some(spritetype), Some(file)) = (
        entry.firstspriteid,
        entry.lastspriteid,
        entry.spritetype,
        entry.file,
    ) else {
        return Err(malformed("incomplete sprite entry".into()));
    };
    let layout = SpriteLayout::from_spritetype(spritetype)
        .ok_or_else(|| malformed(format!("unsupported spritetype {spritetype}")))?;
    if !is_plain_file_name(&file) || !file.ends_with(".bmp.lzma") {
        return Err(malformed(format!("unsafe sprite sheet name {file:?}")));
    }
    let (width, height) = layout.dimensions();
    let capacity = (384 / width) * (384 / height);
    if last < first || last - first + 1 > capacity {
        return Err(malformed(format!(
            "sprite sheet {file} range {first}..={last} does not fit {capacity} sprites"
        )));
    }
    Ok(SpriteSheetDescriptor {
        first_sprite_id: first,
        last_sprite_id: last,
        layout,
        file,
    })
}
