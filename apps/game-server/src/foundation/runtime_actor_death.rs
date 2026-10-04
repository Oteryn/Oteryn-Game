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
    /// [`Self::respawn_position`] for a runtime respawn, or the recorded respawn position a fresh
    /// admission consumes, which the caller has checked is a Walkable cell of the pinned
    /// generation. A stale actor, another session or an unpositioned actor moves nothing.
    pub(crate) fn place_respawned_player(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.player_control_facts(actor, game_session_id)?;
        let current = self.carrier.read_position(&self.continuity, actor.0)?;
        self.carrier
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
            .map(MovementPositionSnapshot)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_player_death_tests.rs"]
mod channel_owner_player_death_tests;
