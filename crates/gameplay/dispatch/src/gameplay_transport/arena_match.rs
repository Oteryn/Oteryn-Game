//! SV15 command owner. The client requests transitions; only the current carrier's
//! committed lethal projection can open the next pit. Lock order: command gate,
//! revision slot, runtime, then the leaf live-match registry. No runtime guard awaits SQL.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::arena_match::{self as durable, MatchState, MatchTransition, donor};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction as A, ArenaDisposition as D};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

#[derive(Default)]
pub(in crate::gameplay_transport) struct Matches {
    command: tokio::sync::Mutex<()>,
    live: Mutex<HashMap<GameSessionId, LiveMatch>>,
}
#[derive(Clone)]
struct LiveMatch {
    actor: ExactActorRef,
    opponent: ExactActorRef,
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

/// Registered on the existing creature-owner cadence. A silent player cannot keep
/// a pit beyond the donor deadline; logout/death/relocation release its opponent.
pub(in crate::gameplay_transport) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
) {
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
            return true;
        }
        let _ = runtime.remove_arena_opponent(live.opponent);
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
    pub(in crate::gameplay_transport) async fn arena_match_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let action = wire::decode_match_request(payload).ok()?;
        if let A::Drome(action) = action { return self.drome_command(admitted, id, action, payload).await; }
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
        let _gate = self.states.arena_matches.command.lock().await;
        let mut fence = self
            .current_quest_fence(admitted.game_session_id)
            .await
            .ok()??;
        if fence.item_fence() != supplied {
            return None;
        }
        if !matches!(
            self.drain_kill_rewards(admitted.game_session_id).await,
            super::kill_reward::DrainOutcome::Drained
        ) {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if binding.world_id() != admitted.world_id
                || binding.channel_id() != Some(admitted.channel_id)
                || binding.runtime_scope() != fence.runtime_scope
                || binding.scope_generation() != fence.scope_ownership_generation
                || runtime.owner_fence().is_err()
                || runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .ok()?
                    .control_loss
                    .is_some()
                || !runtime
                    .bound_attacker_lease(actor, admitted.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return None;
            }
            tick(self, &mut runtime);
        }
        let read = self
            .root
            .read_arena_match(self.character, self.holder, fence, command, payload)
            .await
            .ok()?;
        if !read.dead && read.pit_owned
            && (read.state.rank == 0
                || (read.state.session == *admitted.game_session_id.as_bytes()
                    && read.state.scope_generation == fence.scope_ownership_generation.get()))
        {
            self.restore_arena_projection(
                actor,
                admitted.game_session_id,
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
            .get(&admitted.game_session_id)
            .cloned();
        let stale = after.rank > 0
            && (after.session != *admitted.game_session_id.as_bytes()
                || after.scope_generation != fence.scope_ownership_generation.get()
                || (read.dead && matches!(after.phase, 1 | 2))
                || (!read.pit_owned && matches!(after.phase, 1 | 2)));
        let expired = matches!(after.phase, 1 | 2) && after.deadline_ms <= read.now_ms;
        if stale || expired {
            after = after.idle();
            disposition = if expired { D::TimedOut } else { D::WrongState };
        } else if after.phase == 1
            && let Some(current) = &live
        {
            let mut runtime = self.runtime.lock().await;
            let death_owner = runtime.borrow_combat_death();
            let dead = death_owner.projected_death(current.opponent).is_ok()
                && death_owner
                    .top_damage_character(current.opponent)
                    .is_ok_and(|winner| {
                        winner.is_some_and(|id| id.as_bytes() == fence.character_id.as_bytes())
                    });
            if dead {
                victory = death_owner
                    .projected_death(current.opponent)
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
                A::Enter if self.runtime.lock().await.is_tibiadrome(actor) => {disposition=D::WrongState;}
                A::Enter if after.rank == 0 && after.completed < 3 => {
                    let rank = after.completed + 1;
                    let runtime = self.runtime.lock().await;
                    let at = runtime.read_actor_position(actor).ok()?.position();
                    if !at_entrance(at) {
                        disposition = D::WrongState;
                    } else if !self.arena_pit_ready(&runtime, rank, 1, admitted.game_session_id) {
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
                        admitted.game_session_id,
                    ) {
                        after.pit += 1;
                        after.phase = 1;
                        after.deadline_ms = read.now_ms + i64::from(donor::PIT_SECONDS) * 1000;
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
        self.restore_arena_projection(actor, admitted.game_session_id, &after, read.now_ms)
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
        if existing.as_ref().is_some_and(|live| {
            live.state.run == state.run
                && live.state.pit == state.pit
                && matches!(state.phase, 1 | 2)
        }) {
            return Some(());
        }
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
        if matches!(state.phase, 1 | 2) && state.deadline_ms > now_ms {
            let destination = center(state.pit)?;
            if !walkable(self, &runtime, destination) {
                return None;
            }
            if state.phase == 2 {
                return Some(());
            }
            let current = runtime.read_actor_position(actor).ok()?.position();
            if !at_entrance(current) && !(1..=10).any(|pit| in_pit(current, pit)) {
                return None;
            }
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
                let _ = runtime.remove_arena_opponent(old.opponent);
            }
            self.states.arena_matches.live.lock().ok()?.insert(
                session,
                LiveMatch {
                    actor,
                    opponent: target,
                    state: state.clone(),
                    deadline: Instant::now()
                        + Duration::from_millis(u64::try_from(state.deadline_ms - now_ms).ok()?),
                },
            );
        } else if state.phase == 3 || state.rank == 0 {
            let current = runtime.read_actor_position(actor).ok()?.position();
            let reward_room = position(donor::REWARD_ROOM);
            if (1..=10).any(|pit| in_pit(current, pit))
                || (state.rank == 0
                    && current.floor == reward_room.floor
                    && current.x.abs_diff(reward_room.x) <= 20
                    && current.y.abs_diff(reward_room.y) <= 30)
            {
                if let Some(old) = self.states.arena_matches.live.lock().ok()?.remove(&session) {
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
