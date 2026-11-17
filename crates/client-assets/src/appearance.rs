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

/// The largest displacement (in pixels, per axis) an appearance may carry; an entry beyond it is
/// unusable, so draw-offset arithmetic stays in range.
pub const MAX_DISPLACEMENT_PX: u32 = 1024;

// Hand-written messages with the field numbers of Canary's `appearances.proto` at the 15.30
// pin. They carry only the fields this crate reads; unknown fields are skipped.
#[derive(Clone, PartialEq, Message)]
struct AppearancesMsg {
    #[prost(message, repeated, tag = "1")]
    object: Vec<AppearanceMsg>,
    #[prost(message, repeated, tag = "2")]
    outfit: Vec<AppearanceMsg>,
    #[prost(message, repeated, tag = "3")]
    effect: Vec<AppearanceMsg>,
    #[prost(message, repeated, tag = "4")]
    missile: Vec<AppearanceMsg>,
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
struct PhaseMsg {
    #[prost(uint32, optional, tag = "1")]
    duration_min: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    duration_max: Option<u32>,
}

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
    #[prost(bool, optional, tag = "20")]
    hang: Option<bool>,
    #[prost(message, optional, tag = "21")]
    hook: Option<HookMsg>,
    #[prost(message, optional, tag = "26")]
    shift: Option<ShiftMsg>,
    #[prost(message, optional, tag = "27")]
    height: Option<HeightMsg>,
}

#[derive(Clone, PartialEq, Message)]
struct BankMsg {}

#[derive(Clone, PartialEq, Message)]
struct HookMsg {
    /// `HOOK_TYPE`: 1 south, 2 east.
    #[prost(int32, optional, tag = "1")]
    direction: Option<i32>,
}

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

/// Frame group 0 of an appearance. Visual effects resolve all animation phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appearance {
    pub id: u32,
    pub pattern_width: u32,
    pub pattern_height: u32,
    pub pattern_depth: u32,
    pub layers: u32,
    pub phases: u32,
    /// Validated mean duration per phase, milliseconds.
    pub phase_ms: Vec<u32>,
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
    /// Hangs on a wall hook; its pattern follows the hook of the tile it is on.
    pub hangable: bool,
    /// The hook this entry gives the tile it is on.
    pub hook: Hook,
}

/// The wall hook of a tile, which picks the pattern of a hangable entry on it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Hook {
    #[default]
    None,
    South,
    East,
}

/// The pattern colour of each fluid sub-type in the modern client; unknown sub-types are empty.
const FLUID_COLORS: [u32; 21] = [
    0, 1, 7, 3, 3, 2, 4, 3, 5, 6, 7, 2, 5, 3, 5, 6, 3, 3, 8, 10, 9,
];

fn fluid_color(sub_type: u32) -> u32 {
    usize::try_from(sub_type)
        .ok()
        .and_then(|index| FLUID_COLORS.get(index))
        .copied()
        .unwrap_or(0)
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
    /// The hook given by the entries of the same tile.
    pub hook: Hook,
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
            hook: Hook::None,
        }
    }
}

/// Compatibility export for resolved source-cell coordinates.
pub use oteryn_client_domain::world::DrawCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntry {
    pub cells: Vec<DrawCell>,
    pub elevation: u32,
    pub ground: bool,
    pub ground_border: bool,
    pub on_bottom: bool,
    pub on_top: bool,
}

/// Object and outfit id to frame group 0, from the pinned `appearances` file.
#[derive(Debug, Clone)]
pub struct AppearanceIndex {
    objects: HashMap<u32, Appearance>,
    outfits: HashMap<u32, Appearance>,
    effects: HashMap<u32, Appearance>,
    missiles: HashMap<u32, Appearance>,
}

include!("appearance_methods_1.rs");

/// The `(x, y, z)` pattern of `appearance` at `placement`, chosen as the Tibia client does. The
/// depth pattern is selected from `z = -floor`, never from a negative index.
fn pattern(appearance: &Appearance, placement: Placement) -> (u32, u32, u32) {
    let Placement {
        count,
        sub_type,
        x,
        y,
        floor,
        hook,
    } = placement;
    let z = floor.unsigned_abs();
    let (pattern_x, pattern_y) = if appearance.fluid {
        let color = fluid_color(sub_type);
        (
            color % 4 % appearance.pattern_width,
            color / 4 % appearance.pattern_height,
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
    } else if appearance.hangable {
        // Pattern 1 hangs on a south hook and 2 on an east hook; 0 lies on the floor.
        let hooked = match hook {
            Hook::South if appearance.pattern_width >= 2 => 1,
            Hook::East if appearance.pattern_width >= 3 => 2,
            _ => 0,
        };
        return (hooked, 0, 0);
    } else {
        (
            x.rem_euclid(appearance.pattern_width as i32) as u32,
            y.rem_euclid(appearance.pattern_height as i32) as u32,
        )
    };
    (pattern_x, pattern_y, z % appearance.pattern_depth)
}

include!("appearance_entry.rs");

fn convert(id: u32, object: AppearanceMsg) -> Option<Appearance> {
    let info = object.frame_group.into_iter().next()?.sprite_info?;
    let flags = object.flags.unwrap_or_default();
    let phase_ms: Vec<u32> = info.animation.map_or_else(Vec::new, |animation| {
        animation
            .sprite_phase
            .into_iter()
            .map(|phase| {
                let low = phase.duration_min.unwrap_or(100);
                let high = phase.duration_max.unwrap_or(low).max(low);
                (low / 2 + high / 2).clamp(1, 10_000)
            })
            .collect()
    });
    if phase_ms.len() > 256 {
        return None;
    }
    let phases = phase_ms.len().max(1) as u32;
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
    let (displacement_x, displacement_y) = (shift.x.unwrap_or(0), shift.y.unwrap_or(0));
    if displacement_x.max(displacement_y) > MAX_DISPLACEMENT_PX {
        return None;
    }
    Some(Appearance {
        id,
        pattern_width: width,
        pattern_height: height,
        pattern_depth: depth,
        layers,
        phases,
        phase_ms,
        sprite_ids: info.sprite_id,
        displacement_x,
        displacement_y,
        elevation: flags.height.unwrap_or_default().elevation.unwrap_or(0),
        ground: flags.bank.is_some(),
        ground_border: flags.clip.unwrap_or(false),
        on_bottom: flags.bottom.unwrap_or(false),
        on_top: flags.top.unwrap_or(false),
        stackable: flags.cumulative.unwrap_or(false),
        fluid: flags.liquidcontainer.unwrap_or(false) || flags.liquidpool.unwrap_or(false),
        hangable: flags.hang.unwrap_or(false),
        hook: match flags.hook.and_then(|hook| hook.direction) {
            Some(1) => Hook::South,
            Some(2) => Hook::East,
            _ => Hook::None,
        },
    })
}

include!("appearance_tests_1.rs");

include!("visual_tests.rs");

include!("visual.rs");
