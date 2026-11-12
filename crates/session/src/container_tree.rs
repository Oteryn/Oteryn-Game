//! Typed transport for existing capability 14. Depot browsing is a separate server task.
use super::*;
use oteryn_protocol_oteryn::container_tree as wire;
pub use oteryn_protocol_oteryn::container_tree::*;

#[cfg(test)]
#[path = "features/0014-container-tree/tests/e2e_i01.rs"]
mod e2e_i01;

pub(super) fn initial(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<Option<ContainerViews>, SessionError> {
    if !selected.contains(&14) {
        return Ok(None);
    }
    let snapshot = snapshots
        .iter()
        .find(|s| s.domain_id == 14)
        .ok_or(SessionError::MissingDomain(14))?;
    if snapshot.snapshot_type != 1 {
        return Err(SessionError::UnregisteredSnapshotType {
            domain_id: 14,
            snapshot_type: snapshot.snapshot_type,
        });
    }
    wire::decode_container_views_with_house_capacity(
        &snapshot.payload,
        selected.contains(&230),
        selected.contains(&446),
    )
    .map(Some)
    .map_err(SessionError::ItemView)
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    delta: oteryn_protocol_oteryn::StateDeltaView<'_>,
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(14) {
        return Err(SessionError::UnselectedDomain { domain_id: 14 });
    }
    let revision = session
        .gated_revisions
        .iter()
        .find(|(id, _)| *id == 14)
        .map(|(_, r)| *r)
        .ok_or(SessionError::MissingDomain(14))?;
    check_delta(&delta, 1, revision)?;
    let value = wire::decode_container_views_with_house_capacity(
        delta.payload,
        session.is_selected(230),
        session.is_selected(446),
    )
    .map_err(SessionError::ItemView)?;
    session.container_views = Some(value.clone());
    if let Some(snapshot) = session
        .gated_snapshots
        .iter_mut()
        .find(|s| s.domain_id == 14)
    {
        snapshot.revision = delta.new_revision;
        snapshot.payload = delta.payload.to_vec();
    }
    if let Some((_, revision)) = session.gated_revisions.iter_mut().find(|(id, _)| *id == 14) {
        *revision = delta.new_revision;
    }
    Ok(SessionEvent::ContainerViews(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

impl<S: SessionStream> Session<S> {
    pub fn container_views(&self) -> Option<&ContainerViews> {
        self.container_views.as_ref()
    }

    /// Returns at the command result; contents arrive only through sequenced domain-14 updates.
    pub async fn container_view(
        &mut self,
        intent: ContainerViewIntent,
    ) -> Result<Option<ContainerViewOutcome>, SessionError> {
        let payload =
            wire::encode_container_view_intent(&intent).map_err(SessionError::ItemView)?;
        let result = self.gated_command(14, 21, &payload).await?;
        if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            return Ok(None);
        }
        if result.status != CommandStatus::Accepted {
            return self.poison_on_error(Err(SessionError::InconsistentCommandResult {
                command_id: result.command_id,
                status: result.status,
            }));
        }
        let outcome =
            wire::decode_container_view_result(&result.payload).map_err(SessionError::ItemView);
        self.poison_on_error(outcome).map(Some)
    }
}
