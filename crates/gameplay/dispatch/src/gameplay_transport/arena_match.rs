//! SV15 command owner. The client requests transitions; only the current carrier's
//! committed lethal projection can open the next pit. Lock order: command gate,
//! revision slot, runtime, then the leaf live-match registry. No runtime guard awaits SQL.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::arena_match::{self as durable, MatchState, MatchTransition, donor};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::map::facts::ArenaPillarProjection as PillarProjection;
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction as A, ArenaDisposition as D};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[path = "arena_match/pit_exit.rs"]
mod pit_exit;
#[path = "arena_match/death_poll.rs"]
mod death_poll;
#[path = "arena_match/departure_poll.rs"]
mod departure_poll;
#[path = "arena_match/pillar.rs"]
mod pillar;
#[path = "arena_match/pit_reset.rs"]
mod pit_reset;
#[path = "arena_match/portal_step.rs"]
mod portal_step;
#[path = "arena_match/timeout_poll.rs"]
mod timeout_poll;

macro_rules! authority_methods {
    () => {
        fn arena_match_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn arena_match_command(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.arena_match_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;

struct ArenaCaller {
    actor: ExactActorRef,
    supplied: crate::durability::item_transfer::CurrentCharacterItemFence,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
}

#[derive(Default)]
pub(in crate::gameplay_transport) struct Matches {
    command: tokio::sync::Mutex<()>,
    live: Mutex<HashMap<GameSessionId, LiveMatch>>,
    victory_poll: Mutex<Option<(u64, u64)>>,
    #[cfg(test)]
    fixture_pit_ms: std::sync::atomic::AtomicU64,
}
impl Matches {
    fn pit_millis(&self) -> i64 {
        #[cfg(test)]
        {
            let fixture = self
                .fixture_pit_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            if let Ok(millis) = i64::try_from(fixture)
                && (1..=i64::from(donor::PIT_SECONDS) * 1000).contains(&millis)
            {
                return millis;
            }
        }
        i64::from(donor::PIT_SECONDS) * 1000
    }
}

#[derive(Clone)]
struct LiveMatch {
    actor: ExactActorRef,
    opponent: Option<ExactActorRef>,
    state: MatchState,
    deadline: Instant,
}

fn position(point: (i32, i32, i16)) -> MovementLocalPosition {
    MovementLocalPosition {
        x: point.0,
        y: point.1,
        floor: point.2,
    }
}
fn center(pit: u8) -> Option<MovementLocalPosition> {
    let (x, y) = *donor::CENTERS.get(usize::from(pit.checked_sub(1)?))?;
    Some(position((x, y, 7)))
}
fn opponent(rank: u8, pit: u8) -> Option<String> {
    Some(format!(
        "oteryn:creature.{}",
        donor::OPPONENTS
            .get(usize::from(rank.checked_sub(1)?))?
            .get(usize::from(pit.checked_sub(1)?))?
    ))
}
fn at_entrance(at: MovementLocalPosition) -> bool {
    at.floor == 6 && (32220..=32260).contains(&at.x) && (31080..=31105).contains(&at.y)
}
fn in_pit(at: MovementLocalPosition, pit: u8) -> bool {
    center(pit)
        .is_some_and(|c| at.floor == c.floor && at.x.abs_diff(c.x) <= 6 && at.y.abs_diff(c.y) <= 6)
}
pub(super) fn walkable(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    use crate::content::{CollisionClass, LogicalCell};
    let cell = LogicalCell {
        x: at.x,
        y: at.y,
        z: i32::from(at.floor),
    };
    let cells = owner.movement_cells();
    cells.scope().generation_digest == runtime.content_pin().server_artifact_digest()
        && cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
        && owner
            .door
            .try_lock()
            .is_ok_and(|doors| !doors.blocking_cells().contains(&cell))
}

fn admits(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    walkable(owner, runtime, at) && runtime.native_summon_cell_free(at)
}

fn credited_victory(
    runtime: &mut ChannelRuntimeV1,
    opponent: ExactActorRef,
    character: &[u8; 16],
) -> Option<crate::foundation::CreatureDeathOccurrenceKey> {
    let death = runtime.borrow_combat_death();
    let (key, _) = death.projected_death(opponent).ok()?;
    death.top_damage_character(opponent).ok()?
        .filter(|winner| winner.as_bytes() == character)
        .map(|_| key)
}

#[cfg(test)]
pub(in crate::gameplay_transport) fn pit_exit_open(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> bool {
    owner.states.arena_matches.live.lock().is_ok_and(|matches| {
        matches.get(&session).is_some_and(|live| live.state.phase == 2)
    })
}

#[cfg(test)]
pub(in crate::gameplay_transport) fn expire_pit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) {
    if let Ok(mut matches) = owner.states.arena_matches.live.lock()
        && let Some(live) = matches.get_mut(&session)
    {
        live.deadline = Instant::now();
    }
}

/// Registered on the existing creature-owner cadence. A silent player cannot keep
/// a pit beyond the donor deadline; logout/death/relocation release its opponent.
pub(in crate::gameplay_transport) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
) {
    if runtime.owner_fence().is_err() {
        return;
    }
    let Ok(mut matches) = owner.states.arena_matches.live.lock() else {
        return;
    };
    matches.retain(|session, live| {
        let present = runtime.player_control_facts(live.actor, *session).is_ok();
        let inside = runtime
            .read_actor_position(live.actor)
            .is_ok_and(|p| in_pit(p.position(), live.state.pit));
        let expired = Instant::now() >= live.deadline;
        let dead = owner
            .states
            .spell_states
            .try_lock()
            .is_ok_and(|states| states.is_dead(live.actor));
        if present && inside && !expired && !dead {
            // The existing owner cadence also runs for a silent connection.
            let character = runtime
                .bound_attacker_lease(live.actor, *session)
                .ok()
                .map(|lease| *lease.character_id().as_bytes());
            if live.state.phase == 1
                && character.is_some_and(|character| {
                    credited_victory(runtime, live.opponent, &character).is_some()
                })
            {
                live.state.phase = 2;
            }
            if live.state.phase == 2 && let Some(map) = owner.bundle_map() {
                pit_exit::project(map, live.state.pit, true);
            }
            return true;
        }
        if let Some(map) = owner.bundle_map() {
            pit_exit::project(map, live.state.pit, false);
        }
        let _ = runtime.remove_arena_opponent(live.opponent);
        // Retain the run until its fenced native departure reset settles. A
        // failed database pass must not discard the only retry owner.
        if present && !inside && !expired && !dead {
            return true;
        }
        if let Some(opponent) = live.opponent {
            let _ = runtime.remove_arena_opponent(opponent);
        }
        let _ = pillar::project(owner, &live.state, PillarProjection::Release);
        if present
            && inside
            && expired
            && !dead
            && walkable(owner, runtime, position(donor::KICK))
            && let Ok(before) = runtime.read_actor_position(live.actor)
        {
            let _ =
                runtime.relocate_arena_player(live.actor, *session, before, position(donor::KICK));
        }
        if present && expired && !dead {
            // Physical kick remains timely if persistence fails. Retain the
            // run/deadline so the next native pass can finish its durable reset.
            live.opponent = None;
            return true;
        }
        false
    });
    // A fresh login or reset cannot retain an unfenced placement in a pit.
    if walkable(owner, runtime, position(donor::KICK))
        && let Ok(players) = runtime.positioned_actor_census()
    {
        for (actor, at, session) in players {
            let Some(session) = session else {
                continue;
            };
            if !matches.contains_key(&session)
                && (1..=10).any(|pit| in_pit(at.position(), pit))
                && owner
                    .states
                    .spell_states
                    .try_lock()
                    .is_ok_and(|states| !states.is_dead(actor))
            {
                let _ = runtime.relocate_arena_player(actor, session, at, position(donor::KICK));
            }
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Exercise real database expiry quickly; production always uses donor600s.
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn regression_arena_pit_time(
        &self,
        millis: u64,
    ) -> Option<()> {
        if millis == 0
            || millis > (u64::from(donor::PIT_SECONDS) * 1000)
            || !self.states.arena_matches.live.lock().ok()?.is_empty()
        {
            return None;
        }
        self.states
            .arena_matches
            .fixture_pit_ms
            .store(millis, std::sync::atomic::Ordering::Relaxed);
        Some(())
    }
    pub(in crate::gameplay_transport) async fn arena_match_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let action = wire::decode_match_request(payload).ok()?;
        if let A::Drome(action) = action {
            return self.drome_command(admitted, id, action, payload).await;
        }
        let actor = admitted.runtime_actor?;
        let supplied = admitted.item_fence?;
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?);
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ARENA_V1)
            || supplied.game_session_id != admitted.game_session_id
            || supplied.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let bytes = self.arena_match_native(
            ArenaCaller { actor, supplied, world: admitted.world_id, channel: admitted.channel_id },
            command.into(), action, payload, None,
        ).await?;
        let result = wire::decode_match_result(&bytes).ok()?;
        if result.state.completed_rank == 3 && result.state.claimed_masks.iter().all(|mask| mask & 8 != 0) {
            self.finish_arena_quest(supplied, command).await?;
        }
        Some(bytes)
    }

    async fn finish_arena_quest(
        &self,
        supplied: crate::durability::item_transfer::CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Option<()> {
        use crate::durability::quest_state::QuestTransitionOutcome;
        use oteryn_gameplay_quests::quest::{arena_completion, QuestRefusal};
        let fence = self.current_quest_fence(command.game_session_id()).await.ok()??;
        if fence.item_fence() != supplied { return None; }
        let copy = self.root.read_character_quest_state(self.character, fence.character_id).await.ok()?;
        if copy.states().get(arena_completion::QUEST).is_some_and(|state| state.completed) { return Some(()); }
        let catalogue = self.quest_catalogue.as_ref()?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot.commit_arena_quest_completion(self.root, self.character, self.holder, fence, command,
            std::sync::Arc::clone(catalogue), &self.premium_coordinator).await.ok()?;
        drop(slot);
        match outcome {
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_) => {
                self.refresh_quest_session(command.game_session_id(), false).await;
                Some(())
            }
            // Earned trophies remain claimed while a required current admission is absent.
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch) => Some(()),
            _ => None,
        }
    }

    async fn arena_match_native(
        &self,
        caller: ArenaCaller,
        cause: durable::MatchCause,
        action: A,
        payload: &[u8],
        expected: Option<crate::foundation::MovementPositionSnapshot>,
    ) -> Option<Vec<u8>> {
        let actor = caller.actor;
        let supplied = caller.supplied;
        let command = cause.command();
        let session = command.game_session_id();
        // Native STEP already holds the spell lane. Arena metadata and pending
        // combat settlement serialize on the same Character revision slot.
        if matches!(cause, durable::MatchCause::Command(_)) && !matches!(
            self.drain_kill_rewards(session).await,
            super::kill_reward::DrainOutcome::Drained
        ) {
            return None;
        }
        let _gate = self.states.arena_matches.command.lock().await;
        let mut fence = self
            .current_quest_fence(session)
            .await
            .ok()??;
        if fence.item_fence() != supplied {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if binding.world_id() != caller.world
                || binding.channel_id() != Some(caller.channel)
                || binding.runtime_scope() != fence.runtime_scope
                || binding.scope_generation() != fence.scope_ownership_generation
                || runtime.owner_fence().is_err()
                || runtime
                    .player_control_facts(actor, session)
                    .ok()?
                    .control_loss
                    .is_some()
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return None;
            }
            tick(self, &mut runtime);
            if let Some(expected) = expected
                && (runtime.read_actor_position(actor) != Ok(expected)
                    || !pit_exit::owns_exit(self, actor, session, expected.position()))
            { return None; }
        }
        let read = self
            .root
            .read_arena_match(self.character, self.holder, fence, cause, payload)
            .await
            .ok()?;
        if !read.dead
            && read.pit_owned
            && (read.state.rank == 0
                || (read.state.session == *session.as_bytes()
                    && read.state.scope_generation == fence.scope_ownership_generation.get()))
        {
            self.restore_arena_projection(
                actor,
                session,
                &read.state,
                read.now_ms,
            )
            .await?;
        }
        if let Some(replay) = read.replay {
            return Some(replay);
        }
        let before = read.state;
        let mut victory = None;
        let mut after = before.clone();
        let mut disposition = D::Applied;
        let live = self
            .states
            .arena_matches
            .live
            .lock()
            .ok()?
            .get(&session)
            .cloned();
        let stale = after.rank > 0
            && (after.session != *session.as_bytes()
                || after.scope_generation != fence.scope_ownership_generation.get()
                || (read.dead && matches!(after.phase, 1 | 2))
                || (!read.pit_owned && matches!(after.phase, 1 | 2)));
        let expired = matches!(after.phase, 1 | 2) && after.deadline_ms <= read.now_ms;
        if stale || expired {
            after = after.idle();
            disposition = if expired { D::TimedOut } else { D::WrongState };
        } else if after.phase == 1
            && let Some(current) = &live
            && let Some(opponent) = current.opponent
        {
            let mut runtime = self.runtime.lock().await;
            victory = credited_victory(&mut runtime, current.opponent, fence.character_id.as_bytes());
            if victory.is_some() {
            let death_owner = runtime.borrow_combat_death();
            let dead = death_owner.projected_death(opponent).is_ok()
                && death_owner
                    .top_damage_character(opponent)
                    .is_ok_and(|winner| {
                        winner.is_some_and(|id| id.as_bytes() == fence.character_id.as_bytes())
                    });
            if dead {
                victory = death_owner
                    .projected_death(opponent)
                    .ok()
                    .map(|(key, _)| key);
                after.phase = 2;
            }
        }
        let mut change = None;
        let mut spawn = None;
        if !expired && !stale {
            match action {
                A::Drome(_) => return None,
                A::Status => {}
                A::Enter if self.runtime.lock().await.is_tibiadrome(actor) => {
                    disposition = D::WrongState;
                }
                A::Enter if after.rank == 0 && after.completed < 3 => {
                    let rank = after.completed + 1;
                    let runtime = self.runtime.lock().await;
                    let at = runtime.read_actor_position(actor).ok()?.position();
                    if !at_entrance(at) {
                        disposition = D::WrongState;
                    } else if !self.arena_pit_ready(&runtime, rank, 1, session) {
                        disposition = D::Busy;
                    } else {
                        after = MatchState {
                            completed: after.completed,
                            entry_revision: fence
                                .expected_character_revision
                                .get()
                                .checked_add(1)?,
                            rank,
                            pit: 1,
                            phase: 1,
                            deadline_ms: read.now_ms + i64::from(donor::PIT_SECONDS) * 1000,
                            session: *session.as_bytes(),
                            deadline_ms: read.now_ms + self.states.arena_matches.pit_millis(),
                            session: *admitted.game_session_id.as_bytes(),
                            scope_generation: fence.scope_ownership_generation.get(),
                            run: durable::identity(command, b"run"),
                        };
                        spawn = Some((rank, 1));
                    }
                }
                A::Enter if after.completed == 3 && after.rank == 0 => {
                    disposition = D::AllChallengesCompleted
                }
                A::Advance if after.phase == 2 => {
                    let runtime = self.runtime.lock().await;
                    if !runtime
                        .read_actor_position(actor)
                        .is_ok_and(|p| in_pit(p.position(), after.pit))
                    {
                        disposition = D::WrongState;
                    } else if after.pit == 10
                        && !walkable(self, &runtime, position(donor::REWARD_ROOM))
                    {
                        disposition = D::ContentUnavailable;
                    } else if after.pit == 10 {
                        after.completed = after.rank;
                        after.phase = 3;
                        after.pit = 0;
                        after.deadline_ms = 0;
                    } else if self.arena_pit_ready(
                        &runtime,
                        after.rank,
                        after.pit + 1,
                        session,
                    ) {
                        after.pit += 1;
                        after.phase = 1;
                        after.deadline_ms = read.now_ms + self.states.arena_matches.pit_millis();
                        spawn = Some((after.rank, after.pit));
                    } else {
                        disposition = D::Busy;
                    }
                }
                A::Leave if after.rank > 0 => {
                    let runtime = self.runtime.lock().await;
                    if walkable(self, &runtime, position(donor::KICK)) {
                        after = after.idle();
                    } else {
                        disposition = D::ContentUnavailable;
                    }
                }
                A::ClaimReward {
                    rank,
                    category,
                    choice,
                } => {
                    let runtime = self.runtime.lock().await;
                    let at = runtime.read_actor_position(actor).ok()?.position();
                    let room = position(donor::REWARD_ROOM);
                    if after.phase != 3
                        || at.floor != room.floor
                        || at.x.abs_diff(room.x) > 20
                        || at.y.abs_diff(room.y) > 30
                    {
                        disposition = D::WrongState;
                    } else {
                        drop(runtime);
                        if let Some(request) = self
                            .arena_reward_request(command, rank, category, choice)
                            .await
                        {
                            let native = self.active_generation?.native_gameplay()?;
                            let result = self
                                .root
                                .claim_arena_reward(
                                    self.character,
                                    self.holder,
                                    fence,
                                    request,
                                    rank,
                                    category,
                                    choice,
                                    payload.to_vec(),
                                    &super::item_ref_admission::NativeItemWeights(native),
                                )
                                .await;
                            return match result {
                                Ok(bytes) => Some(bytes),
                                Err(error) => {
                                    super::operator_event(&format!(
                                        "arena_reward_failed reason={error:?}"
                                    ));
                                    None
                                }
                            };
                        } else {
                            disposition = D::ContentUnavailable;
                        }
                    }
                }
                _ => disposition = D::WrongState,
            }
        }
        if spawn.is_some() && before.rank == 0 {
            let bag = self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .ok()?;
            let runtime = self.runtime.lock().await;
            change = super::charm_progression::fee_change(
                self,
                &runtime,
                bag.as_ref(),
                [
                    durable::identity(command, b"change-platinum"),
                    durable::identity(command, b"change-gold"),
                ],
            );
            if change.is_none() {
                after = before.clone();
                disposition = D::ContentUnavailable;
            }
        }
        let response = slot
            .commit_arena_match(
                self.root,
                self.character,
                self.holder,
                fence,
                MatchTransition {
                    cause,
                    native_origin: None,
                    command,
                    payload: payload.to_vec(),
                    before: before.clone(),
                    after: after.clone(),
                    disposition,
                    change,
                    victory,
                    achievement: (after.completed > before.completed).then(|| {
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
                    }),
                },
            )
            .await;
        let bytes = match response {
            Ok(bytes) => bytes,
            Err(error) => {
                super::operator_event(&format!("arena_match_failed reason={error:?}"));
                return None;
            }
        };
        let result = wire::decode_match_result(&bytes).ok()?;
        // The fee owner can refuse the proposed entry atomically. Project only
        // the state it committed, never the unpaid proposal.
        after.reconcile_arena_refusal(&before, result.disposition, &result.state);
        if result.disposition != D::Applied
            && !matches!(result.disposition, D::TimedOut)
            && after == before
        {
            return Some(bytes);
        }
        self.restore_arena_projection(actor, session, &after, read.now_ms)
            .await?;
        Some(bytes)
    }

    async fn restore_arena_projection(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        state: &MatchState,
        now_ms: i64,
    ) -> Option<()> {
        let existing = self
            .states
            .arena_matches
            .live
            .lock()
            .ok()?
            .get(&session)
            .cloned();
        let reusable = existing.as_ref().is_some_and(|live| {
            live.actor == actor
                && live.state == *state
                && state.projection_remaining_ms(now_ms).is_some()
        });
        let mut runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || self
                .states
                .spell_states
                .try_lock()
                .is_ok_and(|states| states.is_dead(actor))
        {
            return None;
        }
        if reusable {
            let projection = if state.phase == 2 {
                PillarProjection::Victory
            } else {
                PillarProjection::Fighting
            };
            let _ = pillar::project(self, state, projection);
            return Some(());
        }
        if let Some(remaining_ms) = state.projection_remaining_ms(now_ms) {
            if let Some(old) = &existing
                && (old.state.pit != state.pit || old.state.run != state.run)
            {
                let _ = pillar::project(self, &old.state, PillarProjection::Release);
            }
            let destination = center(state.pit)?;
            if !walkable(self, &runtime, destination) {
                return None;
            }
            if state.phase == 2 {
                let _ = pillar::project(self, state, PillarProjection::Victory);
                // Crystal svargrond_arena.lua:285-313: waiting at the victory
                // portal still owns the original kick timer, including recovery.
                if let Some(opponent) = existing.and_then(|old| old.opponent) {
                    let _ = runtime.remove_arena_opponent(opponent);
                }
                self.states.arena_matches.live.lock().ok()?.insert(
                    session,
                    LiveMatch {
                        actor,
                        opponent: None,
                        state: state.clone(),
                        deadline: Instant::now() + Duration::from_millis(remaining_ms),
                    },
                );
                return Some(());
            }
            let _ = pillar::project(self, state, PillarProjection::Fighting);
            let current = runtime.read_actor_position(actor).ok()?.position();
            if !at_entrance(current) && !(1..=10).any(|pit| in_pit(current, pit)) {
                return None;
            }
            pit_reset::reset_monsters(&mut runtime, actor, session, state)?;
            let mut summon = destination;
            summon.y += 2;
            let key = opponent(state.rank, state.pit)?;
            let target = runtime
                .spawn_arena_opponent(&key, summon, |runtime, at| Some(admits(self, runtime, at)))
                .ok()?;
            let before = runtime.read_actor_position(actor).ok()?;
            if runtime
                .relocate_arena_player(actor, session, before, destination)
                .is_err()
            {
                let _ = runtime.remove_arena_opponent(target);
                return None;
            }
            if let Some(old) = existing {
                if let Some(map) = self.bundle_map() {
                    pit_exit::project(map, old.state.pit, false);
                }
                let _ = runtime.remove_arena_opponent(old.opponent);
            if let Some(opponent) = existing.and_then(|old| old.opponent) {
                let _ = runtime.remove_arena_opponent(opponent);
            }
            self.states.arena_matches.live.lock().ok()?.insert(
                session,
                LiveMatch {
                    actor,
                    opponent: Some(target),
                    state: state.clone(),
                    deadline: Instant::now() + Duration::from_millis(remaining_ms),
                },
            );
        } else if state.phase == 3 || state.rank == 0 {
            let current = runtime.read_actor_position(actor).ok()?.position();
            let reward_room = position(donor::REWARD_ROOM);
            if let Some(old) = self.states.arena_matches.live.lock().ok()?.remove(&session) {
                let _ = pillar::project(self, &old.state, PillarProjection::Release);
                if let Some(opponent) = old.opponent {
                    let _ = runtime.remove_arena_opponent(opponent);
                }
            }
            if (1..=10).any(|pit| in_pit(current, pit))
                || (state.rank == 0
                    && current.floor == reward_room.floor
                    && current.x.abs_diff(reward_room.x) <= 20
                    && current.y.abs_diff(reward_room.y) <= 30)
            {
                if let Some(old) = self.states.arena_matches.live.lock().ok()?.remove(&session) {
                    if let Some(map) = self.bundle_map() {
                        pit_exit::project(map, old.state.pit, false);
                    }
                    let _ = runtime.remove_arena_opponent(old.opponent);
                }
                let before = runtime.read_actor_position(actor).ok()?;
                runtime
                    .relocate_arena_player(
                        actor,
                        session,
                        before,
                        position(if state.phase == 3 {
                            donor::REWARD_ROOM
                        } else {
                            donor::KICK
                        }),
                    )
                    .ok()?;
            }
        }
        Some(())
    }

    fn arena_pit_ready(
        &self,
        runtime: &ChannelRuntimeV1,
        rank: u8,
        pit: u8,
        session: GameSessionId,
    ) -> bool {
        let Some(key) = opponent(rank, pit) else {
            return false;
        };
        if runtime.companion_policy(&key).is_err()
            || !walkable(self, runtime, position(donor::KICK))
            || !walkable(self, runtime, position(donor::REWARD_ROOM))
        {
            return false;
        }
        let Some(at) = center(pit) else {
            return false;
        };
        // Crystal svargrond_arena.lua:193-205,249-256: native spectator
        // occupancy includes players not present in this process's match registry.
        let layout = match oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit) {
            Some(pit) => pit.layout(),
            None => return false,
        };
        let Ok(occupants) = runtime.positioned_actor_census() else {
            return false;
        };
        if layout.occupied_by_other(
            &session,
            occupants.into_iter().filter_map(|(_, at, player)| {
                let at = at.position();
                player.map(|player| (player, [at.x, at.y, -i32::from(at.floor)]))
            }),
        ) {
            return false;
        }
        let mut summon = at;
        summon.y += 2;
        admits(self, runtime, at)
            && admits(self, runtime, summon)
            && self.states.arena_matches.live.lock().is_ok_and(|matches| {
                !matches
                    .iter()
                    .any(|(other, live)| *other != session && live.state.pit == pit)
            })
    }

    async fn arena_reward_request(
        &self,
        command: CommandRef,
        rank: u8,
        category: u8,
        choice: u8,
    ) -> Option<crate::durability::reward_claim_mint::RewardClaimMintRequest> {
        use crate::durability::item_mint::TypedDefinitionRef;
        use crate::durability::reward_claim_mint::{
            RewardClaimMintContent, RewardClaimMintRequest,
        };
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let facts = |source: u16| {
            let key = format!("oteryn:item.tibia.i{source}");
            super::item_ref_admission::generation_item_facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    revision_ref: index.revision(&key)?.into(),
                    production_key: key,
                },
            )
            .ok()
        };
        let reward = donor::reward(rank, category, choice)?;
        let fence = self
            .current_quest_fence(command.game_session_id())
            .await
            .ok()??;
        let bag = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()?;
        let backpack = match bag {
            Some(bag) => {
                super::item_ref_admission::generation_item_facts(native, &bag.backpack.definition)
                    .ok()?
            }
            None => facts(2854)?,
        };
        let mut contents = Vec::new();
        for (source, quantity) in reward.contents {
            contents.push(RewardClaimMintContent {
                fluid_subtype: None,
                item: facts(source)?,
                quantity,
                key_number: None,
                text: None,
            });
        }
        let pin = format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        Some(RewardClaimMintRequest {
            additional_items: Vec::new(),
            command,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: donor::claim_key(rank, category),
                revision_ref: pin.clone(),
            },
            source_placement: format!("oteryn:svargrond-arena/reward/{rank}/{category}"),
            item: facts(reward.root)?,
            quantity: 1,
            contents,
            root_text: None,
            backpack,
            content_revision: pin,
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        })
    }
}
