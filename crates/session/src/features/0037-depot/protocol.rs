//! Live container sources required by the native depot browser.
use crate::*;

#[cfg(test)]
fn validate_join(selected: &[u32], snapshots: &[GatedSnapshot]) -> Result<(), SessionError> {
    container_tree::initial(selected, snapshots).map(|_| ())
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
                    .container_views()
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
    fn negotiated_house_container_is_kept_in_typed_depot_source_snapshot()
    -> Result<(), Box<dyn std::error::Error>> {
        let views = container_tree::ContainerViews {
            views: vec![container_tree::ContainerView {
                view_id: 0,
                container_handle: ItemHandle::MIN,
                parent_handle: None,
                capacity: 25,
                entries: vec![],
                browse: None,
            }],
        };
        let payload =
            container_tree::encode_container_views_with_house_capacity(&views, false, true)
                .map_err(|error| format!("{error:?}"))?;
        let snapshot = GatedSnapshot {
            domain_id: 14,
            snapshot_type: 1,
            revision: 1,
            payload,
        };
        let accepted = container_tree::initial(&[14, 446], std::slice::from_ref(&snapshot))
            .map_err(|error| format!("{error:?}"))?
            .ok_or("selected container domain remains present")?;
        assert_eq!(accepted, views);
        assert!(accepted.views.iter().any(|view| view.view_id == 0));
        assert!(container_tree::initial(&[14], &[snapshot]).is_err());
        Ok(())
    }

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
