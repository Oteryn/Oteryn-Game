//! M51 native source action25200. Crystal201189f
//! data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:24-64.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::MovementPositionSnapshot;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_arena_portal(
        &self,
        _permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        if origin.position() == entered.position() {
            return entered;
        }
        self.try_arena_portal_step(actor, session, entered)
            .await
            .unwrap_or(entered)
    }

    async fn try_arena_portal_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        // A wire command can hold this gate while waiting for the spell lane.
        // Native STEP already holds that lane: never await the opposite lock.
        let _gate = self.states.arena_matches.command.try_lock().ok()?;
        let live = self
            .states
            .arena_matches
            .live
            .lock()
            .ok()?
            .get(&session)?
            .clone();
        if live.actor != actor || live.state.phase != 2 {
            return None;
        }
        let step = durable::NativeArenaStep::new(
            actor,
            session,
            entered,
            self.reset_epoch,
            live.state.run,
            live.state.pit,
        )?;
        let mut fence = self.current_quest_fence(session).await.ok()??;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || runtime.read_actor_position(actor) != Ok(entered)
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                || !portal_open(self, &live.state)
            {
                return None;
            }
        }
        let read = self
            .root
            .read_arena_portal_step(self.character, self.holder, fence, &step)
            .await
            .ok()?;
        if read.dead || !read.pit_owned {
            return None;
        }
        if read.replay.is_some() {
            self.restore_arena_projection(actor, session, &read.state, read.now_ms)
                .await?;
            return self.runtime.lock().await.read_actor_position(actor).ok();
        }
        let before = read.state;
        if before != live.state || before.phase != 2 || before.deadline_ms <= read.now_ms {
            return None;
        }
        let mut after = before.clone();
        let disposition = {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered) || !portal_open(self, &before) {
                return None;
            }
            if before.pit == 10 {
                if !walkable(self, &runtime, position(donor::REWARD_ROOM)) {
                    D::ContentUnavailable
                } else {
                    after.completed = before.rank;
                    after.phase = 3;
                    after.pit = 0;
                    after.deadline_ms = 0;
                    D::Applied
                }
            } else if self.arena_pit_ready(&runtime, before.rank, before.pit + 1, session) {
                after.pit += 1;
                after.phase = 1;
                after.deadline_ms = read.now_ms + i64::from(donor::PIT_SECONDS) * 1000;
                D::Applied
            } else {
                D::Busy
            }
        };
        let command = step.command();
        let achievement = (after.completed > before.completed).then(|| {
            use crate::durability::account_achievement::{
                AchievementGrantRequest, AchievementSourceEvent,
            };
            let key = donor::ACHIEVEMENTS[usize::from(after.completed - 1)];
            AchievementGrantRequest {
                achievement_key: key.into(),
                catalogue: self.achievements.lookup(key),
                source: AchievementSourceEvent {
                    kind: "oteryn:svargrond-arena".into(),
                    event_id: before.run.to_vec(),
                },
            }
        });
        let bytes = slot
            .commit_arena_match(
                self.root,
                self.character,
                self.holder,
                fence,
                MatchTransition {
                    native_origin: Some(durable::NativeArenaOrigin::Step(step)),
                    command,
                    payload: wire::encode_match_request(A::Advance).ok()?,
                    before: before.clone(),
                    after: after.clone(),
                    disposition,
                    victory: None,
                    change: None,
                    achievement,
                },
            )
            .await
            .ok()?;
        let result = wire::decode_match_result(&bytes).ok()?;
        after.reconcile_arena_refusal(&before, result.disposition, &result.state);
        self.restore_arena_projection(actor, session, &after, read.now_ms)
            .await?;
        self.runtime.lock().await.read_actor_position(actor).ok()
    }
}

fn portal_open(owner: &ComposedFreshAdmission<'_, '_, '_>, state: &MatchState) -> bool {
    let Some(map) = owner.bundle_map() else {
        return false;
    };
    let Some(pit) = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(state.pit) else {
        return false;
    };
    let [x, y, floor] = pit.layout().pillar;
    let (Ok(x), Ok(y), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(floor))
    else {
        return false;
    };
    map.facts
        .svargrond_portal_open(crate::map::overlay::TilePos { x, y, floor }, state.run)
}
