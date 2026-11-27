//! Domain-17 projection of actual House items through the existing map owner.
use super::*;

impl State {
    pub(super) async fn refresh_map<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        let (Some(map), Some(items)) = (self.world_map.as_mut(), self.items.as_mut()) else {
            return Ok(());
        };
        let update = authority
            .observe_house_map(*admitted, map, items)
            .await
            .ok_or_else(|| ServeEnd::server("serve_state_unavailable"))?
            .map_err(|_| ServeEnd::server("serve_state_unavailable"))?;
        if let Some(update) = update {
            match update {
                MapUpdate::Delta(delta) => {
                    let (sequence, frame) = item_view_delta(
                        admitted.continuity.connection_generation,
                        admitted.continuity.server_sequence,
                        &delta,
                    )
                    .ok_or_else(|| ServeEnd::server("serve_state_unavailable"))?;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::server("frame_write_refused"))?;
                    admitted.continuity.server_sequence = sequence;
                }
                MapUpdate::Snapshot(snapshot) => {
                    let sequence = admitted
                        .continuity
                        .server_sequence
                        .checked_add(1)
                        .ok_or_else(|| ServeEnd::server("sequence_exhausted"))?;
                    let frames = encode_single_chunk_snapshot(
                        admitted.continuity.connection_generation,
                        sequence,
                        sequence,
                        &[DomainSnapshot {
                            domain_id: snapshot.domain_id,
                            revision: snapshot.revision,
                            snapshot_type: snapshot.snapshot_type,
                            payload: &snapshot.payload,
                        }],
                    )
                    .map_err(|_| ServeEnd::server("serve_state_unavailable"))?;
                    for frame in frames {
                        write_frame(stream, &frame)
                            .await
                            .map_err(|_| ServeEnd::server("frame_write_refused"))?;
                    }
                    admitted.continuity.server_sequence = sequence;
                }
            }
        }
        admitted.continuity.world_map_revision = map.revision();
        admitted.continuity.item_view = items.continuity();
        Ok(())
    }
}
