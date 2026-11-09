//! M51 silent source death consumer, on the existing coalesced creature cadence.
//! Crystal201189f creaturescripts_arena_kill.lua:2-38. The current runtime proves
//! most-damage credit; the existing fenced arena writer persists the victory.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_arena_victories(&self, now_us: u64) {
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
        // This entry is before the spell lane. Keep the same gate/lane/slot
        // order as arena commands; corpse/kill consumers settle before removal.
        let _gate = self.states.arena_matches.command.lock().await;
        let pending = {
            let Ok(live) = self.states.arena_matches.live.lock() else {
                return;
            };
            live.iter()
                .filter(|(_, live)| live.state.phase == 1)
                .take(10)
                .map(|(session, live)| (*session, live.clone()))
                .collect::<Vec<_>>()
        };
        for (session, live) in pending {
            self.settle_arena_victory(session, live).await;
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
        let Some(_permit) = self.spell_lane_permit().await else {
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
            if observe(self, &mut runtime, session, &live) != Some((death, character))
                || !runtime
                    .bound_attacker_lease(live.actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return;
            }
        }
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
        if read.dead || !read.pit_owned {
            return;
        }
        if read.replay.is_some() || read.state != live.state {
            if read.state.run == live.state.run && read.state.pit == live.state.pit {
                let _ = self
                    .restore_arena_projection(live.actor, session, &read.state, read.now_ms)
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
                    command,
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
                .restore_arena_projection(live.actor, session, &after, read.now_ms)
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
