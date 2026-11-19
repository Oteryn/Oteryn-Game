enum PeriodicCapture {
    Ready(Box<Capture>),
    Retained(RetainedPosition),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// §3.2 periodic writes (`CHARPOS0-RL-01`): sleep until the earliest deadline (at most
    /// [`WRITER_MAX_SLEEP`], at least [`WRITER_MIN_PAUSE`]), then write every due session. It
    /// never ends on its own and parks once `shutdown` is requested.
    pub(super) async fn run_last_position_writes(
        &self,
        shutdown: &oteryn_foundation::CancellationToken,
    ) {
        loop {
            let now = Instant::now();
            let wake = self
                .states
                .last_positions
                .next_due()
                .map_or(now + WRITER_MAX_SLEEP, |due| {
                    due.min(now + WRITER_MAX_SLEEP)
                })
                .max(now + WRITER_MIN_PAUSE);
            if super::first(
                tokio::time::sleep_until(tokio::time::Instant::from_std(wake)),
                shutdown.cancelled(),
            )
            .await
            .is_none()
            {
                std::future::pending::<()>().await;
            }
            self.write_due_last_positions(false).await;
        }
    }
    /// One pass of §3.2 writes: the due sessions, most overdue first, or with `all` the
    /// shutdown write of every tracked session, in batches of [`LAST_POSITION_WRITE_BATCH`]. A
    /// cell and vitals unchanged since their last write write nothing; a dead actor is skipped.
    /// Returns the number of failed writes, which stay due.
    pub(super) async fn write_due_last_positions(&self, all: bool) -> usize {
        let due = self.states.last_positions.due(Instant::now(), all);
        let mut failed = 0;
        let batches: Vec<_> = due.chunks(LAST_POSITION_WRITE_BATCH).collect();
        let start = self.background_start("last-positions", batches.len());
        let count = batches.len();
        for index in (start..count).chain(0..start) {
            failed += self
                .background_turn(
                    "last-positions",
                    index + 1,
                    self.write_last_position_batch(batches[index]),
                )
                .await;
        }
        failed
    }

    /// One batch reads the candidate sessions' fences in one root pass and writes in one
    /// transaction (`CHARPOS0-RL-01`). Retained captures use the fence read before capture.
    /// The write transaction reads the retained vitals under that same fence.
    async fn write_last_position_batch(&self, batch: &[(GameSessionId, Track)]) -> usize {
        self.root
            .with_background_operation(self.write_last_position_batch_inner(batch))
            .await
    }

    async fn write_last_position_batch_inner(&self, batch: &[(GameSessionId, Track)]) -> usize {
        let mut changed = Vec::with_capacity(batch.len());
        let mut failed = 0;
        for &(session, track) in batch {
            match self.capture_last_position(&track, session).await {
                Captured::At(capture) => {
                    if capture.at.saturating_duration_since(track.checked) > LAST_POSITION_WRITE_INTERVAL {
                        operator_event("last_position_interval_exceeded");
                    }
                    if track.written == Some(Saved::of(&capture.record)) {
                        self.states.last_positions.settled(session, track.actor, None, capture.at);
                    } else {
                        changed.push((session, track, PeriodicCapture::Ready(capture)));
                    }
                }
                Captured::Vocation(retained) => {
                    changed.push((session, track, PeriodicCapture::Retained(retained)));
                }
                Captured::Held => {}
                // The placement of its respawn writes the respawn cell.
                Captured::Dead => {}
                Captured::Gone => self.states.last_positions.forget(session),
            }
        }
        if changed.is_empty() {
            return failed;
        }
        let current = match FreshAdmissionStore::from_root(self.root.clone())
            .current_sessions_at(changed.iter().map(|(session, _, _)| *session).collect())
            .await
        {
            Ok(current) => current,
            Err(error) => {
                operator_event(&format!("last_position_write_failed reason={error}"));
                return failed + changed.len();
            }
        };
        // `current` has one entry per changed session, in order; a missing one stays due.
        failed += changed.len().saturating_sub(current.len());
        let mut writes = Vec::with_capacity(changed.len());
        let mut fenced = Vec::with_capacity(changed.len());
        for ((session, track, capture), current) in changed.into_iter().zip(current) {
            let Some(current) = current else {
                failed += 1;
                continue;
            };
            // The fence `current_quest_fence` builds, with the admission revision (not read).
            if current.session_state() == crate::foundation::GameSessionState::Terminal {
                self.states.last_positions.forget(session);
                continue;
            }
            let Ok(character_id) =
                crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
            else {
                failed += 1;
                continue;
            };
            let capture = match capture {
                PeriodicCapture::Ready(capture) => capture,
                PeriodicCapture::Retained(retained) => {
                    let Captured::At(capture) = self
                        .capture_retained_tutorial_position(&track, session, character_id, retained)
                        .await
                    else {
                        failed += 1;
                        continue;
                    };
                    if capture.at.saturating_duration_since(track.checked) > LAST_POSITION_WRITE_INTERVAL {
                        operator_event("last_position_interval_exceeded");
                    }
                    #[cfg(test)]
                    retained_capture_pause(RetainedCaptureBoundary::AfterCapture).await;
                    capture
                }
            };
            writes.push(ScopedPeriodicLastPosition {
                house: capture.house.clone(),
                preserve_vitals: capture.preserve_vitals,
                write: PeriodicLastPosition {
                    fence: CurrentCharacterGameplayFence {
                        character_id,
                        game_session_id: session,
                        connection_generation: current.current_connection_generation(),
                        character_lease_generation: current.current_character_lease().generation(),
                        runtime_scope: current.current_runtime_scope(),
                        scope_ownership_generation: current.current_scope_generation(),
                        expected_character_revision: track.revision,
                    },
                    write_sequence: capture.write_sequence,
                    record: capture.record,
                },
            });
            fenced.push((session, track, capture));
        }
        if writes.is_empty() {
            return failed;
        }
        let outcomes = match self
            .root
            .write_scoped_character_last_positions(self.character, self.holder, writes)
            .await
        {
            Ok(outcomes) => outcomes,
            Err(error) => {
                operator_event(&format!("last_position_write_failed reason={error}"));
                return failed + fenced.len();
            }
        };
        failed += fenced.len().saturating_sub(outcomes.len());
        for ((session, track, capture), outcome) in fenced.into_iter().zip(outcomes) {
            match outcome {
                Ok(outcome) => {
                    let saved = match outcome {
                        LastPositionWrite::Written => Some(Saved::of(&capture.record)),
                        LastPositionWrite::RetainedVitals(vitals) => {
                            let mut saved = Saved::of(&capture.record);
                            saved.vitals = vitals;
                            Some(saved)
                        }
                        LastPositionWrite::Superseded | LastPositionWrite::PendingObligation => None,
                    };
                    self.states.last_positions.settled(session, track.actor, saved, capture.at);
                }
                Err(error) => {
                    failed += 1;
                    operator_event(&format!("last_position_write_failed reason={error}"));
                }
            }
        }
        failed
    }
}
