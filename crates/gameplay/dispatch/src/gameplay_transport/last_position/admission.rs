/// Whether `placed` records exactly `cell`: the in-session respawn upserts only the cell it
/// places the actor at.
pub(super) fn respawn_cell_recorded(
    cell: MovementLocalPosition,
    placed: &FinalLastPosition,
) -> bool {
    placed.record.cell == cell_of(cell)
}

/// §3.3 "valid", the parts the stored row decides: the same World and the same coordinate
/// frame. The map and artifact digests are diagnostics only: a cell of another map revision is
/// re-checked against the active cells, never trusted or skipped.
fn stored_position_applies(
    stored: &StoredLastPosition,
    world_id: WorldId,
    coordinate_frame_digest: [u8; 32],
) -> bool {
    stored.record.world_id == world_id
        && stored.record.coordinate_frame_digest == coordinate_frame_digest
}

pub use oteryn_gameplay_housing::last_position::house_cell as house_cell_rule;

/// §3.3 "valid": a house tile is invalid until HOUSE-OWN-0 decides entry. A bundle World reads
/// its map facts; any other World reads its qualified house placements, and a cell whose
/// placement is unknown counts as a house tile (fail closed).
fn house_cell(
    movement_cells: &crate::content::NativeEntryMovementCells,
    cell: MovementLocalPosition,
) -> bool {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::view::MapFacts;
    let logical = crate::content::LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    if let NativeMovementCollisionIndex::Bundle(index) = movement_cells.index() {
        // The runtime floor is legacy `z` in 0..=15; the native floor is `-z`.
        let tile = i8::try_from(cell.floor)
            .ok()
            .filter(|z| (0..=15).contains(z))
            .and_then(|z| {
                Some(crate::map::overlay::TilePos {
                    x: u16::try_from(cell.x).ok()?,
                    y: u16::try_from(cell.y).ok()?,
                    floor: -z,
                })
            });
        return house_cell_rule(tile.map(|tile| index.map().facts.house_tile(tile)));
    }
    house_cell_rule(
        movement_cells
            .house_tiles()
            .house_at(movement_cells.scope(), logical)
            .map(|house| house.is_some())
            .ok(),
    )
}

/// §3.3: what the durable rows decide for a new runtime actor, read under its session fence
/// before the first-entry Channel-owner step ([`ComposedFreshAdmission::read_admission_position`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdmissionPosition {
    /// Step 1: a pending respawn. The first-entry step places the actor at its cell; the
    /// respawn is consumed after the progression step.
    Respawn(ConsumedRespawn),
    Arrival(
        crate::durability::npc_travel::PendingArrival,
        Option<LastVitals>,
        Option<[u32; 4]>,
    ),
    /// Step 3: the stored last position, placed only if it is valid (else step 4).
    Stored(StoredLastPosition),
    /// No row: a first login, step 4.
    FirstLogin,
    /// P20: a stored row or a first login moved to this temple cell because the account's
    /// Premium ended in the premium area; the row's vitals and rest of this World still apply.
    /// The actual placed cell is persisted with citizenship before admission completes.
    Relocated(MovementLocalPosition, Option<LastVitals>, Option<[u32; 4]>),
}

impl AdmissionPosition {
    /// P20 overrides pending premium-area respawns and arrivals too; their obligations are
    /// consumed with the actual placement and citizenship after the first-entry owner step.
    pub(super) fn relocated(&self, cell: MovementLocalPosition, world_id: WorldId) -> Self {
        match self {
            Self::Stored(row) if row.record.world_id == world_id => {
                Self::Relocated(cell, row.record.vitals, row.record.rest)
            }
            Self::Stored(_) | Self::FirstLogin | Self::Respawn(_) => {
                Self::Relocated(cell, None, None)
            }
            Self::Arrival(_, vitals, rest) | Self::Relocated(_, vitals, rest) => {
                Self::Relocated(cell, *vitals, *rest)
            }
        }
    }
}

/// §3.3: the input that replaces `entry_start` in the first-entry owner step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstEntryInput {
    /// Step 4: the entry start itself.
    EntryStart,
    /// Step 1 or 3: this cell, else the NPC-0 §6.1 fallback, else step 4. `None` (an
    /// undecodable respawn record) goes straight to step 4.
    Recorded(Option<MovementLocalPosition>),
}

/// CHAR-VITALS-1: the (health, mana) a newly placed actor takes from the stored row that decided
/// its first entry. A respawn (the respawned actor is at its maxima, DEATH-2 §4.5), a first
/// login, a row without vitals and a row of another World start at the maxima (SPELL-D2). The
/// vitals are the Character's, so in its World they apply even when the stored cell does not
/// (another coordinate frame, a house tile).
pub(super) fn admitted_vitals(
    decided: &AdmissionPosition,
    world_id: WorldId,
) -> Option<(u32, u32)> {
    match decided {
        AdmissionPosition::Stored(stored) if stored.record.world_id == world_id => stored
            .record
            .vitals
            .map(|vitals| (vitals.health(), vitals.mana())),
        AdmissionPosition::Arrival(_, vitals, _) | AdmissionPosition::Relocated(_, vitals, _) => {
            vitals.map(|vitals| (vitals.health(), vitals.mana()))
        }
        AdmissionPosition::Stored(_)
        | AdmissionPosition::Respawn(_)
        | AdmissionPosition::FirstLogin => None,
    }
}

/// §3.3: the first-entry input `decided` selects. A pending respawn's recorded cell; a stored
/// cell only when it is of `world_id` and `frame_digest` and not a `house` tile (a saved house
/// tile goes straight to step 4, never through the radius fallback); else the entry start.
fn first_entry_input(
    decided: &AdmissionPosition,
    world_id: WorldId,
    frame_digest: [u8; 32],
    house: impl Fn(MovementLocalPosition) -> bool,
) -> FirstEntryInput {
    match decided {
        AdmissionPosition::Respawn(pending) => {
            FirstEntryInput::Recorded(respawn_cell(&pending.respawn_position))
        }
        AdmissionPosition::Arrival(pending, _, _) => {
            FirstEntryInput::Recorded(Some(position_of(pending.destination)))
        }
        AdmissionPosition::Relocated(cell, _, _) => FirstEntryInput::Recorded(Some(*cell)),
        AdmissionPosition::Stored(stored)
            if stored_position_applies(stored, world_id, frame_digest)
                && !house(position_of(stored.record.cell)) =>
        {
            FirstEntryInput::Recorded(Some(position_of(stored.record.cell)))
        }
        AdmissionPosition::Stored(_) | AdmissionPosition::FirstLogin => FirstEntryInput::EntryStart,
    }
}

/// §3.3 "it replaces `entry_start` as the input to `initialize_position`", in one Channel-owner
/// step: the actor's position is initialized and, before the caller releases the lock and any
/// owner reads it, moved to the cell `input` selects (`admissible` and free, or by the NPC-0 §6.1
/// fallback, else `entry_start`). An initialization that had already completed (`Reconciled`)
/// moves nothing. An error leaves the actor not playable.
pub(super) fn initialize_first_entry_at(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    input: FirstEntryInput,
    admissible: impl Fn(MovementLocalPosition) -> bool,
) -> Result<FirstEntryPosition, CarrierError> {
    match (runtime.initialize_first_entry_position(actor)?, input) {
        (FirstEntryPosition::Initialized(_), FirstEntryInput::Recorded(cell)) => runtime
            .place_admitted_respawn(actor, session, cell, admissible)
            .map(FirstEntryPosition::Initialized),
        (initialized, _) => Ok(initialized),
    }
}
