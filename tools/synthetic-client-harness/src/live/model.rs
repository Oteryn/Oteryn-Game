//! Pure state and geometry mapping for the live harness: join snapshot / command outcome ->
//! [`RenderModel`], and pixel click -> [`LiveCommand`]. No I/O, no GPU, no window.

use oteryn_dev_client::{CastOutcome, JoinSnapshot, SessionEvent, StepOutcome, UseOutcome};
use oteryn_foundation::ProcessGeneration;
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectOverlayEntry};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_renderer::{RendererError, SurfaceDecision, SurfaceEvent, SurfaceState};
use std::num::NonZeroU32;

/// The native entry room's one door placement (accepted content `accepted::DOOR_CELL`).
pub const DOOR_PLACEMENT: &[u8] = b"oteryn:cell/entry-door";
/// Overlay state key of a closed door.
pub const DOOR_STATE_CLOSED: &[u8] = b"oteryn:reference.state.closed";
/// Overlay state key of an open door.
pub const DOOR_STATE_OPEN: &[u8] = b"oteryn:reference.state.open";
/// The door's cell. The wire carries the placement key only, so the tile is a fixed fact of the
/// entry room (`accepted::DOOR_CELL` = x 1, y -1, floor 0).
pub const DOOR_TILE: Tile = Tile {
    x: 1,
    y: -1,
    floor: 0,
};
/// Edge of one square tile in surface pixels.
pub const TILE_PIXELS: u32 = 32;

/// A world cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// Door state as the overlay reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorState {
    Closed,
    Open,
    /// A state key this harness does not know.
    Unrecognised,
}

/// The door as drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorView {
    pub tile: Tile,
    pub state: DoorState,
}

/// What the last join or command reported, for the status line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    Joined,
    Moved,
    Blocked,
    StepRejected,
    DoorCommitted,
    DoorNothingToUse,
    DoorOccupied,
    DoorStale,
    DoorTooFar,
    DoorRejected,
    SpellCast(SpellCastDisposition),
}

impl Notice {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Joined => "joined",
            Self::Moved => "moved",
            Self::Blocked => "step blocked",
            Self::StepRejected => "step rejected",
            Self::DoorCommitted => "door used",
            Self::DoorNothingToUse => "nothing to use there",
            Self::DoorOccupied => "door occupied",
            Self::DoorStale => "door state was stale",
            Self::DoorTooFar => "too far from the door",
            Self::DoorRejected => "use rejected",
            Self::SpellCast(disposition) => match disposition {
                SpellCastDisposition::Cast => "spell cast",
                SpellCastDisposition::CoolingDown => "spell cooling down",
                SpellCastDisposition::LevelTooLow => "spell level too low",
                SpellCastDisposition::MagicLevelTooLow => "spell magic level too low",
                SpellCastDisposition::NotEnoughMana => "spell not enough mana",
                SpellCastDisposition::NotEnoughSoul => "spell not enough soul",
                SpellCastDisposition::NotAvailable => "spell not available",
                SpellCastDisposition::TargetRequired => "spell target required",
                SpellCastDisposition::TargetIllegal => "spell target illegal",
                SpellCastDisposition::Rejected => "spell rejected",
            },
        }
    }
}

/// Everything the live view draws, plus the overlay revision a door click must name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderModel {
    pub actor: Tile,
    pub door: Option<DoorView>,
    /// The `WORLD_OBJECT_OVERLAY` domain revision last applied: `use_object`'s
    /// `expected_revision`.
    pub overlay_revision: u64,
    /// Own-actor vitals, once the server has sent them (join snapshot or a pushed delta).
    pub vitals: Option<ActorVitals>,
    pub notice: Notice,
}

/// What one input asks the session to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveCommand {
    Step(StepDirection),
    UseDoor {
        expected_revision: u64,
    },
    Cast {
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    },
}

#[must_use]
pub fn door_state_from_key(key: &[u8]) -> DoorState {
    if key == DOOR_STATE_CLOSED {
        DoorState::Closed
    } else if key == DOOR_STATE_OPEN {
        DoorState::Open
    } else {
        DoorState::Unrecognised
    }
}

fn door_view(entry: &WorldObjectOverlayEntry) -> Option<DoorView> {
    (entry.placement == DOOR_PLACEMENT).then(|| DoorView {
        tile: DOOR_TILE,
        state: door_state_from_key(&entry.state),
    })
}

impl RenderModel {
    /// Records the actual cast disposition and the cast's own vitals delta. The cast wire carries
    /// own-actor vitals, not visual effects or target damage; those must not be inferred by the
    /// harness.
    #[must_use]
    pub fn apply_cast(&self, outcome: &CastOutcome) -> Self {
        let mut next = self.clone();
        next.notice = Notice::SpellCast(outcome.disposition);
        if let Some(delta) = &outcome.actor_vitals_delta {
            next.vitals = Some(delta.value);
        }
        next
    }

    /// The model right after the join.
    #[must_use]
    pub fn from_snapshot(snapshot: &JoinSnapshot) -> Self {
        let position = snapshot.world_spatial.actor_position;
        Self {
            actor: Tile {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            door: snapshot.world_object_overlay.iter().find_map(door_view),
            overlay_revision: snapshot.world_object_overlay_revision,
            vitals: None,
            notice: Notice::Joined,
        }
    }

    /// The model after a `step` outcome: a `Moved` delta relocates the actor.
    #[must_use]
    pub fn apply_step(&self, outcome: &StepOutcome) -> Self {
        let mut next = self.clone();
        if let Some(delta) = &outcome.world_spatial_delta {
            let position = delta.value.actor_position;
            next.actor = Tile {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
        }
        next.notice = match outcome.disposition {
            StepDisposition::Moved => Notice::Moved,
            StepDisposition::Blocked => Notice::Blocked,
            // SPEED-1: a paced refusal; the harness selects no capability 13, so it never decodes.
            StepDisposition::Rejected | StepDisposition::TooEarly => Notice::StepRejected,
        };
        next
    }

    /// The model after a `use_object` outcome. A use returns at its result and names no domain,
    /// so only the notice changes here; the door state arrives through [`Self::apply_events`].
    #[must_use]
    pub fn apply_use(&self, outcome: &UseOutcome) -> Self {
        let mut next = self.clone();
        next.notice = match outcome.disposition {
            UseDisposition::Committed => Notice::DoorCommitted,
            UseDisposition::NothingToUse => Notice::DoorNothingToUse,
            UseDisposition::Occupied => Notice::DoorOccupied,
            UseDisposition::StaleState => Notice::DoorStale,
            UseDisposition::TooFar => Notice::DoorTooFar,
            UseDisposition::Rejected => Notice::DoorRejected,
        };
        next
    }
}

impl RenderModel {
    /// The model after the pushed deltas the session applied, in order (the notice is kept).
    #[must_use]
    pub fn apply_events(&self, events: &[SessionEvent]) -> Self {
        let mut next = self.clone();
        for event in events {
            match event {
                SessionEvent::WorldSpatial(delta) => {
                    let position = delta.value.actor_position;
                    next.actor = Tile {
                        x: position.x,
                        y: position.y,
                        floor: position.floor,
                    };
                }
                SessionEvent::WorldObjectOverlay(delta) => {
                    if let Some(door) = door_view(&delta.value) {
                        next.door = Some(door);
                    }
                    next.overlay_revision = delta.new_revision;
                }
                SessionEvent::ActorVitals(delta) => next.vitals = Some(delta.value),
            }
        }
        next
    }
}

/// The tile grid a configured surface shows, centred on the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub cols: u32,
    pub rows: u32,
}

impl Viewport {
    /// Configures the harness renderer's surface state for `width` x `height` and derives the
    /// tile grid from the size it decided on.
    ///
    /// # Errors
    ///
    /// Returns the renderer's error for a zero-sized or otherwise refused surface.
    pub fn configure(width: u32, height: u32) -> Result<Self, RendererError> {
        let generation = ProcessGeneration::new(1);
        let mut surface = SurfaceState::new(generation);
        let decision = surface.apply(SurfaceEvent::Resize {
            generation,
            width,
            height,
        })?;
        Ok(match decision {
            SurfaceDecision::Configure(size) => Self::for_size(size.width(), size.height()),
            _ => Self { cols: 0, rows: 0 },
        })
    }

    #[must_use]
    pub const fn for_size(width: u32, height: u32) -> Self {
        Self {
            cols: width / TILE_PIXELS,
            rows: height / TILE_PIXELS,
        }
    }

    const fn centre(self) -> (i64, i64) {
        (self.cols as i64 / 2, self.rows as i64 / 2)
    }
}

/// The world tile under surface pixel (`px`, `py`), or `None` outside the grid.
#[must_use]
pub fn tile_at_pixel(view: Viewport, actor: Tile, px: i32, py: i32) -> Option<Tile> {
    if px < 0 || py < 0 {
        return None;
    }
    let col = i64::from(px) / i64::from(TILE_PIXELS);
    let row = i64::from(py) / i64::from(TILE_PIXELS);
    if col >= i64::from(view.cols) || row >= i64::from(view.rows) {
        return None;
    }
    let (cx, cy) = view.centre();
    Some(Tile {
        x: i32::try_from(i64::from(actor.x) + col - cx).ok()?,
        y: i32::try_from(i64::from(actor.y) + row - cy).ok()?,
        floor: actor.floor,
    })
}

/// The pixel at the centre of world `tile`, or `None` when it is off the grid or on another floor.
#[must_use]
pub fn tile_centre_pixel(view: Viewport, actor: Tile, tile: Tile) -> Option<(i32, i32)> {
    if tile.floor != actor.floor {
        return None;
    }
    let (cx, cy) = view.centre();
    let col = i64::from(tile.x) - i64::from(actor.x) + cx;
    let row = i64::from(tile.y) - i64::from(actor.y) + cy;
    if col < 0 || row < 0 || col >= i64::from(view.cols) || row >= i64::from(view.rows) {
        return None;
    }
    let half = i64::from(TILE_PIXELS / 2);
    Some((
        i32::try_from(col * i64::from(TILE_PIXELS) + half).ok()?,
        i32::try_from(row * i64::from(TILE_PIXELS) + half).ok()?,
    ))
}

/// A click on the door tile is `USE` of the door under the mirror's current overlay revision;
/// a click anywhere else does nothing.
#[must_use]
pub fn command_for_click(
    view: Viewport,
    model: &RenderModel,
    px: i32,
    py: i32,
) -> Option<LiveCommand> {
    let door = model.door?;
    (tile_at_pixel(view, model.actor, px, py)? == door.tile).then_some(LiveCommand::UseDoor {
        expected_revision: model.overlay_revision,
    })
}

/// Semantic movement action id -> wire step direction.
#[must_use]
pub fn step_direction_for_action(action: &str) -> Option<StepDirection> {
    match action {
        "move.north" => Some(StepDirection::North),
        "move.east" => Some(StepDirection::East),
        "move.south" => Some(StepDirection::South),
        "move.west" => Some(StepDirection::West),
        _ => None,
    }
}

/// One frame of the model as text: `@` actor, `+` closed door, `/` open door, `?` door in an
/// unrecognised state, `.` ground; north is up. Followed by a one-line status.
#[must_use]
pub fn render_text(view: Viewport, model: &RenderModel) -> String {
    let (cx, cy) = view.centre();
    let mut out = String::new();
    for row in 0..i64::from(view.rows) {
        for col in 0..i64::from(view.cols) {
            let x = i64::from(model.actor.x) + col - cx;
            let y = i64::from(model.actor.y) + row - cy;
            let glyph = if (col, row) == (cx, cy) {
                '@'
            } else {
                match model.door {
                    Some(door)
                        if door.tile.floor == model.actor.floor
                            && i64::from(door.tile.x) == x
                            && i64::from(door.tile.y) == y =>
                    {
                        match door.state {
                            DoorState::Closed => '+',
                            DoorState::Open => '/',
                            DoorState::Unrecognised => '?',
                        }
                    }
                    _ => '.',
                }
            };
            out.push(glyph);
        }
        out.push('\n');
    }
    let vitals = model.vitals.map_or_else(String::new, |vitals| {
        format!(
            " | hp {}/{} mp {}/{}",
            vitals.health, vitals.max_health, vitals.mana, vitals.max_mana
        )
    });
    out.push_str(&format!(
        "actor ({}, {}, {}){vitals} | {}\n",
        model.actor.x,
        model.actor.y,
        model.actor.floor,
        model.notice.as_str()
    ));
    out
}
