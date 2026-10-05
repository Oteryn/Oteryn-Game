//! DEATH-2 (`reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md` §4.5): the
//! Channel owner's placement of a respawning player.
//!
//! Until a temple and home-town decision exists, a player respawns at its admitted entry spawn:
//! the pinned Content generation's first-entry start cell (control-plane ruling, reversible). The
//! actor keeps its reference: the respawn is one compare-committed position successor in the
//! actor's own position context, so the connection that serves it stays bound to it.

use super::*;

impl ChannelRuntimeV1 {
    /// The respawn position: the pinned generation's first-entry start cell.
    pub(crate) const fn respawn_position(&self) -> MovementLocalPosition {
        let start = self.content.entry_start;
        MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor,
        }
    }

    /// The map revision the durable death cell records: the pinned generation's digest.
    pub(crate) const fn map_revision_digest(&self) -> [u8; 32] {
        self.content.map_revision_digest
    }

    /// Place the committed player of `game_session_id` at `position` as one position successor:
    /// [`Self::respawn_position`] for a runtime respawn, or the cell
    /// [`Self::place_admitted_respawn`] selects for a fresh admission. A stale actor, another
    /// session or an unpositioned actor moves nothing.
    pub(crate) fn place_respawned_player(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.player_control_facts(actor, game_session_id)?;
        let current = self.carrier.read_position(&self.continuity, actor.0)?;
        let committed = self
            .carrier
            .compare_commit_position(
                &self.continuity,
                current,
                current.version.context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)?;
        self.clear_respawn_player_conditions(actor, game_session_id);
        Ok(committed)
    }

    /// DEATH-2b and CHAR-POSITION-0 §3.3: place an admitted player for its pending respawn.
    /// The recorded cell is used if it is `admissible` and free. Otherwise the NPC-0 §6.1 fallback
    /// applies: the nearest admissible free cell of the same floor within Chebyshev distance
    /// [`RESPAWN_FALLBACK_RADIUS`], in spiral order (north first, clockwise), else step 4, the
    /// respawn position. An undecodable record goes straight to step 4. A free cell is one no
    /// other live actor occupies.
    pub(crate) fn place_admitted_respawn(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        recorded: Option<MovementLocalPosition>,
        admissible: impl Fn(MovementLocalPosition) -> bool,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.player_control_facts(actor, game_session_id)?;
        let current = self.carrier.read_position(&self.continuity, actor.0)?;
        let free = |cell: MovementLocalPosition| {
            let cell = LocalPosition {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            };
            !occupied_by_another(&self.carrier, actor.0, current.version.context, cell)
        };
        let target = recorded
            .and_then(|recorded| {
                respawn_fallback_cells(recorded).find(|cell| admissible(*cell) && free(*cell))
            })
            .unwrap_or_else(|| self.respawn_position());
        self.place_respawned_player(actor, game_session_id, target)
    }
}

/// True when a live actor other than `actor` occupies `cell` under `context`, under the same
/// rule as `ChannelActorCarrier::cell_occupied`: a committed player, or a creature with health
/// left. Only `actor` itself is excluded, so another actor on its own current cell still counts.
fn occupied_by_another(
    carrier: &ChannelActorCarrier,
    actor: ActorRef,
    context: PreProductionPositionContext,
    cell: LocalPosition,
) -> bool {
    carrier.slots.iter().any(|slot| match slot {
        Slot::Occupied {
            committed: true,
            position: Some(version),
            ..
        }
        | Slot::CreatureOccupied {
            position: Some(version),
            health: 1..,
            ..
        } => {
            version.context == context
                && version.position == cell
                && (version.actor_local_id, version.actor_local_generation)
                    != (actor.actor_local_id, actor.actor_local_generation)
        }
        _ => false,
    })
}

/// NPC-0 §6.1: the placement fallback searches within this Chebyshev distance.
const RESPAWN_FALLBACK_RADIUS: i32 = 3;

/// `recorded`, then each ring of the same floor out to [`RESPAWN_FALLBACK_RADIUS`], each ring
/// from its north cell clockwise (north is `y - 1`, east is `x + 1`).
fn respawn_fallback_cells(
    recorded: MovementLocalPosition,
) -> impl Iterator<Item = MovementLocalPosition> {
    let ring = |radius: i32| {
        // The north edge from the middle eastward, then the east, south and west edges, then
        // the north edge from its west corner back to the middle.
        let north_east = (0..radius).map(move |dx| (dx, -radius));
        let east = (-radius..radius).map(move |dy| (radius, dy));
        let south = (-radius + 1..=radius).rev().map(move |dx| (dx, radius));
        let west = (-radius + 1..=radius).rev().map(move |dy| (-radius, dy));
        let north_west = (-radius..0).map(move |dx| (dx, -radius));
        north_east
            .chain(east)
            .chain(south)
            .chain(west)
            .chain(north_west)
    };
    std::iter::once((0, 0))
        .chain((1..=RESPAWN_FALLBACK_RADIUS).flat_map(ring))
        .filter_map(move |(dx, dy)| {
            Some(MovementLocalPosition {
                x: recorded.x.checked_add(dx)?,
                y: recorded.y.checked_add(dy)?,
                floor: recorded.floor,
            })
        })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_player_death_tests.rs"]
mod channel_owner_player_death_tests;
