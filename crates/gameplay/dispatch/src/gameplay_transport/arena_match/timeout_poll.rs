//! M51 silent timer reset. Crystal201189f svargrond_arena.lua:259-313.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The poll gate precedes the spell lane; the writer owns the deciding clock.
    pub(super) async fn settle_arena_timeout(
        &self,
        session: GameSessionId,
        live: &LiveMatch,
    ) -> Option<()> {
        if Instant::now() < live.deadline {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().ok()?;
            if runtime
                .player_control_facts(live.actor, session)
                .ok()?
                .control_loss
                .is_some()
                || self
                    .states
                    .spell_states
                    .try_lock()
                    .ok()?
                    .is_dead(live.actor)
            {
                return None;
            }
        }
        if !matches!(
            self.drain_kill_rewards(session).await,
            super::super::kill_reward::DrainOutcome::Drained
        ) {
            return None;
        }
        let _permit = self.spell_lane_permit().await?;
        let mut fence = self.current_quest_fence(session).await.ok()??;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime
                    .player_control_facts(live.actor, session)
                    .ok()?
                    .control_loss
                    .is_some()
                || self
                    .states
                    .spell_states
                    .try_lock()
                    .ok()?
                    .is_dead(live.actor)
                || !runtime
                    .bound_attacker_lease(live.actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return None;
            }
        }
        let origin = durable::NativeArenaOrigin::Timeout(durable::NativeArenaTimeout::new(
            live.actor,
            session,
            self.reset_epoch,
            &live.state,
        )?);
        let read = self
            .root
            .read_arena_native_origin(self.character, self.holder, fence, &origin)
            .await
            .ok()?;
        if read.dead {
            return None;
        }
        if read.replay.is_some() || read.state != live.state {
            if read.state.rank == 0 {
                return self
                    .restore_arena_projection(live.actor, session, &read.state, read.now_ms)
                    .await;
            }
            return None;
        }
        if read.now_ms < read.state.deadline_ms {
            return None;
        }
        let after = read.state.idle();
        let command = origin.command();
        let bytes = slot
            .commit_arena_match(
                self.root,
                self.character,
                self.holder,
                fence,
                MatchTransition {
                    native_origin: Some(origin),
 cause: (command).into(),
                    payload: wire::encode_match_request(A::Status).ok()?,
                    before: read.state,
                    after: after.clone(),
                    disposition: D::TimedOut,
                    victory: None,
                    change: None,
                    achievement: None,
                },
            )
            .await
            .ok()?;
        if wire::decode_match_result(&bytes).ok()?.disposition != D::TimedOut {
            return None;
        }
        self.restore_arena_projection(live.actor, session, &after, read.now_ms)
            .await
    }
}
