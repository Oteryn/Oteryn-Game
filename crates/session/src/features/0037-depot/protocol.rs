//! Live container sources required by the native depot browser.
use crate::*;

pub(crate) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&14) {
        return Ok(());
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
    container_tree::decode_container_views_for(&snapshot.payload, selected.contains(&230))
        .map_err(SessionError::ItemView)?;
    Ok(())
}

pub(crate) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(14) {
        return Err(SessionError::UnselectedDomain { domain_id: 14 });
    }
    let delta = decode_state_delta(payload)?;
    let snapshot = session
        .gated_snapshots
        .iter_mut()
        .find(|s| s.domain_id == 14)
        .ok_or(SessionError::MissingDomain(14))?;
    check_delta(&delta, 1, snapshot.revision)?;
    let value = container_tree::decode_container_views_for(
        delta.payload,
        session.selected_capabilities.contains(&230),
    )
    .map_err(SessionError::ItemView)?;
    snapshot.revision = delta.new_revision;
    snapshot.payload = delta.payload.to_vec();
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
    /// Open a depot source bag and wait within this session’s configured deadline.
    /// A missing view update is stale UI state, so play remains usable.
    pub async fn open_depot_container(
        &mut self,
        handle: ItemHandle,
    ) -> Result<Option<container_tree::ContainerViewOutcome>, SessionError> {
        use container_tree::ContainerViewOutcome;
        if !self.is_selected(14) {
            return Ok(None);
        }
        let payload = container_tree::encode_container_view_intent(
            &container_tree::ContainerViewIntent::Open {
                handle,
                replace_view: self
                    .gated_snapshot(14)
                    .and_then(|s| {
                        container_tree::decode_container_views_for(
                            &s.payload,
                            self.is_selected(230),
                        )
                        .ok()
                    })
                    .filter(|views| views.views.iter().any(|v| v.view_id == 0))
                    .map(|_| 0),
            },
        )
        .map_err(|_| SessionError::SessionUnusable)?;
        let revision = self.gated_snapshot(14).map(|s| s.revision);
        let result = self.gated_command(14, 21, &payload).await?;
        if result.status != CommandStatus::Accepted {
            return Ok(None);
        }
        let outcome = container_tree::decode_container_view_result(&result.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if outcome == ContainerViewOutcome::Opened {
            let until = tokio::time::Instant::now() + self.deadline;
            while self.gated_snapshot(14).map(|s| s.revision) == revision {
                if tokio::time::Instant::now() >= until {
                    return Ok(Some(ContainerViewOutcome::Stale));
                }
                self.service_liveness(
                    until
                        .saturating_duration_since(tokio::time::Instant::now())
                        .min(Duration::from_millis(20)),
                )
                .await?;
            }
        }
        Ok(Some(outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_container_snapshot_is_required_typed_and_versioned() {
        assert!(validate_join(&[], &[]).is_ok());
        assert!(validate_join(&[14], &[]).is_err());
        let mut snapshot = GatedSnapshot {
            domain_id: 14,
            snapshot_type: 1,
            revision: 1,
            payload: vec![],
        };
        assert!(validate_join(&[14], &[snapshot.clone()]).is_ok());
        snapshot.payload = vec![0xff];
        assert!(validate_join(&[14], &[snapshot.clone()]).is_err());
        snapshot.payload.clear();
        snapshot.snapshot_type = 2;
        assert!(validate_join(&[14], &[snapshot]).is_err());
    }
}
