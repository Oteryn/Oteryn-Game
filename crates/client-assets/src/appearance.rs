use crate::catalog::Catalog;
use crate::error::AssetError;
use crate::sheets::{CELL_PX, SpriteSheets};
use crate::store::{AssetStore, MAX_APPEARANCES_BYTES};
use prost::Message;
use std::collections::HashMap;

/// Largest appearance id the loader accepts (the 15.30 maximum is 55,117).
pub const MAX_APPEARANCE_ID: u32 = 65_535;
/// Most 32-pixel cells one drawn entry may resolve to: the measured maximum over the pinned
/// 15.30 appearances (frame group 0, phase 0, all layers) is 16, the bound is at least 16.
pub const MAX_ENTRY_CELLS: usize = 16;

// Hand-written messages with the field numbers of Canary's `appearances.proto` at the 15.30
// pin. They carry only the fields this crate reads; unknown fields are skipped.
#[derive(Clone, PartialEq, Message)]
struct AppearancesMsg {
    #[prost(message, repeated, tag = "1")]
    object: Vec<AppearanceMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct AppearanceMsg {
    #[prost(uint32, optional, tag = "1")]
    id: Option<u32>,
    #[prost(message, repeated, tag = "2")]
    frame_group: Vec<FrameGroupMsg>,
    #[prost(message, optional, tag = "3")]
    flags: Option<FlagsMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct FrameGroupMsg {
    #[prost(message, optional, tag = "3")]
    sprite_info: Option<SpriteInfoMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct SpriteInfoMsg {
    #[prost(uint32, optional, tag = "1")]
    pattern_width: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    pattern_height: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pattern_depth: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    layers: Option<u32>,
    #[prost(uint32, repeated, packed = "false", tag = "5")]
    sprite_id: Vec<u32>,
    #[prost(message, optional, tag = "6")]
    animation: Option<AnimationMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct AnimationMsg {
    #[prost(message, repeated, tag = "6")]
    sprite_phase: Vec<PhaseMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct PhaseMsg {}

#[derive(Clone, PartialEq, Message)]
struct FlagsMsg {
    #[prost(message, optional, tag = "1")]
    bank: Option<BankMsg>,
    #[prost(bool, optional, tag = "2")]
    clip: Option<bool>,
    #[prost(bool, optional, tag = "3")]
    bottom: Option<bool>,
    #[prost(bool, optional, tag = "4")]
    top: Option<bool>,
    #[prost(bool, optional, tag = "6")]
    cumulative: Option<bool>,
    #[prost(bool, optional, tag = "12")]
    liquidpool: Option<bool>,
    #[prost(bool, optional, tag = "19")]
    liquidcontainer: Option<bool>,
    #[prost(message, optional, tag = "26")]
    shift: Option<ShiftMsg>,
    #[prost(message, optional, tag = "27")]
    height: Option<HeightMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct BankMsg {}

#[derive(Clone, PartialEq, Message)]
struct ShiftMsg {
    #[prost(uint32, optional, tag = "1")]
    x: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    y: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
struct HeightMsg {
    #[prost(uint32, optional, tag = "1")]
    elevation: Option<u32>,
}

/// Frame group 0 of one object appearance. Animation phases are kept; only phase 0 is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appearance {
    pub id: u32,
    pub pattern_width: u32,
    pub pattern_height: u32,
    pub pattern_depth: u32,
    pub layers: u32,
    pub phases: u32,
    /// Sprite ids ordered `[phase][depth][height][width][layer]`; 0 is an empty cell.
    pub sprite_ids: Vec<u32>,
    pub displacement_x: u32,
    pub displacement_y: u32,
    pub elevation: u32,
    pub ground: bool,
    pub ground_border: bool,
    pub on_bottom: bool,
    pub on_top: bool,
    pub stackable: bool,
    pub fluid: bool,
}

/// Where an entry sits and what it holds; these pick its pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// Stack count of a stackable item.
    pub count: u32,
    /// Fluid sub-type of a fluid container or pool.
    pub sub_type: u32,
    pub x: i32,
    pub y: i32,
    /// Native floor `-15..=0`.
    pub floor: i32,
}

impl Placement {
    #[must_use]
    pub const fn at(x: i32, y: i32, floor: i32) -> Self {
        Self {
            count: 1,
            sub_type: 0,
            x,
            y,
            floor,
        }
    }
}

/// One 32x32 cell to draw, with its pixel offset from the top-left of the tile. The offset
/// includes the sprite's anchoring at the tile's bottom-right and the displacement; elevation
/// is returned apart, because the caller accumulates it over a tile's stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCell {
    pub sprite_id: u32,
    pub cell_x: u32,
    pub cell_y: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntry {
    pub cells: Vec<DrawCell>,
    pub elevation: u32,
    pub ground: bool,
    pub ground_border: bool,
    pub on_bottom: bool,
    pub on_top: bool,
}

/// Object id to frame group 0, from the pinned `appearances` file.
#[derive(Debug, Clone)]
pub struct AppearanceIndex {
    objects: HashMap<u32, Appearance>,
}

impl AppearanceIndex {
    /// Loads the `appearances` file the catalogue names, verified against its manifest entry.
    pub fn load(store: &AssetStore, catalog: &Catalog) -> Result<Self, AssetError> {
        let file = catalog.appearances_file();
        let bytes = store.read_verified(file, MAX_APPEARANCES_BYTES)?;
        let message =
            AppearancesMsg::decode(bytes.as_slice()).map_err(|error| AssetError::Malformed {
                file: file.to_owned(),
                reason: error.to_string(),
            })?;
        let mut objects = HashMap::with_capacity(message.object.len());
        for object in message.object {
            let Some(id) = object.id.filter(|id| (1..=MAX_APPEARANCE_ID).contains(id)) else {
                return Err(AssetError::Malformed {
                    file: file.to_owned(),
                    reason: "object without a valid id".into(),
                });
            };
            if let Some(appearance) = convert(id, object) {
                objects.insert(id, appearance);
            }
        }
        Ok(Self { objects })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    #[must_use]
    pub fn max_id(&self) -> Option<u32> {
        self.objects.keys().copied().max()
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Appearance> {
        self.objects.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Appearance> {
        self.objects.values()
    }

    /// The cells to draw for one entry, chosen as the Tibia client does. `floor` is the native
    /// floor `-15..=0`; the depth pattern is selected from `z = -floor`, never from a negative
    /// index.
    pub fn resolve(
        &self,
        sheets: &SpriteSheets,
        appearance_id: u32,
        placement: Placement,
    ) -> Result<ResolvedEntry, AssetError> {
        let Placement {
            count,
            sub_type,
            x,
            y,
            floor,
        } = placement;
        if !(1..=MAX_APPEARANCE_ID).contains(&appearance_id) {
            return Err(AssetError::InvalidAppearanceId { id: appearance_id });
        }
        if !(-15..=0).contains(&floor) {
            return Err(AssetError::InvalidFloor { floor });
        }
        let appearance = self
            .objects
            .get(&appearance_id)
            .ok_or(AssetError::UnknownAppearance { id: appearance_id })?;
        let z = floor.unsigned_abs();
        let (pattern_x, pattern_y) = if appearance.fluid {
            (
                sub_type % 4 % appearance.pattern_width,
                sub_type / 4 % appearance.pattern_height,
            )
        } else if appearance.stackable {
            // Count thresholds 1, 2, 3, 4, 5, 10, 25, 50 select patterns 0..=7.
            let step = match count {
                0..=4 => count.saturating_sub(1),
                5..=9 => 4,
                10..=24 => 5,
                25..=49 => 6,
                _ => 7,
            };
            (
                step % appearance.pattern_width,
                step / appearance.pattern_width % appearance.pattern_height,
            )
        } else {
            (
                x.rem_euclid(appearance.pattern_width as i32) as u32,
                y.rem_euclid(appearance.pattern_height as i32) as u32,
            )
        };
        let pattern_z = z % appearance.pattern_depth;
        let mut cells = Vec::new();
        for layer in 0..appearance.layers {
            let index = ((pattern_z * appearance.pattern_height + pattern_y)
                * appearance.pattern_width
                + pattern_x)
                * appearance.layers
                + layer;
            // Phase 0 is the first block, so the index needs no phase term.
            let sprite_id = appearance
                .sprite_ids
                .get(index as usize)
                .copied()
                .ok_or(AssetError::UnknownSprite { sprite_id: 0 })?;
            if sprite_id == 0 {
                continue;
            }
            let (cells_x, cells_y) = sheets.layout(sprite_id)?.cells();
            for cell_y in 0..cells_y {
                for cell_x in 0..cells_x {
                    cells.push(DrawCell {
                        sprite_id,
                        cell_x,
                        cell_y,
                        offset_x: (cell_x as i32 + 1 - cells_x as i32) * CELL_PX as i32
                            - appearance.displacement_x as i32,
                        offset_y: (cell_y as i32 + 1 - cells_y as i32) * CELL_PX as i32
                            - appearance.displacement_y as i32,
                    });
                }
            }
        }
        if cells.len() > MAX_ENTRY_CELLS {
            return Err(AssetError::TooManyCells { cells: cells.len() });
        }
        Ok(ResolvedEntry {
            cells,
            elevation: appearance.elevation,
            ground: appearance.ground,
            ground_border: appearance.ground_border,
            on_bottom: appearance.on_bottom,
            on_top: appearance.on_top,
        })
    }
}

fn convert(id: u32, object: AppearanceMsg) -> Option<Appearance> {
    let info = object.frame_group.into_iter().next()?.sprite_info?;
    let flags = object.flags.unwrap_or_default();
    let phases = info
        .animation
        .map_or(1, |animation| animation.sprite_phase.len().max(1) as u32);
    let (width, height, depth, layers) = (
        info.pattern_width.unwrap_or(1).max(1),
        info.pattern_height.unwrap_or(1).max(1),
        info.pattern_depth.unwrap_or(1).max(1),
        info.layers.unwrap_or(1).max(1),
    );
    // The sprite list must hold every phase of the pattern grid; otherwise the entry is unusable.
    let expected = [phases, depth, height, width, layers]
        .iter()
        .try_fold(1_usize, |total, factor| total.checked_mul(*factor as usize))?;
    if info.sprite_id.len() != expected {
        return None;
    }
    let shift = flags.shift.unwrap_or_default();
    Some(Appearance {
        id,
        pattern_width: width,
        pattern_height: height,
        pattern_depth: depth,
        layers,
        phases,
        sprite_ids: info.sprite_id,
        displacement_x: shift.x.unwrap_or(0),
        displacement_y: shift.y.unwrap_or(0),
        elevation: flags.height.unwrap_or_default().elevation.unwrap_or(0),
        ground: flags.bank.is_some(),
        ground_border: flags.clip.unwrap_or(false),
        on_bottom: flags.bottom.unwrap_or(false),
        on_top: flags.top.unwrap_or(false),
        stackable: flags.cumulative.unwrap_or(false),
        fluid: flags.liquidcontainer.unwrap_or(false) || flags.liquidpool.unwrap_or(false),
    })
}
