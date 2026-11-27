//! Publish a committed turn on the connection's existing spatial revision stream.
use super::*;

pub(super) struct Publication {
    frames: Vec<Vec<u8>>,
    pub(super) sequence: u64,
    revision: u64,
}

pub(super) fn prepare(
    session: &mut ServeState,
    turn: CommittedTurn,
) -> Result<Option<Publication>, ServeEnd> {
    let view = session
        .visibility
        .as_mut()
        .ok_or_else(|| ServeEnd::server("state_projection_unavailable"))?;
    let Some(mut delta) = view.publish_turn(turn) else {
        // Peers may already have projected the turn, or advanced past it.
        return if turn.actor == session.actor {
            Err(ServeEnd::server("state_projection_unavailable"))
        } else {
            Ok(None)
        };
    };
    if session
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1)
    {
        session
            .appearances
            .attach(&mut delta.update)
            .ok_or_else(|| ServeEnd::server("state_projection_unavailable"))?;
    }
    let (frames, sequence, revision) = visibility_frames(
        session.generation,
        session.sequence,
        session.revision,
        session.selected_capabilities.as_slice(),
        view.lights(),
        &VisibilityUpdate::Delta(delta),
    )
    .ok_or_else(|| ServeEnd::server("state_projection_unavailable"))?;
    Ok(Some(Publication {
        frames,
        sequence,
        revision,
    }))
}

impl Publication {
    pub(super) async fn write<S: AsyncWrite + Unpin>(
        self,
        stream: &mut S,
        session: &mut ServeState,
    ) -> Result<(), ServeEnd> {
        for frame in self.frames {
            write_frame(stream, &frame)
                .await
                .map_err(|error| ServeEnd::Transport(TransportEnd::Write(error.kind())))?;
        }
        session.sequence = self.sequence;
        session.revision = self.revision;
        session.admitted.continuity.server_sequence = self.sequence;
        session.admitted.continuity.spatial_revision = self.revision;
        Ok(())
    }
}
