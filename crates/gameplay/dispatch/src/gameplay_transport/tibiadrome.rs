//! W17: Drome uses the existing fenced arena route and physical creature owner.
//! Rules: docs/reference/tibia-manual/combat.md:249-284. Svargrond deaths retain
//! ordinary penalties: Crystal 201189f data-crystal/scripts/quests/arena/
//! creaturescripts_player_death.lua:2-5 only clears PitDoor; no loss exemption.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::tibiadrome::{self as rules, Record};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_protocol_oteryn::arena::{self as wire, DromeAction as A};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
pub(super) mod modifiers;

#[derive(Default)]
pub(super) struct Drome {
    /// Mechanics-only tests exercise the preexisting physical path without donor qualification.
    #[cfg(test)]
    pub(super) mechanics_test_context: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    pub(super) mechanics_test_hazards: std::sync::atomic::AtomicBool,
    command: tokio::sync::Mutex<()>,
    live: Mutex<HashMap<GameSessionId, Live>>,
}
#[cfg(test)]
impl Drome {
    pub(in crate::gameplay_transport) fn test_exploding_corpses(
        &self,
        runtime: &mut ChannelRuntimeV1,
        session: GameSessionId,
    ) -> Result<(), &'static str> {
        modifiers::select_test_modifier(self, runtime, session,
            oteryn_gameplay_entities::runtime_actor_carrier::tibiadrome_modifiers::DromeModifier::ExplodingCorpses)
    }
    pub(in crate::gameplay_transport) fn test_somersault(
        &self,
        runtime: &mut ChannelRuntimeV1,
        session: GameSessionId,
    ) -> Result<(), &'static str> {
        modifiers::select_test_modifier(self, runtime, session,
            oteryn_gameplay_entities::runtime_actor_carrier::tibiadrome_modifiers::DromeModifier::Somersault)
    }
}
struct Live {
    actor: ExactActorRef,
    run: [u8; 16],
    origin: MovementLocalPosition,
    battle: Arc<Mutex<Battle>>,
    ended: bool,
    persisted: u32,
    health: u32,
    mana: u32,
}
struct Battle {
    modifiers: Option<modifiers::ModifierOwner>,
    modifier_deaths: Vec<ExactActorRef>,
    center: MovementLocalPosition,
    opponents: Vec<ExactActorRef>,
    wave: u32,
    completed: u32,
    clears: u32,
    preparing: bool,
    deadline: Instant,
    wave_deadline_ms: u64,
    ended: bool,
}

fn walkable(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    super::arena_match::walkable(owner, runtime, at)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drome_command(
        &self,
        admitted: AdmittedSession,
        id: u64,
        action: A,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let actor = admitted.runtime_actor?;
        let supplied = admitted.item_fence?;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ARENA_V1)
            || supplied.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?);
        let _gate = self.states.tibiadrome.command.lock().await;
        let mut fence = self
            .current_quest_fence(admitted.game_session_id)
            .await
            .ok()??;
        if fence.item_fence() != supplied {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        let at_shop = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .ok()?
                    .control_loss
                    .is_some()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, admitted.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return None;
            }
            at_hall(runtime.read_actor_position(actor).ok()?.position())
        };
        let read = self
            .root
            .read_tibiadrome(self.character, self.holder, fence, command, payload)
            .await
            .ok()?;
        if let Some(reply) = read.replay {
            if !read.state.active && !self.drome_exit(actor, admitted.game_session_id).await {
                return None;
            }
            return Some(reply);
        }
        let purchase = if let A::Buy(selector) = action {
            Some(selector)
        } else {
            None
        };
        let reward = if let Some(selector) = purchase {
            rules::shop_reward(selector).map(|(item, _)| (rules::shop_key(command), item))
        } else if action == A::ClaimPotion {
            read.potion.map(|(rotation, claimed, item)| {
                (
                    format!("oteryn:reward.tibiadrome.{rotation}.{claimed}"),
                    item,
                )
            })
        } else {
            None
        };
        let requested_reward = reward.is_some();
        if at_shop
            && !read.state.active
            && let Some((key, item)) = reward
            && let Some(request) = self.drome_reward_request(command, key, item).await
        {
            let native = self.active_generation?.native_gameplay()?;
            return self
                .root
                .claim_drome_reward(
                    self.character,
                    self.holder,
                    fence,
                    request,
                    payload.to_vec(),
                    purchase,
                    &super::item_ref_admission::NativeItemWeights(native),
                )
                .await
                .ok();
        }
        let before = read.state;
        let mut after = before.clone();
        let rotation = read.now / rules::ROTATION_SECONDS;
        // A newly seen Character has no unplayed historical rotations to settle.
        if after == Record::default() {
            after.progress.rotation = rotation;
        }
        if !read.rotation_points_eligible {
            after.progress.waves = 0;
        }
        after.progress.rotate(rotation);
        let mut disposition = 1;
        let premium = self
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current();
        let level = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
            .ok()?
            .map_or(0, |p| p.level);
        {
            let mut runtime = self.runtime.lock().await;
            tick(self, &mut runtime);
            let mut live = self.states.tibiadrome.live.lock().ok()?;
            if after.active {
                let current = live
                    .get(&admitted.game_session_id)
                    .filter(|l| l.actor == actor && l.run == after.run);
                if let Some(current) = current {
                    let battle = current.battle.lock().ok()?;
                    if after.progress.rotation == before.progress.rotation
                        && battle.clears > after.completed
                    {
                        // Manual combat.md:264: each missed clear adds one
                        // starting level; the latest wave level is a single floor.
                        // Match the durable server-cadence settlement calculation.
                        let count = battle.clears - after.completed;
                        after.progress.level = after.progress.level.max(battle.completed);
                        after.progress.start = after
                            .progress
                            .start
                            .saturating_add(count)
                            .max(battle.completed);
                        after.progress.waves = after.progress.waves.saturating_add(count);
                        after.completed = battle.clears;
                    }
                    after.wave = battle.wave;
                    after.preparing = battle.preparing;
                    after.expires = read.now.saturating_add(
                        u32::try_from(
                            battle
                                .deadline
                                .saturating_duration_since(Instant::now())
                                .as_secs(),
                        )
                        .ok()?,
                    );
                    if current.ended || battle.ended {
                        after.active = false;
                        after.progress.finish(read.now);
                    }
                } else {
                    // oteryn-choice: a restart cancels an unfinished battle and applies its cooldown;
                    // only server-observed completed waves already committed remain creditable.
                    after.active = false;
                    after.progress.finish(read.now);
                }
                if after.progress.rotation != before.progress.rotation {
                    after.active = false;
                }
            }
            match action {
                A::Status => {}
                A::Enter if !after.active => {
                    if !rules::eligible(level, premium) {
                        disposition = 3;
                    } else if after.progress.cooldown_until > read.now {
                        disposition = 4;
                    } else if live.values().filter(|l| !l.ended).count() >= 5
                        || live.values().any(|l| {
                            !l.ended
                                && l.battle.lock().is_ok_and(|b| {
                                    !b.preparing || !b.opponents.is_empty() || b.clears > 0
                                })
                        })
                    {
                        disposition = 5;
                    } else {
                        let at = runtime.read_actor_position(actor).ok()?.position();
                        let vitals = self
                            .states
                            .spell_states
                            .try_lock()
                            .ok()?
                            .get(&runtime, actor, admitted.game_session_id)?
                            .vitals();
                        if !entry_ready(self, &runtime, at) {
                            disposition = 6;
                        } else if let Some(modifier_context) = modifiers::qualified_context(
                            self,
                            &runtime,
                            crate::durability::arena_match::identity(command, b"drome"),
                            1,
                            at,
                        ) {
                            let existing = live
                                .values()
                                .find(|l| !l.ended)
                                .map(|l| (l.run, Arc::clone(&l.battle)));
                            let (run, battle) = existing.unwrap_or_else(|| {
                                (
                                    crate::durability::arena_match::identity(command, b"drome"),
                                    Arc::new(Mutex::new(Battle {
                                        modifiers: Some(modifier_context),
                                        modifier_deaths: Vec::new(),
                                        center: at,
                                        opponents: Vec::new(),
                                        wave: after.progress.start,
                                        completed: after.progress.start,
                                        clears: 0,
                                        preparing: true,
                                        deadline: Instant::now()
                                            + Duration::from_secs(u64::from(
                                                rules::PREPARATION_SECONDS,
                                            )),
                                        wave_deadline_ms: 0,
                                        ended: false,
                                    })),
                                )
                            });
                            {
                                let mut battle = battle.lock().ok()?;
                                if at.floor != battle.center.floor
                                    || at.x.abs_diff(battle.center.x) > 3
                                    || at.y.abs_diff(battle.center.y) > 3
                                {
                                    return None;
                                }
                                battle.wave = battle.wave.min(after.progress.start);
                                battle.completed = battle.wave.saturating_sub(1);
                                after.completed = battle.clears;
                                after.wave = battle.wave;
                            }
                            after.run = run;
                            after.session = *admitted.game_session_id.as_bytes();
                            after.generation = fence.scope_ownership_generation.get();
                            after.active = true;
                            after.preparing = true;
                            after.expires = read.now.saturating_add(rules::PREPARATION_SECONDS);
                            live.insert(
                                admitted.game_session_id,
                                Live {
                                    actor,
                                    run,
                                    origin: at,
                                    battle,
                                    ended: false,
                                    persisted: 0,
                                    health: vitals.health,
                                    mana: vitals.mana,
                                },
                            );
                            runtime.set_tibiadrome(actor, true).ok()?;
                            if let Some(modifiers) = &live
                                .get(&admitted.game_session_id)?
                                .battle
                                .lock()
                                .ok()?
                                .modifiers
                            {
                                runtime
                                    .bind_tibiadrome_modifiers(actor, run, modifiers.selected())
                                    .ok()?;
                            }
                            self.states
                                .spell_states
                                .try_lock()
                                .ok()?
                                .get_mut(&runtime, actor, admitted.game_session_id)?
                                .drome_practice = true;
                        } else {
                            // No admitted native pool/room: never grant a legacy arena substitute.
                            disposition = 6;
                        }
                    }
                }
                A::Leave if after.active => {
                    after.active = false;
                    after.progress.finish(read.now);
                }
                A::LowerStart if !after.active && at_shop => {
                    after.progress.start = after.progress.start.saturating_sub(5);
                }
                A::Buy(17..=18) if !after.active && at_shop => {
                    disposition = 1;
                }
                A::ClaimPotion | A::Buy(_) => {
                    disposition = if !at_shop {
                        2
                    } else if requested_reward {
                        6
                    } else {
                        7
                    };
                }
                _ => {
                    disposition = 2;
                }
            }
            // Keep the physical reservation through the durable attempt. A failed commit is retried
            // against the same run rather than spawning another set of opponents.
        }
        let cosmetics = {
            let runtime = self.runtime.lock().await;
            self.active_generation
                .and_then(|g| g.native_gameplay())
                .filter(|n| n.source_digest() == runtime.content_pin().server_artifact_digest())
                .and_then(|n| n.cosmetics())
                .cloned()
        };
        let reply = self
            .root
            .commit_tibiadrome(
                self.character,
                self.holder,
                fence,
                command,
                payload.to_vec(),
                before,
                after.clone(),
                disposition,
                cosmetics,
                modifiers::snapshot(self, admitted.game_session_id),
            )
            .await
            .ok()?;
        if !after.active && !self.drome_exit(actor, admitted.game_session_id).await {
            return None;
        }
        Some(reply)
    }

    pub(super) async fn drome_exit(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        let mut runtime = self.runtime.lock().await;
        {
            let Ok(live) = self.states.tibiadrome.live.lock() else {
                return false;
            };
            if live.get(&session).is_none_or(|l| l.actor != actor) {
                return true;
            }
        }

        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return false;
        };
        let Ok(mut live) = self.states.tibiadrome.live.lock() else {
            return false;
        };
        let Some(current) = live.get(&session).filter(|l| l.actor == actor) else {
            return true;
        };
        if !walkable(self, &runtime, current.origin)
            || runtime.assert_actor_spell_unreserved(actor).is_err()
            || states.has_pending_spell_commit(actor, session)
        {
            return false;
        }
        let Ok(at) = runtime.read_actor_position(actor) else {
            return false;
        };
        if runtime
            .return_tibiadrome_player(actor, session, at, current.origin)
            .is_err()
        {
            return false;
        }
        if let Some(death) = states.player_death(&runtime, actor, session)
            && states
                .respawn(&runtime, actor, session, death.occurrence, self.owner_now())
                .is_none()
        {
            return false;
        }
        let Some(next) = states
            .get(&runtime, actor, session)
            .and_then(|s| s.after_drome_exit(current.health, current.mana))
        else {
            return false;
        };
        if !states.commit(&runtime, actor, session, next) {
            return false;
        }
        let Some(current) = live.remove(&session) else {
            return false;
        };
        if !live.values().any(|l| !l.ended && l.run == current.run)
            && let Ok(mut battle) = current.battle.lock()
        {
            for opponent in battle.opponents.drain(..) {
                let _ = runtime.remove_arena_opponent(opponent);
            }
            battle.ended = true;
        }
        runtime.set_tibiadrome(actor, false).is_ok()
    }

    pub(super) async fn drome_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: super::actor_spell::PlayerDeath,
    ) -> Option<(u64, super::actor_spell::ActorVitals)> {
        let mut runtime = self.runtime.lock().await;
        if !runtime.is_tibiadrome(actor) {
            return None;
        }
        let mut spells = self.states.spell_states.try_lock().ok()?;
        let mut live = self.states.tibiadrome.live.lock().ok()?;
        let current = live.get_mut(&session).filter(|l| l.actor == actor)?;
        if !walkable(self, &runtime, current.origin) {
            return None;
        }
        let at = runtime.read_actor_position(actor).ok()?;
        runtime
            .relocate_arena_player(actor, session, at, current.origin)
            .ok()?;
        current.ended = true;
        let _result =
            spells.respawn(&runtime, actor, session, death.occurrence, self.owner_now())?;
        if let Some(mut next) = spells
            .get(&runtime, actor, session)?
            .after_drome_exit(current.health, current.mana)
        {
            next.drome_practice = true;
            let result = (next.revision(), next.vitals());
            return spells
                .commit(&runtime, actor, session, next)
                .then_some(result);
        }
        None
    }
}

fn at_hall(at: MovementLocalPosition) -> bool {
    at.floor == 6 && (32220..=32260).contains(&at.x) && (31080..=31105).contains(&at.y)
}
fn entry_ready(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    // Oteryn's current gathering area remains in the admitted Svargrond hall.
    // Manual combat.md:267,274-276 requires space for17 opponents and100
    // warned tiles; an artificial square ring refuses the genuine irregular map.
    if !at_hall(at) {
        return false;
    }
    let cells = battle_cells(owner, runtime, at);
    cells.len() >= 100
        && rules::monster_count(5).is_some_and(|maximum| {
            cells
                .iter()
                .filter(|cell| runtime.native_summon_cell_free(**cell))
                .take(maximum)
                .count()
                == maximum
        })
}

/// At most169 cells. Every candidate has a qualified same-floor route from the
/// gathering cell; a walkable island behind source walls is not a spawn point.
fn battle_cells(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    center: MovementLocalPosition,
) -> Vec<MovementLocalPosition> {
    use std::collections::{BTreeSet, VecDeque};
    if !walkable(owner, runtime, center) {
        return Vec::new();
    }
    let mut seen = BTreeSet::from([(center.x, center.y)]);
    let mut pending = VecDeque::from([center]);
    let mut cells = Vec::new();
    while let Some(at) = pending.pop_front() {
        cells.push(at);
        for (dx, dy) in [(0_i32, -1_i32), (-1, 0), (0, 1), (1, 0)] {
            let (Some(x), Some(y)) = (at.x.checked_add(dx), at.y.checked_add(dy)) else {
                continue;
            };
            if x.abs_diff(center.x) > 6 || y.abs_diff(center.y) > 6 || seen.contains(&(x, y)) {
                continue;
            }
            let next = MovementLocalPosition {
                x,
                y,
                floor: center.floor,
            };
            if walkable(owner, runtime, next) {
                seen.insert((x, y));
                pending.push_back(next);
            }
        }
    }
    // The manual leaves exact spawn offsets to room content. Prefer distant
    // qualified cells, with stable coordinates breaking ties, rather than walls.
    cells.sort_by_key(|cell| {
        (
            std::cmp::Reverse(cell.x.abs_diff(center.x).max(cell.y.abs_diff(center.y))),
            cell.y,
            cell.x,
        )
    });
    cells
}

pub(super) fn tick(owner: &ComposedFreshAdmission<'_, '_, '_>, runtime: &mut ChannelRuntimeV1) {
    tick_at(owner, runtime, Instant::now());
}
pub(in crate::gameplay_transport) fn tick_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    now: Instant,
) {
    let Ok(mut live) = owner.states.tibiadrome.live.lock() else {
        return;
    };
    for (session, current) in live.iter_mut() {
        if current.ended {
            continue;
        }
        let present = runtime
            .player_control_facts(current.actor, *session)
            .is_ok();
        let center = match current.battle.lock() {
            Ok(battle) => battle.center,
            Err(_) => {
                current.ended = true;
                continue;
            }
        };
        let inside = runtime.read_actor_position(current.actor).is_ok_and(|p| {
            p.position().floor == center.floor
                && p.position().x.abs_diff(center.x) <= 6
                && p.position().y.abs_diff(center.y) <= 6
        });
        if !present || !inside {
            current.ended = true;
        }
    }
    let mut visited = std::collections::HashSet::new();
    for current in live.values() {
        if !visited.insert(current.run) {
            continue;
        }
        let players = live
            .values()
            .filter(|l| l.run == current.run && !l.ended)
            .count();
        let Ok(mut battle) = current.battle.lock() else {
            continue;
        };
        if battle.ended {
            continue;
        }
        // Manual combat.md:255,279: the committed native kills, rather
        // than the later observer cadence, decide the 120-second wave result.
        let defeated_in_time = !battle.preparing
            && !battle.opponents.is_empty()
            && battle.opponents.iter().all(|target| {
                let deaths = runtime.borrow_combat_death();
                deaths.projected_death(*target).is_ok()
                    && deaths
                        .projected_death_at_ms(*target)
                        .is_ok_and(|at| at.is_some_and(|at| at <= battle.wave_deadline_ms))
            });
        // Manual combat.md:255,264: native victory earns one clear immediately.
        // Corpse effects may await their HP owner, and participant exit must not
        // erase an already completed wave. Keep physical advancement below gated.
        if defeated_in_time && (battle.clears == 0 || battle.completed < battle.wave) {
            battle.completed = battle.wave;
            battle.clears = battle.clears.saturating_add(1);
        }
        if players == 0 || (!battle.preparing && now >= battle.deadline && !defeated_in_time) {
            battle.ended = true;
            for target in battle.opponents.drain(..) {
                let _ = runtime.remove_arena_opponent(target);
            }
            continue;
        }
        let modifier_players: Vec<_> = live
            .iter()
            .filter(|(_, member)| member.run == current.run && !member.ended)
            .map(|(session, member)| (member.actor, *session))
            .collect();
        modifiers::deaths(runtime, &mut battle, &modifier_players);
        if !battle.preparing {
            modifiers::tick(owner, runtime, &mut battle, &modifier_players);
        }
        if defeated_in_time
            && battle
                .modifiers
                .as_ref()
                .is_none_or(|modifiers| modifiers.settled())
        {
            battle.wave = battle.wave.saturating_add(1);
            for target in battle.opponents.drain(..) {
                let _ = runtime.remove_arena_opponent(target);
            }
            battle.modifier_deaths.clear();
            // ARENA1 A4/RL-08: reaching300 has the ordinary left-battle
            // settlement, after crediting clear299 and before preparing300.
            if battle.wave >= rules::MAX_WAVE {
                battle.ended = true;
                battle.preparing = false;
                continue;
            }
            battle.preparing = true;
            battle.deadline = now + Duration::from_secs(u64::from(rules::PREPARATION_SECONDS));
        }
        if battle.preparing && now >= battle.deadline {
            // STOPPED W17 content: these legacy arena profiles do not qualify as
            // the five Drome monsters or the nine accepted modifier consumers.
            // Exact native identities/speeds/attack clocks remain source-pending.
            let count = rules::monster_count(players).unwrap_or(0);
            let placements: Vec<_> = battle_cells(owner, runtime, battle.center)
                .into_iter()
                .filter(|at| runtime.native_summon_cell_free(*at))
                .take(count)
                .collect();
            for (index, at) in placements.into_iter().enumerate() {
                let key = format!(
                    "oteryn:creature.{}",
                    crate::durability::arena_match::donor::OPPONENTS[0][index % 5]
                );
                if let Ok(target) = runtime.spawn_arena_opponent(&key, at, |r, p| {
                    Some(walkable(owner, r, p) && r.native_summon_cell_free(p))
                }) {
                    if runtime
                        .scale_tibiadrome_opponent(target, battle.wave)
                        .is_err()
                    {
                        let _ = runtime.remove_arena_opponent(target);
                        break;
                    }
                    if let Some(modifiers) = &battle.modifiers
                        && runtime
                            .bind_tibiadrome_modifiers(target, current.run, modifiers.selected())
                            .is_err()
                    {
                        let _ = runtime.remove_arena_opponent(target);
                        break;
                    }
                    battle.opponents.push(target);
                }
            }
            if battle.opponents.len() != count {
                for target in battle.opponents.drain(..) {
                    let _ = runtime.remove_arena_opponent(target);
                }
                battle.ended = true;
            } else {
                let wave = battle.wave;
                if let Some(modifiers) = &mut battle.modifiers
                    && !modifiers.begin_wave(wave, owner.owner_now().get(), players)
                {
                    battle.ended = true;
                }
                if battle.opponents.iter().any(|actor| {
                    runtime
                        .begin_drome_native_wave(*actor, owner.owner_now().get())
                        .is_err()
                }) {
                    battle.ended = true;
                }
                battle.preparing = false;
                battle.deadline = now + Duration::from_secs(u64::from(rules::WAVE_SECONDS));
                // The projection's ms clock is the same runtime clock_origin;
                // this remains exact when observation is delayed or retried.
                battle.wave_deadline_ms = u64::try_from(
                    battle
                        .deadline
                        .saturating_duration_since(owner.clock_origin)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
            }
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn drome_reward_request(
        &self,
        command: CommandRef,
        claim_key: String,
        item: u16,
    ) -> Option<crate::durability::reward_claim_mint::RewardClaimMintRequest> {
        use crate::durability::item_mint::TypedDefinitionRef;
        use crate::durability::reward_claim_mint::RewardClaimMintRequest;
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
                production_key: claim_key,
                revision_ref: pin.clone(),
            },
            source_placement: "oteryn:tibiadrome/potion".into(),
            item: facts(item)?,
            quantity: 1,
            contents: Vec::new(),
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

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drome_sync(&self, session: GameSessionId, ending: bool) -> bool {
        let snapshot = {
            let mut runtime = self.runtime.lock().await;
            tick(self, &mut runtime);
            let Ok(mut live) = self.states.tibiadrome.live.lock() else {
                return false;
            };
            let Some(current) = live.get_mut(&session) else {
                return true;
            };
            if ending {
                current.ended = true;
            }
            let Ok(battle) = current.battle.lock() else {
                return false;
            };
            if !ending && !current.ended && !battle.ended && battle.clears <= current.persisted {
                return true;
            }
            (
                current.actor,
                current.run,
                battle.clears,
                if battle.clears > 0 {
                    battle.completed + 1
                } else {
                    0
                },
                current.ended || battle.ended,
            )
        };
        let mut fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) if ending => return self.drome_exit(snapshot.0, session).await,
            _ => return false,
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(revision) = slot.cursor(self.root, self.character).await else {
            return false;
        };
        fence.expected_character_revision = revision;
        if self
            .root
            .settle_tibiadrome(
                self.character,
                self.holder,
                fence,
                rules::RunUpdate {
                    run: snapshot.1,
                    completed: snapshot.2,
                    level: snapshot.3,
                    ended: snapshot.4,
                },
            )
            .await
            .is_err()
        {
            return false;
        }
        if snapshot.4 {
            return self.drome_exit(snapshot.0, session).await;
        } else if let Ok(mut live) = self.states.tibiadrome.live.lock()
            && let Some(current) = live.get_mut(&session).filter(|l| l.run == snapshot.1)
        {
            current.persisted = current.persisted.max(snapshot.2);
        }
        true
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn drome_item_changes_allowed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        cells: impl Iterator<Item = crate::spell::chain::TilePosition>,
    ) -> bool {
        // oteryn-choice: the qualified Channel room cannot export persistent spell
        // Items. Damage/healing runes remain usable; world-item spells are refused
        // through their existing atomic new-write rollback until an ephemeral Item
        // owner is qualified. Outsiders cannot place fields into the reserved room.
        if runtime.is_tibiadrome(actor) {
            return false;
        }
        let Ok(live) = self.states.tibiadrome.live.lock() else {
            return false;
        };
        for cell in cells {
            for current in live.values().filter(|l| !l.ended) {
                let Ok(battle) = current.battle.lock() else {
                    return false;
                };
                if !battle.ended
                    && cell.floor == battle.center.floor
                    && cell.x.abs_diff(battle.center.x) <= 6
                    && cell.y.abs_diff(battle.center.y) <= 6
                {
                    return false;
                }
            }
        }
        true
    }
}
