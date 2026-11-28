//! M51 silent death/departure consumer on the coalesced creature cadence.
//! Crystal201189f creaturescripts_arena_kill.lua:2-38. The current runtime proves
//! most-damage credit; the existing fenced arena writer persists the victory.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_arena_matches(&self, now_us: u64) {
        {
            let Ok(mut poll) = self.states.arena_matches.victory_poll.lock() else {
                return;
            };
            if poll.is_some_and(|(epoch, due)| epoch == self.reset_epoch && now_us < due) {
                return;
            }
            *poll = Some((
                self.reset_epoch,
                now_us.saturating_add(
                    crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS.saturating_mul(1_000),
                ),
            ));
        }
        // The persisted lease can be ahead of the live opponent after a failed
        // Ground reset or a lost transition acknowledgement. Reconcile it
        // before consulting a deadline from the previous fight.
        let Some(resumed) = Box::pin(self.resume_arena_projections()).await else {
            return;
        };
        let pending = {
            let Ok(live) = self.states.arena_matches.live.lock() else {
                return;
            };
            live.iter()
                .filter(|(session, live)| {
                    matches!(live.state.phase, 1 | 2) && !resumed.contains(session)
                })
                .take(10)
                .map(|(session, live)| (*session, live.clone()))
                .collect::<Vec<_>>()
        };
        for (session, live) in pending {
            if Instant::now() >= live.deadline {
                let _ = Box::pin(self.settle_arena_timeout(session, &live)).await;
                continue;
            }
            // Keep the optional reset future out of every connection's stack.
            let _ = Box::pin(self.settle_arena_departure(session, &live)).await;
            if live.state.phase == 1 {
                self.settle_arena_victory(session, live).await;
            }
        }
        self.clear_arena_orphans().await;
    }

    async fn resume_arena_projections(&self) -> Option<Vec<GameSessionId>> {
        if self.bundle_map().is_none() {
            return Some(Vec::new());
        }
        let (scope, generation, positioned) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().ok()?;
            let binding = runtime.binding();
            let positioned: Vec<_> = runtime.positioned_actor_census().ok()?
                .into_iter().filter_map(|(_, _, session)| session).collect();
            (binding.runtime_scope(), binding.scope_generation().get(), positioned)
        };
        let cursor = *self.states.arena_matches.relocation_cursor.lock().ok()?;
        let read = self.root.read_arena_projection_leases(
            self.character, scope, generation, &positioned, cursor,
        );
        #[cfg(test)]
        let read = pause_poll_read(read);
        let (now_ms, leased) = read.await.ok()?;
        // Advance even when a row cannot be projected. An empty page starts
        // a new sweep so temporarily blocked rows still get retried.
        {
            let mut saved = self.states.arena_matches.relocation_cursor.lock().ok()?;
            if *saved == cursor {
                *saved = leased.iter()
                    .filter(|projection| projection.relocation_pending)
                    .map(|projection| projection.character)
                    .max_by_key(|character| *character.as_bytes());
            }
        }
        let mut resumed = Vec::new();
        for projection in leased {
            let state = projection.state;
            let session = projection.session;
            let live = self.states.arena_matches.live.lock().ok()?.get(&session).cloned();
            if !projection.relocation_pending && live.as_ref().is_some_and(|current| current.state == state) {
                continue;
            }
            let actor = {
                let runtime = self.runtime.lock().await;
                runtime.positioned_player_for_session(session).ok().flatten().map(|(actor, _)| actor)
            };
            let Some(actor) = actor else { continue };
            let Some(permit) = self.spell_lane_permit().await else { continue };
            let Ok(Some(fence)) = self.current_quest_fence(session).await else { continue };
            if fence.character_id != projection.character
                || fence.runtime_scope != scope
                || fence.scope_ownership_generation.get() != generation
            {
                continue;
            }
            {
                let runtime = self.runtime.lock().await;
                if !self.arena_poll_current(&runtime, actor, fence) {
                    continue;
                }
            }
            let state = if projection.relocation_pending {
                let Ok(Some(rebound)) = self.root
                    .prepare_arena_relocation(self.character, self.holder, fence, &state).await
                else { continue };
                rebound
            } else if self.root
                .arena_projection_current(self.character, self.holder, fence, &state).await.unwrap_or(false)
            {
                state
            } else {
                continue;
            };
            let already_left = {
                let mut runtime = self.runtime.lock().await;
                if !self.arena_poll_current(&runtime, actor, fence) {
                    continue;
                }
                if projection.legacy_victory_pit.is_none() && projection.relocation_pending && state.phase == 3 {
                    let Ok(current) = runtime.read_actor_position(actor) else { continue };
                    // Admission can restore any saved cell. Prove the reward destination
                    // before acknowledging a committed victory.
                    let reward = position(donor::REWARD_ROOM);
                    if current.position() != reward
                        && (!walkable(self, &runtime, reward)
                            || runtime.relocate_arena_player(actor, session, current, reward).is_err())
                    {
                        continue;
                    }
                }
                projection.legacy_victory_pit.is_some_and(|pit| {
                    runtime.read_actor_position(actor).is_ok_and(|at| !in_pit(at.position(), pit))
                })
            };
            resumed.push(session);
            if matches!(state.phase, 1 | 2) && now_ms >= state.deadline_ms {
                drop(permit);
                let pending = LiveMatch { actor, opponent: None, state, deadline: Instant::now() };
                let _ = Box::pin(self.settle_arena_timeout(session, &pending)).await;
                continue;
            }
            if (already_left || self.restore_arena_projection(actor, session, &state, now_ms, &permit, fence).await.is_some())
                && projection.relocation_pending
            {
                let _ = self.root.acknowledge_arena_relocation(self.character, self.holder, fence, &state).await;
            }
        }
        Some(resumed)
    }

    async fn clear_arena_orphans(&self) {
        let candidates = {
            let runtime = self.runtime.lock().await;
            if !walkable(self, &runtime, position(donor::KICK)) {
                return;
            }
            let Ok(matches) = self.states.arena_matches.live.lock() else { return };
            let Ok(positioned) = runtime.positioned_actor_census() else { return };
            positioned.into_iter().filter_map(|(actor, at, session)| {
                let session = session?;
                (!matches.contains_key(&session) && (1..=10).any(|pit| in_pit(at.position(), pit)))
                    .then_some((actor, at, session))
            }).take(donor::CENTERS.len()).collect::<Vec<_>>()
        };
        for (actor, at, session) in candidates {
            let Some(_permit) = self.spell_lane_permit().await else { continue };
            let Ok(Some(fence)) = self.current_quest_fence(session).await else { continue };
            if !self.root.arena_orphan_can_leave(self.character, self.holder, fence).await.unwrap_or(false) {
                continue;
            }
            let mut runtime = self.runtime.lock().await;
            if self.arena_poll_current(&runtime, actor, fence)
                && runtime.read_actor_position(actor) == Ok(at)
                && walkable(self, &runtime, position(donor::KICK))
            {
                let _ = runtime.relocate_arena_player(actor, session, at, position(donor::KICK));
            }
        }
    }

    async fn settle_arena_victory(&self, session: GameSessionId, live: LiveMatch) {
        let candidate = {
            let mut runtime = self.runtime.lock().await;
            observe(self, &mut runtime, session, &live)
        };
        let Some((death, character)) = candidate else {
            return;
        };
        if !matches!(
            self.drain_kill_rewards(session).await,
            super::super::kill_reward::DrainOutcome::Drained
        ) {
            return;
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let Some(cause) = durable::NativeArenaDeath::new(
            live.actor,
            session,
            death,
            self.reset_epoch,
            live.state.run,
            live.state.pit,
        ) else {
            return;
        };
        let origin = durable::NativeArenaOrigin::Death(cause);
        let Ok(read) = self
            .root
            .read_arena_native_origin(self.character, self.holder, fence, &origin)
            .await
        else {
            return;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let Ok(Some(mut fence)) = self.current_quest_fence(session).await else {
            return;
        };
        if fence.character_id.as_bytes() != character.as_bytes() {
            return;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(revision) = slot.cursor(self.root, self.character).await else {
            return;
        };
        fence.expected_character_revision = revision;
        {
            let mut runtime = self.runtime.lock().await;
            if !self.arena_poll_current(&runtime, live.actor, fence)
                || observe(self, &mut runtime, session, &live) != Some((death, character))
            {
                return;
            }
        }
        if read.dead || !read.pit_owned {
            return;
        }
        if read.replay.is_some() || read.state != live.state {
            if read.state.run == live.state.run
                && read.state.pit == live.state.pit
                && self.root.arena_projection_current(self.character, self.holder, fence, &read.state)
                    .await.unwrap_or(false)
            {
                let _ = self
                    .restore_arena_projection(live.actor, session, &read.state, read.now_ms, &permit, fence)
                    .await;
            }
            return;
        }
        let before = read.state;
        if before.phase != 1 || before.deadline_ms <= read.now_ms {
            return;
        }
        let mut after = before.clone();
        after.phase = 2;
        let command = origin.command();
        let Ok(payload) = wire::encode_match_request(A::Status) else {
            return;
        };
        let Ok(bytes) = slot
            .commit_arena_match(
                self.root,
                self.character,
                self.holder,
                fence,
                MatchTransition {
                    native_origin: Some(origin),
 cause: (command).into(),
                    payload,
                    before,
                    after: after.clone(),
                    disposition: D::Applied,
                    victory: Some(death),
                    change: None,
                    achievement: None,
                },
            )
            .await
        else {
            return;
        };
        if wire::decode_match_result(&bytes).is_ok_and(|result| result.disposition == D::Applied) {
            let _ = self
                .restore_arena_projection(live.actor, session, &after, read.now_ms, &permit, fence)
                .await;
        }
    }
}

fn observe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    session: GameSessionId,
    live: &LiveMatch,
) -> Option<(
    crate::foundation::CreatureDeathOccurrenceKey,
    crate::foundation::CharacterId,
)> {
    if runtime.owner_fence().is_err()
        || runtime.player_control_facts(live.actor, session).is_err()
        || !runtime
            .read_actor_position(live.actor)
            .is_ok_and(|at| in_pit(at.position(), live.state.pit))
        || !owner
            .states
            .spell_states
            .try_lock()
            .is_ok_and(|states| !states.is_dead(live.actor))
        || Instant::now() >= live.deadline
    {
        return None;
    }
    let character = runtime
        .bound_attacker_lease(live.actor, session)
        .ok()?
        .character_id();
    let opponent = live.opponent?;
    let deaths = runtime.borrow_combat_death();
    let key = deaths.projected_death(opponent).ok()?.0;
    (deaths.top_damage_character(opponent).ok()? == Some(character)).then_some((key, character))
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static ARENA_POLL_READ_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

#[cfg(test)]
async fn pause_poll_read<T>(read: impl std::future::Future<Output = T>) -> T {
    let result = read.await;
    if let Ok(barrier) = ARENA_POLL_READ_BARRIER.try_with(std::sync::Arc::clone) {
        // Keep the read future pending at the handoff, with its SQL holder released.
        barrier.0.notify_one();
        barrier.1.notified().await;
    }
    result
}
