//! Wild-monster dispatch: source-qualified melee and one-pass summon composition.
//! Uses the actual positioned census, bound Creature policies and existing AI-4
//! owner. Summon activation requires current map/Item facts; ranged/defensive/custom
//! Ability execution remains uncomposed.
use std::collections::BTreeMap;

use super::ComposedFreshAdmission;
use super::actor_spell::ChannelSpellStates;
use super::attack::ChannelAttackStates;
use crate::ability::creature_bite::ReentryProtection;
use crate::ai::{AiProvenance, AiProvenanceInput, ResourceLimit};
use crate::ai_monster_melee::{Dispatch, MeleeDefinition, think_target_only};
use crate::ai_think::targeting::TargetCandidate;
use crate::ai_think::{
    AttackReadiness, CreatureAiTable, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId,
    ThinkReport,
};
use crate::combat::charm_effects::CharmHookEvent;
use crate::content::QualifiedNativeEntryRoom;
use crate::content::{
    LogicalCell, ProjectV2AuthoringProfileData, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
};
use crate::durability::spell_item_transaction as item_tx;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot, RuntimeWorkStamp,
};
use crate::monster_summon::{NativeSummonOwner, SummonLocationPolicy};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One coalesced creature think pass. The same Channel's scheduler is shared by
    /// all connections; player count never multiplies monster attack frequency.
    pub(in crate::gameplay_transport) async fn drain_monster_melee(&self) {
        let now = self.owner_now();
        // SPAWN-1a remains independent of the Item lane and of playable content gates.
        {
            let mut runtime = self.runtime.lock().await;
            drive_owner_spawns(&mut runtime, now.get());
        }
        // BUNDLE-MONSTER-AI-1: the pass reads the combat room, the entry room on the fixture
        // World and its bundle view (bundle cells and map revision) on a bundle World.
        let Some(room) = self.rooms.gate_monster_ai() else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        // Current Item reads use the existing lane before any long-lived Channel guard.
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || content.source_digest() != runtime.content_pin().server_artifact_digest()
            || room.compiled().server_digest() != runtime.content_pin().server_artifact_digest()
        {
            return;
        }
        let mut states = self.spell_states.lock().await;
        if now.get() < states.next_monster_ai_pass_us {
            return;
        }
        let Some(due) = now
            .get()
            .checked_add(crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS * 1_000)
        else {
            return;
        };
        let Some(sequence) = states.monster_ai_sequence.checked_add(1) else {
            return;
        };
        // SPAWN-CEILINGS-1 (Codex 4242381774): no Channel-wide actor gate. The census holds at
        // most the carrier's slots, and the work below is budgeted per pass instead, so a Channel
        // past the old `AI01-ACTIVE-ACTORS` 256 keeps thinking and swinging.
        // Codex 4243397472: only a swing pass (one per think interval) takes the census and
        // admits, forgets and wakes creatures. A pass between them only runs thinks left due by
        // `CREATUREAI0-RL-05`, against the last swing pass's players at their current positions,
        // so a think backlog costs each window its thinks, not a census of every actor.
        let swing_pass = now.get() >= states.monster_melee.next_swing_pass_us;
        let census = if swing_pass {
            let Ok(census) = runtime.positioned_melee_census() else {
                return;
            };
            census
        } else {
            think_player_census(&runtime, &states.monster_melee.think_players)
        };
        // Shared spell/map lock order: lane, runtime, spell states, objects, attack.
        // Keep the actual object owner through the current Item transaction and its consumer.
        let objects = self.door.lock().await;
        let mut attack = self.attack.lock().await;
        let semantic_now =
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get());
        let targets = melee_targets(room, &runtime, &states, &attack, &census, now.get());
        let monsters = census
            .iter()
            .filter(|(_, _, session)| session.is_none())
            .collect::<Vec<_>>();
        if swing_pass && monsters.is_empty() {
            return;
        }
        states.next_monster_ai_pass_us = due;
        states.monster_ai_sequence = sequence;
        let mut melee_owner = std::mem::take(&mut states.monster_melee);
        let digest = content
            .source_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let Ok(revisions) = crate::ability::RevisionSet::new(
            "ruleset:ai-v1",
            &format!("content:{digest}"),
            "world:ai-v1",
            "formula:source-melee-baseline",
            "simulation:v1",
        ) else {
            states.monster_melee = melee_owner;
            return;
        };
        if swing_pass {
            melee_owner.think_players = census
                .iter()
                .filter_map(|(actor, _, session)| session.map(|session| (*actor, session)))
                .collect();
            // CREATURE-AI-1 §2.1: every qualified monster is admitted to the think table, which
            // decides its target; the swing below only swings at that target. A monster already
            // admitted is not qualified again.
            for (actor, ..) in &monsters {
                if melee_owner.creature_ai.state(*actor).is_some() {
                    continue;
                }
                if qualified_melee(&runtime, content, *actor).is_none() {
                    continue;
                }
                let _ = admit_native_creature_ai(
                    &runtime,
                    &mut melee_owner.creature_ai,
                    content,
                    *actor,
                    now.get(),
                );
            }
            // An idle creature that died or despawned has no due think to remove it.
            melee_owner.creature_ai.forget_dead(&runtime);
        }
        if melee_owner.creature_ai.is_empty() {
            states.monster_melee = melee_owner;
            return;
        }
        if swing_pass {
            // §2.1 item 1: players wake the idle creatures they see; a creature already thinking
            // is unaffected. One indexed pass for all players (Codex 4242808144): its work does
            // not grow with players times creatures.
            let players = targets
                .iter()
                .map(|(player, ..)| *player)
                .collect::<Vec<_>>();
            let _ = melee_owner
                .creature_ai
                .wake_for_players(&runtime, &players, now.get());
        }
        // §2.1 item 3: protected players are ineligible, and invisible ones to a creature that
        // does not sense invisibility. The native artifact carries no player vitals or damage
        // contributors yet, so the strategies that read them see equal candidates.
        let candidates = targets
            .iter()
            .map(
                |(actor, position, _, invisible, protected)| TargetCandidate {
                    actor: *actor,
                    position: *position,
                    eligible: !*protected,
                    invisible: *invisible,
                    health: 0,
                    damage: 0,
                },
            )
            .collect::<Vec<_>>();
        // Restore the real Channel state before any fallible/cancellable SQL await. A
        // cancellation may defer work, but cannot drop an extracted think table or its memos.
        states.monster_melee = melee_owner;
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return;
        };
        let mut completed_next = None;
        let mut context = (
            self,
            &permit,
            &mut *runtime,
            &mut *states,
            &mut *attack,
            &*objects,
            room,
            content,
            &revisions,
            &candidates,
            &mut completed_next,
        );
        let _ = pass
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (
                        owner,
                        permit,
                        runtime,
                        states,
                        attack,
                        objects,
                        room,
                        content,
                        revisions,
                        candidates,
                        completed_next,
                    ) = context;
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let binding = runtime.binding();
                    let authority = item_tx::assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        crate::foundation::RuntimeScopeRefV1::channel(
                            binding.world_id(),
                            binding.channel_id(),
                        ),
                        binding.scope_generation().get(),
                    )
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    let player_waiting = attack.player_chase_waiting(now);
                    let (ai, summons) = (
                        &mut states.monster_melee.creature_ai,
                        &mut states.native_summons,
                    );
                    let (_, next) = run_current_native_monster_thinks(
                        runtime,
                        ai,
                        summons,
                        content,
                        now.get(),
                        &GameplayDecisionRoot::from_bytes(content.source_digest()),
                        revisions,
                        candidates,
                        room,
                        objects,
                        &mut tx,
                        &authority,
                        attack.chase_search_window(),
                        player_waiting,
                    )
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    // Native changes and replay receipts are complete before cleanup can suspend.
                    // This transaction only read Item state: it needs no durable COMMIT verdict.
                    **completed_next = Some(next);
                    tx.rollback()
                        .await
                        .map_err(crate::durability::DurabilityError::from)
                })
            })
            .await;
        let next_think =
            completed_next.unwrap_or_else(|| states.monster_melee.creature_ai.earliest_due());
        let mut melee_owner = std::mem::take(&mut states.monster_melee);
        // CREATUREAI0-RL-05 (Codex 4243189401): thinks left due past this window's budget, or
        // due before the next interval pass, run in a pass of their own as soon as the budget
        // allows, not one think interval later. Such a pass only thinks: swings keep their one
        // pass per think interval.
        if swing_pass {
            melee_owner.next_swing_pass_us = due;
        }
        let next_pass =
            next_monster_ai_pass_us(now.get(), melee_owner.next_swing_pass_us, next_think);
        // §2.1: only a monster whose think holds a target swings, in census order from the
        // cursor and at most `CREATUREAI0-RL-05` per pass. A creature that stepped swings from,
        // and is perceived at, its new cell.
        let swinging = monsters
            .iter()
            .filter(|(actor, ..)| {
                swing_pass
                    && melee_owner
                        .creature_ai
                        .state(*actor)
                        .and_then(|state| state.target())
                        .is_some()
            })
            .map(|(actor, position, _)| {
                (
                    *actor,
                    runtime.read_actor_position(*actor).unwrap_or(*position),
                )
            })
            .collect::<Vec<_>>();
        let (cursor, count) = swing_window(swinging.len(), states.monster_ai_cursor);
        for offset in 0..count {
            let (actor, position) = &swinging[(cursor + offset) % swinging.len()];
            let Some(monster) = qualified_melee(&runtime, content, *actor) else {
                continue;
            };
            let p = position.position();
            let Ok(tile) = room.movement_cells().spell_tiles().lookup(
                room.movement_cells().scope(),
                LogicalCell {
                    x: p.x,
                    y: p.y,
                    z: i32::from(p.floor),
                },
            ) else {
                continue;
            };
            if tile.flags().protection_zone {
                continue;
            }
            // Perception's limit applies to this monster's eligible local
            // candidates, not every player elsewhere in the Channel. Protected
            // players cannot occupy the nearest-candidate slot.
            let Some(indices) = local_target_indices(
                p,
                monster.behavior.targeting.sense_invisible,
                targets
                    .iter()
                    .map(|(_, p, _, invisible, protected)| (*p, *invisible, *protected)),
            ) else {
                continue;
            };
            // CREATURE-AI-1 §2.1: the swing goes only at the target the think decided.
            let perceived = think_target_only(
                melee_owner
                    .creature_ai
                    .state(*actor)
                    .and_then(|state| state.target()),
                indices.iter().map(|index| {
                    (
                        targets[*index].0,
                        PerceivedPlayer {
                            id: PerceivedPlayerId::new(*index as u64 + 1),
                            position: targets[*index].1,
                            legal_attack_target: true,
                        },
                    )
                }),
            );
            let mut root = content.source_digest();
            let placement = actor.placement_identity();
            for (byte, actor_byte) in root.iter_mut().zip(placement) {
                *byte ^= actor_byte;
            }
            let mut occurrence = placement;
            for (byte, seq_byte) in occurrence.iter_mut().zip(sequence.to_be_bytes()) {
                *byte ^= seq_byte;
            }
            // Numeric provenance markers hash actual opaque identities/revisions;
            // they are local deterministic snapshot keys, never canonical IDs or grants.
            let marker = |bytes: &[u8]| {
                let mut value = [0; 8];
                value.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(value)
            };
            let input = CreatureThinkInput {
                provenance: AiProvenance::new(AiProvenanceInput {
                    scope_id: marker(runtime.binding().channel_id().as_bytes()),
                    scope_generation: runtime.binding().scope_generation().get(),
                    actor_generation: marker(&placement),
                    behavior_revision: marker(&content.source_digest()),
                    content_revision: marker(&content.source_digest()),
                    navigation_revision: marker(&room.map_revision_digest()),
                    ruleset_revision: 1,
                    determinism_profile_revision: 1,
                }),
                decision_root: GameplayDecisionRoot::from_bytes(root),
                occurrence: DecisionOccurrenceId::from_bytes(occurrence),
                path_work_id: sequence,
                position: p,
                home: p,
                attack: AttackReadiness {
                    off_cooldown: false,
                    chance_percent: 0,
                },
            };
            let decision_root = GameplayDecisionRoot::from_bytes(root);
            let decision_occurrence = DecisionOccurrenceId::from_bytes(occurrence);
            let race_key = &monster.race_key;
            // ATTACK-1b: the player this think resolved, for the incoming hooks below.
            let attacked = std::cell::Cell::new(None);
            let dispatch = melee_owner.think(
                &mut runtime,
                &mut *states,
                *actor,
                sequence,
                monster.definition,
                input,
                &perceived,
                |id| {
                    let index = usize::try_from(id.get().checked_sub(1)?).ok()?;
                    let (actor, _, session, _, protected) = *targets.get(index)?;
                    if protected {
                        return None;
                    }
                    attacked.set(Some((actor, session)));
                    // The held runtime owner and exact current protection
                    // read above remain live through the synchronous bite commit.
                    Some((
                        actor,
                        session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                revisions.clone(),
                SemanticTimeMicros::from_micros(now.get()),
            );
            // ATTACK-0 §4: a creature attack on a player runs the incoming Charm hooks, and a
            // landed attack starts the player's in-fight deadline. A replayed think returns its
            // recorded dispatch without resolving a target again, so nothing runs twice.
            let (Some((player, session)), Ok(dispatch)) = (attacked.get(), dispatch) else {
                continue;
            };
            let base_damage = match dispatch {
                Dispatch::Bite(Ok(applied)) => Some(u64::from(applied.requested)),
                Dispatch::ZeroDamage => None,
                Dispatch::Bite(Err(_)) | Dispatch::Decision(_) => continue,
            };
            if let Ok(lease) = runtime.bound_attacker_lease(player, session) {
                let character = *lease.character_id().as_bytes();
                super::attack::run_charm_hook(
                    self.imported_charms,
                    character,
                    race_key,
                    &decision_root,
                    decision_occurrence,
                    CharmHookEvent::IncomingCreatureAttack,
                );
                if let Some(base_damage) = base_damage {
                    super::attack::run_charm_hook(
                        self.imported_charms,
                        character,
                        race_key,
                        &decision_root,
                        decision_occurrence,
                        CharmHookEvent::IncomingCreatureHit { base_damage },
                    );
                }
            }
            attack.record_hit_taken(&runtime, player, session, semantic_now);
        }
        states.monster_melee = melee_owner;
        states.next_monster_ai_pass_us = next_pass;
        if !swinging.is_empty() {
            states.monster_ai_cursor = (cursor + count) % swinging.len();
        }
    }
}

/// SPAWN-CEILINGS-1 (Codex 4243397472): the census of a pass between swing passes: the last swing
/// pass's `players` whose position still reads, in that census's order. Such a pass only runs
/// thinks, so it needs no creature rows; `melee_targets` still checks each player's session,
/// vitals, invisibility and protection now. It reads one position per player, not every actor.
fn think_player_census(
    runtime: &ChannelRuntimeV1,
    players: &[(ExactActorRef, GameSessionId)],
) -> Vec<(
    ExactActorRef,
    MovementPositionSnapshot,
    Option<GameSessionId>,
)> {
    players
        .iter()
        .filter_map(|(actor, session)| {
            let position = runtime.read_actor_position(*actor).ok()?;
            Some((*actor, position, Some(*session)))
        })
        .collect()
}

/// SPAWN-CEILINGS-1 (Codex 4243189401): when the coalesced pass after one at `now_us` runs. A
/// pass runs at the next swing pass (`swing_due_us`, one `CREATURE_THINK_INTERVAL_MILLIS` after
/// the last). When a think falls due sooner (`earliest_think_us`), the next pass runs at that
/// time, and a think still due now, left over by the window's `CREATUREAI0-RL-05` budget, runs
/// at the start of the next `CREATUREAI0_THINK_WINDOW_MICROS` window, whose budget is fresh. So a
/// backlog of N due thinks takes ceil(N / 1,024) owner windows, not ceil(N / 1,024) think
/// intervals, and a creature whose think was deferred is not delayed again by the pass cadence.
fn next_monster_ai_pass_us(now_us: u64, swing_due_us: u64, earliest_think_us: Option<u64>) -> u64 {
    let Some(earliest) = earliest_think_us else {
        return swing_due_us;
    };
    let window = crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS;
    let next_window = (now_us / window).saturating_add(1).saturating_mul(window);
    earliest.max(next_window).min(swing_due_us)
}

/// SPAWN-CEILINGS-1 (Codex 4242381774): the swings of one pass over the `len` monsters whose
/// think holds a target, as (first index, count). It replaces `AI01-EVALUATION-WORK` 8, which
/// left a Channel of hundreds of targeting monsters swinging once in dozens of passes: a pass
/// swings every one of them up to `CREATUREAI0-RL-05`, and above it the cursor serves the rest
/// first in the next passes. A swing is one evaluation of at most `CREATUREAI0-RL-06` units, and
/// a pass swings once per `CREATURE_THINK_INTERVAL_MILLIS` (20 owner windows; the passes between
/// only run thinks left due, Codex 4243189401), so the swings stay within one window's think
/// budget.
fn swing_window(len: usize, cursor: usize) -> (usize, usize) {
    if len == 0 {
        return (0, 0);
    }
    (
        cursor % len,
        len.min(crate::ai_think::CREATUREAI0_THINKS_PER_WINDOW),
    )
}

/// Admit into the one native think table. Melee still belongs to the existing swing owner;
/// source Ability execution stays disabled, while the same think retains authored summons.
fn admit_native_creature_ai(
    runtime: &ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    content: &crate::content::native_gameplay::NativeGameplayState,
    actor: ExactActorRef,
    now_us: u64,
) -> Result<(), CarrierError> {
    let record = current_native_creature_record(runtime, content, actor)
        .ok_or(CarrierError::ProfileMissing)?;
    let Some(ProjectV2AuthoringProfileData::Behavior(behavior)) =
        record.behavior.as_ref().map(|profile| &profile.data)
    else {
        return Err(CarrierError::ProfileMissing);
    };
    if !behavior.targeting.hostile || !behavior.targeting.can_target {
        return Err(CarrierError::ProfileInvalid);
    }
    let mut projected = behavior.clone();
    projected.attacks.clear();
    projected.defenses.clear();
    let speed =
        crate::movement::speed::NativeCreatureSpeed::from_current_native(runtime, content, actor);
    ai.admit_creature_with_summons(runtime, actor, Some(&projected), &BTreeMap::new(), now_us)?;
    if let Some(speed) = speed
        && let Err(error) = ai.bind_native_movement(runtime, actor, speed, None)
    {
        ai.remove(actor);
        return Err(error);
    }
    Ok(())
}

/// The installed policy carries the exact source revision; an actor key alone is insufficient.
fn current_native_creature_record<'a>(
    runtime: &ChannelRuntimeV1,
    content: &'a crate::content::native_gameplay::NativeGameplayState,
    actor: ExactActorRef,
) -> Option<&'a crate::content::native_gameplay::CreatureProfileRecord> {
    if content.source_digest() != runtime.content_pin().server_artifact_digest() {
        return None;
    }
    let snapshot = runtime.companion_snapshot(actor).ok()?;
    if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
        return None;
    }
    content.creature_profiles().records.iter().find(|record| {
        record.profile.target.key == snapshot.state.policy.definition_key
            && record.profile.target.revision == snapshot.state.policy.definition_revision
    })
}

type NativeThinkPass = (Vec<(ExactActorRef, ThinkReport)>, Option<u64>);

/// One transaction's current Map/Item read. The transaction and LocalObject owner remain
/// borrowed through the synchronous consumer; these facts cannot outlive their locks.
/// Only cells actually read in this transaction enter the bounded per-creature table.
struct NativeSummonMap<'read, 'connection, 'owner> {
    tx: &'read mut sqlx::Transaction<'connection, sqlx::Postgres>,
    authority: &'read item_tx::SpellItemScopeAuthority,
    room: &'owner QualifiedNativeEntryRoom,
    objects: &'owner crate::world_runtime::LocalObjectRuntime,
    stamp: RuntimeWorkStamp,
    tiles: BTreeMap<LogicalCell, Option<CurrentNativeTile>>,
    path: Option<CurrentSummonPath>,
}

/// Borrow the actual Map owner for one search; the search's read future is explicitly Send.
struct NativeCurrentPathQuery<'query, 'read, 'connection, 'owner> {
    map: &'query mut NativeSummonMap<'read, 'connection, 'owner>,
    runtime: &'query ChannelRuntimeV1,
    sight_target: Option<MovementLocalPosition>,
}

impl crate::movement::path_search::CurrentPathQuery for NativeCurrentPathQuery<'_, '_, '_, '_> {
    async fn query(&mut self, position: MovementLocalPosition, goal: bool) -> Option<(u32, bool)> {
        if self.map.read_tile(self.runtime, position).await != Some(true) {
            return None;
        }
        let goal_admits = if goal && let Some(target) = self.sight_target {
            self.map.sight_clear(self.runtime, position, target).await == Some(true)
        } else {
            true
        };
        Some((
            if self.runtime.native_summon_cell_free(position) {
                0
            } else {
                crate::movement::path_search::CREATURE_COST
            },
            goal_admits,
        ))
    }
}

#[derive(Clone, Copy)]
struct CurrentNativeTile {
    walkable: bool,
    projectile_clear: bool,
    ground_speed: u16,
}

struct CurrentSummonPath {
    parent: ExactActorRef,
    target: ExactActorRef,
    parent_position: MovementPositionSnapshot,
    target_position: MovementPositionSnapshot,
    result: Option<bool>,
    steps: Vec<crate::movement::CardinalStep>,
}

fn logical_cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}

fn path_destination(
    position: MovementLocalPosition,
    step: crate::movement::CardinalStep,
) -> Option<MovementLocalPosition> {
    let (dx, dy) = crate::movement::path_search::delta(step);
    Some(MovementLocalPosition {
        x: position.x.checked_add(dx)?,
        y: position.y.checked_add(dy)?,
        floor: position.floor,
    })
}

impl NativeSummonMap<'_, '_, '_> {
    fn current(&self, runtime: &ChannelRuntimeV1, stamp: RuntimeWorkStamp) -> bool {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        stamp == self.stamp
            && runtime
                .owner_fence()
                .is_ok_and(|owner| owner.accepts_stamp(stamp))
            && self.authority.world_bytes() == *binding.world_id().as_bytes()
            && self.authority.channel_bytes() == *binding.channel_id().as_bytes()
            && self.authority.generation() == binding.scope_generation().get()
            && self.room.movement_cells().scope().world_id == pin.world_id()
            && self.room.compiled().server_digest() == pin.server_artifact_digest()
            && self.room.map_revision_digest() == pin.map_revision_digest()
            && self.room.frame_binding().digest() == pin.frame_binding_digest()
    }

    async fn read_tile(
        &mut self,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Option<bool> {
        self.read_tile_facts(runtime, position)
            .await
            .map(|tile| tile.walkable)
    }

    async fn read_tile_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Option<CurrentNativeTile> {
        if !self.current(runtime, self.stamp) {
            return None;
        }
        let cell = logical_cell(position);
        if let Some(admits) = self.tiles.get(&cell) {
            return *admits;
        }
        let cells = self.room.movement_cells();
        let entry_door = self.room.source_world().is_none()
            && !bundle_cells(cells)
            && self.objects.collision_cells().contains(&cell);
        let observed = async {
            // Known absent/blocked cells refuse without an Item read. Unknown source cells
            // still go through the actual initialization/Item owner below.
            if entry_door
                || cells.index().lookup(cells.scope(), cell)
                    == Ok(crate::content::CollisionClass::Blocked)
                || cells.spell_tiles().lookup(cells.scope(), cell).err()
                    == Some(crate::content::SpellTileLookupError::Absent)
            {
                return Some(CurrentNativeTile {
                    walkable: false,
                    projectile_clear: false,
                    ground_speed: 0,
                });
            }
            let terrain =
                crate::spell::world_execution::qualified_creature_source_tile_in_transaction(
                    self.tx,
                    self.authority,
                    self.room,
                    runtime,
                    self.objects,
                    position,
                )
                .await
                .ok()?;
            if !terrain.matches_runtime(runtime) || terrain.position() != position {
                return None;
            }
            let mut tile = CurrentNativeTile {
                walkable: terrain.ground_present()
                    && !terrain.block_solid()
                    && !terrain.floor_change()
                    && !terrain.protection_zone()
                    && !terrain.qualified_tile().flags().immovable_block_solid
                    && !crate::map::floor::is_teleport(self.room.movement_cells(), cell),
                projectile_clear: !terrain.block_projectile(),
                ground_speed: crate::movement::speed::qualified_ground_speed(
                    terrain.ground_present(),
                    terrain.qualified_tile().ground_speed(),
                ),
            };
            let address =
                crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
                    self.room, runtime, position,
                )
                .ok()?;
            let items =
                item_tx::read_creature_scope_tile_in_transaction(self.tx, self.authority, &address)
                    .await
                    .ok()?;
            tile.walkable &= !items
                .items
                .iter()
                .any(|item| item.blocks_movement || item.immovable_block_solid);
            tile.projectile_clear &= !items.items.iter().any(|item| item.blocks_projectile);
            Some(tile)
        }
        .await;
        self.tiles.insert(cell, observed);
        observed
    }

    /// Charge the existing shared player/creature window before the one search, including a
    /// failed search. Revalidation later in this same transaction never searches or charges.
    async fn prepare_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        target: ExactActorRef,
        budget: &mut crate::combat_attack::ChaseSearchBudget,
        now_us: u64,
        player_waiting: bool,
    ) -> Option<bool> {
        if self.path.is_some() {
            return self.current_target_reachable(runtime, parent, target, self.stamp);
        }
        if !self.current(runtime, self.stamp) {
            return None;
        }
        let parent_position = runtime.read_actor_position(parent).ok()?;
        let target_position = runtime.read_actor_position(target).ok()?;
        if parent_position.context() != target_position.context() {
            return None;
        }
        self.path = Some(CurrentSummonPath {
            parent,
            target,
            parent_position,
            target_position,
            result: None,
            steps: Vec::new(),
        });
        if !budget.charge_creature(
            now_us,
            crate::combat_attack::UNITS_PER_SEARCH,
            player_waiting,
        ) {
            return None;
        }
        let endpoints = self
            .read_tile(runtime, parent_position.position())
            .await
            .zip(self.read_tile(runtime, target_position.position()).await);
        let result = match endpoints {
            Some((true, true)) => {
                let steps = {
                    let mut query = NativeCurrentPathQuery {
                        map: &mut *self,
                        runtime,
                        sight_target: None,
                    };
                    crate::movement::path_search::route_next_to_current(
                        parent_position.position(),
                        target_position.position(),
                        &mut query,
                    )
                    .await
                };
                if let Some(steps) = steps {
                    self.path.as_mut()?.steps = steps;
                    Some(true)
                } else if self.tiles.values().any(Option::is_none) {
                    None
                } else {
                    Some(false)
                }
            }
            Some(_) => Some(false),
            None => None,
        };
        self.path.as_mut()?.result = result;
        self.current_target_reachable(runtime, parent, target, self.stamp)
    }

    async fn read_placement_cells(
        &mut self,
        runtime: &ChannelRuntimeV1,
        center: MovementLocalPosition,
    ) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (Some(x), Some(y)) = (center.x.checked_add(dx), center.y.checked_add(dy))
                else {
                    continue;
                };
                let _ = self
                    .read_tile(
                        runtime,
                        MovementLocalPosition {
                            x,
                            y,
                            floor: center.floor,
                        },
                    )
                    .await;
            }
        }
    }

    async fn sight_clear(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> Option<bool> {
        if !self.current(runtime, self.stamp) {
            return None;
        }
        for (at, exempt) in crate::spell::world_execution::source_sight_steps(from, to).ok()? {
            if !exempt && !self.read_tile_facts(runtime, at).await?.projectile_clear {
                return Some(false);
            }
        }
        Some(true)
    }

    fn current_step_admits(
        &self,
        runtime: &ChannelRuntimeV1,
        expected: MovementPositionSnapshot,
        destination: MovementLocalPosition,
    ) -> Option<bool> {
        if !self.current(runtime, self.stamp)
            || expected.context() != runtime.pinned_movement_context()
        {
            return None;
        }
        self.tiles
            .get(&logical_cell(destination))
            .copied()
            .flatten()
            .map(|tile| tile.walkable)
    }

    /// A retained route is not retained Map authority. Re-read the actual suffix in this
    /// transaction, optionally extending it by a bounded direct cardinal suffix to adjacency.
    /// This is path validation, not another search; inability to prove the suffix refuses.
    async fn prove_retained_target_path(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        target: ExactActorRef,
        mut steps: Vec<crate::movement::CardinalStep>,
    ) -> Option<bool> {
        let parent_position = runtime.read_actor_position(parent).ok()?;
        let target_position = runtime.read_actor_position(target).ok()?;
        if parent_position.context() != target_position.context()
            || parent_position.position().floor != target_position.position().floor
        {
            return None;
        }
        let mut at = parent_position.position();
        let mut known = self.read_tile(runtime, at).await?;
        for &direction in &steps {
            at = path_destination(at, direction)?;
            known &= self.read_tile(runtime, at).await?;
        }
        while crate::movement::path_search::chebyshev(at, target_position.position()) > 1 {
            if steps.len() >= crate::movement::path_search::ROUTE_STEPS {
                return None;
            }
            let target = target_position.position();
            let direction = if at.x < target.x {
                crate::movement::CardinalStep::East
            } else if at.x > target.x {
                crate::movement::CardinalStep::West
            } else if at.y < target.y {
                crate::movement::CardinalStep::South
            } else {
                crate::movement::CardinalStep::North
            };
            at = path_destination(at, direction)?;
            known &= self.read_tile(runtime, at).await?;
            steps.push(direction);
        }
        known &= self.read_tile(runtime, target_position.position()).await?;
        known &= crate::movement::path_search::chebyshev(at, target_position.position()) == 1;
        self.path = Some(CurrentSummonPath {
            parent,
            target,
            parent_position,
            target_position,
            result: Some(known),
            steps,
        });
        self.current_target_reachable(runtime, parent, target, self.stamp)
    }

    /// Local keep-distance / flee candidates use the same current admission as an actual
    /// step. `Some(None)` holds at the goal; `None` asks the think's one bounded path request.
    async fn local_distance_step(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        target: ExactActorRef,
        distance: u16,
        fleeing: bool,
    ) -> Option<Option<crate::movement::CardinalStep>> {
        use crate::movement::CardinalStep::{East, North, South, West};
        let at = runtime.read_actor_position(actor).ok()?.position();
        let target = runtime.read_actor_position(target).ok()?.position();
        if at.floor != target.floor {
            return None;
        }
        let current = crate::movement::path_search::chebyshev(at, target);
        if fleeing && current >= 11 {
            return Some(None);
        }
        if !fleeing && self.sight_clear(runtime, at, target).await != Some(true) {
            return None;
        }
        if !fleeing && current == i64::from(distance) {
            return Some(None);
        }
        let mut sideways = None;
        let mut best = None;
        let mut best_distance = current;
        for direction in [North, East, South, West] {
            let next = path_destination(at, direction)?;
            if self.read_tile(runtime, next).await != Some(true)
                || runtime.position_occupied_by_other(actor, next) != Ok(false)
            {
                continue;
            }
            let away = crate::movement::path_search::chebyshev(next, target);
            if fleeing && away == current && sideways.is_none() {
                sideways = Some(direction);
            }
            let improves = if fleeing || current < i64::from(distance) {
                away > best_distance && (fleeing || away <= i64::from(distance))
            } else {
                away < best_distance && away >= i64::from(distance)
            };
            if improves && (fleeing || self.sight_clear(runtime, next, target).await == Some(true))
            {
                best = Some(direction);
                best_distance = away;
            }
        }
        if fleeing {
            Some(best.or(sideways))
        } else {
            best.map(Some)
        }
    }
}

impl crate::movement::speed::GroundSpeedSource for NativeSummonMap<'_, '_, '_> {
    fn ground_speed(&self, cell: LogicalCell) -> u16 {
        self.tiles
            .get(&cell)
            .copied()
            .flatten()
            .filter(|tile| tile.walkable)
            .map_or(0, |tile| tile.ground_speed)
    }
}

impl SummonLocationPolicy for NativeSummonMap<'_, '_, '_> {
    fn current_target_reachable(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        if !self.current(runtime, stamp) {
            return None;
        }
        let path = self.path.as_ref()?;
        if path.parent != parent
            || path.target != target
            || runtime.read_actor_position(parent).ok()? != path.parent_position
            || runtime.read_actor_position(target).ok()? != path.target_position
        {
            return None;
        }
        if path.result != Some(true) {
            return path.result;
        }
        let mut position = path.parent_position.position();
        if self
            .tiles
            .get(&logical_cell(position))
            .copied()
            .flatten()
            .map(|tile| tile.walkable)
            != Some(true)
        {
            return None;
        }
        for &step in &path.steps {
            position = path_destination(position, step)?;
            if self
                .tiles
                .get(&logical_cell(position))
                .copied()
                .flatten()
                .map(|tile| tile.walkable)
                != Some(true)
            {
                return None;
            }
        }
        Some(
            crate::movement::path_search::chebyshev(position, path.target_position.position()) == 1
                && self
                    .tiles
                    .get(&logical_cell(path.target_position.position()))
                    .copied()
                    .flatten()
                    .map(|tile| tile.walkable)
                    == Some(true),
        )
    }

    fn current_cell_admits(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        _child: &ProjectV2DefinitionRef,
        position: MovementLocalPosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        if !self.current(runtime, stamp) {
            return None;
        }
        let target = self.path.as_ref()?.target;
        if self.current_target_reachable(runtime, parent, target, stamp) != Some(true) {
            return None;
        }
        self.tiles
            .get(&logical_cell(position))
            .copied()
            .flatten()
            .map(|tile| tile.walkable)
    }
}

struct PreparedNativeMovement {
    goal: crate::ai_think::NativeCreatureGoal,
    goal_sequence: u64,
    request_sequence: u64,
    path: Option<crate::ai_think::NativeCreaturePath>,
    search_pending: bool,
    searched: bool,
}

/// One admitted goal owns at most one pending request. Revalidation uses no search work;
/// a failed search waits for the next think and a budget deferral keeps this same request.
#[allow(clippy::too_many_arguments)]
async fn prepare_native_movement_path(
    map: &mut NativeSummonMap<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    goal: crate::ai_think::NativeCreatureGoal,
    goal_sequence: u64,
    request_sequence: u64,
    retained: Option<crate::ai_think::NativeCreaturePath>,
    revisions: &crate::ability::RevisionSet,
    budget: &mut crate::combat_attack::ChaseSearchBudget,
    now_us: u64,
    player_waiting: bool,
) -> PreparedNativeMovement {
    use crate::ai_think::NativeCreatureGoal as G;
    let mut prepared = PreparedNativeMovement {
        goal,
        goal_sequence,
        request_sequence,
        path: None,
        search_pending: false,
        searched: false,
    };
    let Ok(from) = runtime.read_actor_position(actor) else {
        prepared.goal = G::Stand;
        return prepared;
    };
    if let Some(path) =
        retained.filter(|path| path.current(runtime, actor, goal, goal_sequence, revisions))
    {
        if let Some(target) = goal.target() {
            let steps = path.steps().collect::<Vec<_>>();
            let _ = map
                .prove_retained_target_path(runtime, actor, target, steps)
                .await;
        }
        prepared.path = Some(path);
        return prepared;
    }
    let (target_position, target_snapshot, distance) = match goal {
        G::Chase { target, distance } => {
            let Ok(target_at) = runtime.read_actor_position(target) else {
                prepared.goal = G::Stand;
                return prepared;
            };
            if distance > 1
                && map
                    .local_distance_step(runtime, actor, target, distance, false)
                    .await
                    .is_some()
            {
                return prepared;
            }
            (target_at.position(), Some(target_at), Some(distance))
        }
        G::WalkBack { home } if from.position() != home => (home, None, None),
        G::WalkBack { .. } => {
            prepared.goal = G::Stand;
            return prepared;
        }
        _ => return prepared,
    };
    if !budget.charge_creature(
        now_us,
        crate::combat_attack::UNITS_PER_SEARCH,
        player_waiting,
    ) {
        prepared.search_pending = true;
        return prepared;
    }
    prepared.searched = true;
    if map.read_tile(runtime, from.position()).await != Some(true)
        || map.read_tile(runtime, target_position).await != Some(true)
    {
        prepared.goal = G::Stand;
        return prepared;
    }
    let steps: Option<Vec<crate::movement::CardinalStep>> = {
        let mut query = NativeCurrentPathQuery {
            map: &mut *map,
            runtime,
            sight_target: distance.map(|_| target_position),
        };
        match distance {
            Some(distance) => {
                crate::movement::path_search::route_at_distance_current(
                    from.position(),
                    target_position,
                    distance,
                    &mut query,
                )
                .await
            }
            None => {
                crate::movement::path_search::route_to_cell_current(
                    from.position(),
                    target_position,
                    &mut query,
                )
                .await
            }
        }
    };
    if let Some(steps) = steps {
        prepared.path = crate::ai_think::NativeCreaturePath::from_search(
            runtime,
            actor,
            goal,
            goal_sequence,
            request_sequence,
            target_snapshot,
            &steps,
            revisions,
        );
        if let Some(target) = goal.target() {
            let _ = map
                .prove_retained_target_path(runtime, actor, target, steps)
                .await;
        }
    } else {
        prepared.goal = G::Stand;
    }
    prepared
}

fn next_search_window(now_us: u64) -> u64 {
    (now_us / crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS)
        .saturating_add(1)
        .saturating_mul(crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS)
}

fn install_native_movement_goal(
    runtime: &ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    stamp: RuntimeWorkStamp,
    actor: ExactActorRef,
    prepared: PreparedNativeMovement,
    now_us: u64,
) {
    use crate::ai_think::NativeCreatureGoal as G;
    let changed = ai
        .native_movement(actor)
        .is_some_and(|movement| movement.goal != prepared.goal);
    if changed {
        ai.cancel_native_step(actor);
    }
    let Some(movement) = ai.native_movement_mut(actor) else {
        return;
    };
    movement.goal = prepared.goal;
    movement.goal_sequence = prepared.goal_sequence;
    movement.request_sequence = prepared.request_sequence;
    movement.path = prepared.path;
    movement.search_pending = prepared.search_pending;
    let due = if movement.search_pending {
        next_search_window(now_us)
    } else {
        movement
            .cadence
            .next_due()
            .map_or(now_us, |due| due.get().max(now_us))
    };
    let due = match movement.goal {
        G::Wander { interval_ms, .. } => {
            due.max(now_us.saturating_add(interval_ms.saturating_mul(1_000)))
        }
        _ => due,
    };
    if movement.goal == G::Stand {
        ai.cancel_native_step(actor);
    } else {
        let _ = ai.arm_native_step(runtime, stamp, actor, due);
    }
}

fn native_step_draw(
    root: &GameplayDecisionRoot,
    occurrence: crate::ai_think::ThinkOccurrence,
    purpose: &str,
    index: u64,
) -> Option<u64> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:native-creature-step:v1")
        .chain_update(occurrence.actor.placement_identity())
        .chain_update(occurrence.sequence.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    oteryn_simulation_determinism::deterministic_decision_u64(
        root,
        oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes),
        purpose,
        index,
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
async fn choose_native_goal_step(
    map: &mut NativeSummonMap<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    goal: crate::ai_think::NativeCreatureGoal,
    path: Option<&crate::ai_think::NativeCreaturePath>,
    occurrence: crate::ai_think::ThinkOccurrence,
    root: &GameplayDecisionRoot,
    static_attack_chance_ppm: u32,
) -> Option<Option<crate::movement::CardinalStep>> {
    use crate::ai_think::NativeCreatureGoal as G;
    use crate::movement::CardinalStep::{East, North, South, West};
    let at = runtime.read_actor_position(actor).ok()?.position();
    let step = match goal {
        G::Stand => return Some(None),
        G::Flee { target } => {
            return map
                .local_distance_step(runtime, actor, target, 11, true)
                .await;
        }
        G::Wander {
            anchor,
            radius_tiles,
            ..
        } => {
            let index =
                usize::try_from(native_step_draw(root, occurrence, "AI_WANDER", 0)? % 4).ok()?;
            let step = [North, East, South, West][index];
            let next = path_destination(at, step)?;
            if next.floor != anchor.floor
                || crate::movement::path_search::chebyshev(next, anchor) > i64::from(radius_tiles)
            {
                return Some(None);
            }
            step
        }
        G::WalkBack { home } => {
            if at == home {
                return Some(None);
            }
            path?.steps().next()?
        }
        G::Chase { target, distance } => {
            let target_at = runtime.read_actor_position(target).ok()?.position();
            if target_at.floor != at.floor {
                return None;
            }
            if distance == 1 && crate::movement::path_search::chebyshev(at, target_at) == 1 {
                if native_step_draw(root, occurrence, "AI_DANCE", 0)? % 1_000_000
                    < u64::from(static_attack_chance_ppm)
                {
                    return Some(None);
                }
                let mut candidates = Vec::new();
                for step in [North, East, South, West] {
                    let next = path_destination(at, step)?;
                    if crate::movement::path_search::chebyshev(next, target_at) == 1
                        && map.read_tile(runtime, next).await == Some(true)
                        && runtime.position_occupied_by_other(actor, next) == Ok(false)
                    {
                        candidates.push(step);
                    }
                }
                if candidates.is_empty() {
                    return Some(None);
                }
                let index = usize::try_from(
                    native_step_draw(root, occurrence, "AI_DANCE", 1)?
                        % u64::try_from(candidates.len()).ok()?,
                )
                .ok()?;
                return Some(Some(candidates[index]));
            }
            if distance > 1
                && let Some(local) = map
                    .local_distance_step(runtime, actor, target, distance, false)
                    .await
            {
                return Some(local);
            }
            path?.steps().next()?
        }
    };
    let next = path_destination(at, step)?;
    if map.read_tile(runtime, next).await != Some(true)
        || runtime.position_occupied_by_other(actor, next) != Ok(false)
    {
        return None;
    }
    Some(Some(step))
}

/// Pending owner timer inputs run before fresh thinks, so older carried search requests are
/// charged first. New goals arm this same lane and wait for its next bounded owner input.
/// No timer is removed before the current Map read; sync completion consumes it exactly once.
#[allow(clippy::too_many_arguments)]
async fn run_native_movement_steps(
    runtime: &mut ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    content: &crate::content::native_gameplay::NativeGameplayState,
    now_us: u64,
    root: &GameplayDecisionRoot,
    revisions: &crate::ability::RevisionSet,
    candidates: &[TargetCandidate],
    room: &QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &item_tx::SpellItemScopeAuthority,
    budget: &mut crate::combat_attack::ChaseSearchBudget,
    player_waiting: bool,
    stamp: RuntimeWorkStamp,
) -> Result<(), CarrierError> {
    use crate::ai_think::NativeCreatureGoal as G;
    for occurrence in ai.due_native_steps(runtime, now_us) {
        let actor = occurrence.actor;
        let Some(movement) = ai.native_movement(actor) else {
            continue;
        };
        let goal = movement.goal;
        let goal_sequence = movement.goal_sequence;
        let request_sequence = movement.request_sequence;
        let mut path = movement.path.clone();
        let search_pending = movement.search_pending;
        let mut near = movement.near;
        let before = runtime.read_actor_position(actor)?;
        let Some(record) = current_native_creature_record(runtime, content, actor) else {
            ai.idle_native_movement(actor, false);
            continue;
        };
        let Some(ProjectV2AuthoringProfileData::Behavior(behavior)) =
            record.behavior.as_ref().map(|profile| &profile.data)
        else {
            ai.idle_native_movement(actor, false);
            continue;
        };
        if let Some(target) = goal.target() {
            if !candidates.iter().any(|candidate| {
                candidate.actor == target
                    && runtime.read_actor_position(target).is_ok_and(|position| {
                        let actual = TargetCandidate {
                            position: position.position(),
                            ..*candidate
                        };
                        crate::ai_think::targeting::is_eligible(
                            &behavior.targeting,
                            before.position(),
                            &actual,
                        )
                    })
            }) {
                ai.idle_native_movement(actor, false);
                continue;
            }
            if let G::Chase { .. } = goal {
                near.evaluate(
                    before.position(),
                    runtime.read_actor_position(target)?.position(),
                );
            }
        }
        let mut map = NativeSummonMap {
            tx,
            authority,
            room,
            objects,
            stamp,
            tiles: BTreeMap::new(),
            path: None,
        };
        let mut searched = None;
        if search_pending {
            let prepared = prepare_native_movement_path(
                &mut map,
                runtime,
                actor,
                goal,
                goal_sequence,
                request_sequence,
                path,
                revisions,
                budget,
                now_us,
                player_waiting,
            )
            .await;
            path = prepared.path.clone();
            searched = Some(prepared);
        }
        let deferred = searched
            .as_ref()
            .is_some_and(|prepared| prepared.search_pending);
        let failed_search = searched
            .as_ref()
            .is_some_and(|prepared| prepared.goal == G::Stand);
        let route_current = path
            .as_ref()
            .is_none_or(|path| path.current(runtime, actor, goal, goal_sequence, revisions));
        let chosen = if deferred || failed_search || !route_current {
            None
        } else {
            choose_native_goal_step(
                &mut map,
                runtime,
                actor,
                goal,
                path.as_ref(),
                occurrence,
                root,
                behavior.targeting.static_attack_chance_ppm,
            )
            .await
        };
        // Cancellation before this point leaves the same pending input and all AI / pacing
        // state intact. From this actual-owner check through completion there is no await.
        if !map.current(runtime, stamp) {
            return Err(CarrierError::WrongScope);
        }
        if !ai.consume_native_step(runtime, stamp, occurrence) {
            continue;
        }
        if deferred {
            if let Some(prepared) = searched {
                install_native_movement_goal(runtime, ai, stamp, actor, prepared, now_us);
            }
            continue;
        }
        if failed_search || !route_current || chosen.is_none() {
            ai.idle_native_movement(actor, matches!(goal, G::WalkBack { .. }));
            continue;
        }
        let Some(direction) = chosen.flatten() else {
            if matches!(goal, G::WalkBack { .. }) {
                ai.idle_native_movement(actor, true);
            } else if let Some(movement) = ai.native_movement_mut(actor) {
                movement.near = near;
            }
            continue;
        };
        let Some(movement) = ai.native_movement_mut(actor) else {
            continue;
        };
        movement.near = near;
        movement.path = path;
        movement.search_pending = false;
        let outcome = movement.cadence.step_from_current_owner(
            runtime,
            stamp,
            &movement.source,
            &map,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now_us),
            room.movement_cells().scope(),
            direction,
            &movement.near,
            matches!(goal, G::Flee { .. }),
            |runtime, expected, next| map.current_step_admits(runtime, expected, next),
        );
        match outcome {
            Ok(Some(crate::foundation::NativeSummonMovementOutcome::Removed(child))) => {
                ai.remove(child);
            }
            Ok(Some(crate::foundation::NativeSummonMovementOutcome::Moved(after))) => {
                if let Some(movement) = ai.native_movement_mut(actor)
                    && movement
                        .path
                        .as_mut()
                        .is_some_and(|path| !path.moved(before, after))
                {
                    movement.path = None;
                }
                if matches!(goal, G::WalkBack { home } if after.position() == home) {
                    ai.idle_native_movement(actor, true);
                } else if let Some(movement) = ai.native_movement(actor) {
                    let mut due = movement.cadence.next_due().map_or(now_us, |due| due.get());
                    if let G::Wander { interval_ms, .. } = goal {
                        due = due.max(now_us.saturating_add(interval_ms.saturating_mul(1_000)));
                    }
                    let _ = ai.arm_native_step(runtime, stamp, actor, due);
                }
            }
            Ok(None) => {
                if let Some(due) = ai
                    .native_movement(actor)
                    .and_then(|movement| movement.cadence.next_due())
                {
                    let _ = ai.arm_native_step(runtime, stamp, actor, due.get());
                }
            }
            Err(_) => ai.idle_native_movement(actor, false),
        }
    }
    Ok(())
}

/// The actual current Map reader is held through think, Movement, admission and registration.
/// The AI entry stays in the Channel throughout every await. Each taken actor schedules once;
/// a newly admitted child waits for the next existing bounded owner pass.
#[allow(clippy::too_many_arguments)]
async fn run_current_native_monster_thinks(
    runtime: &mut ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    summons: &mut NativeSummonOwner,
    content: &crate::content::native_gameplay::NativeGameplayState,
    now_us: u64,
    root: &GameplayDecisionRoot,
    revisions: &crate::ability::RevisionSet,
    candidates: &[TargetCandidate],
    room: &QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &item_tx::SpellItemScopeAuthority,
    budget: &mut crate::combat_attack::ChaseSearchBudget,
    player_waiting: bool,
) -> Result<NativeThinkPass, CarrierError> {
    use crate::ai_think::{NativeCreatureGoal as G, profile_schedule::ScheduleError};
    let stamp = runtime.issue_owner_work()?;
    ai.forget_dead(runtime);
    // A carried step (including its pending search) has priority over fresh think requests.
    // New goals below only arm this same deadline lane; they never step in the think.
    run_native_movement_steps(
        runtime,
        ai,
        content,
        now_us,
        root,
        revisions,
        candidates,
        room,
        objects,
        tx,
        authority,
        budget,
        player_waiting,
        stamp,
    )
    .await?;
    let (taken, _) = ai.take_due_bounded(now_us);
    let homes = runtime
        .spawn_point_creatures()
        .into_iter()
        .filter_map(|(_, cell, live)| live.map(|actor| (actor.placement_identity(), (actor, cell))))
        .collect::<BTreeMap<_, _>>();
    let mut reports = Vec::new();
    for actor in taken {
        let (Some(health), Ok(before)) = (
            runtime.live_creature_health(actor),
            runtime.read_actor_position(actor),
        ) else {
            ai.remove(actor);
            continue;
        };
        let is_summon = runtime.native_summon_role(actor)?.is_some();
        let home = homes
            .get(&actor.placement_identity())
            .filter(|(exact, _)| *exact == actor && !is_summon)
            .map(|(_, cell)| *cell);
        if let Some(movement) = ai.native_movement_mut(actor) {
            movement.home = home;
        }
        let mut map = NativeSummonMap {
            tx,
            authority,
            room,
            objects,
            stamp,
            tiles: BTreeMap::new(),
            path: None,
        };
        if let Some(home) = home
            && (before.position().x.abs_diff(home.x) > 50
                || before.position().y.abs_diff(home.y) > 50
                || before.position().floor.abs_diff(home.floor) > 2)
        {
            let admits = map.read_tile(runtime, home).await;
            // The actual spawn cell and current Item/Map read stay bound through the sole
            // Movement relocation. A refused home idles in place; summons never enter here.
            if !map.current(runtime, stamp) {
                return Err(CarrierError::WrongScope);
            }
            if admits == Some(true) {
                let _ = crate::movement::relocate_creature(
                    runtime,
                    stamp,
                    actor,
                    before,
                    home,
                    |runtime, expected, next| map.current_step_admits(runtime, expected, next),
                );
            }
            ai.idle_native_movement(actor, true);
            ai.forget_dead(runtime);
            continue;
        }
        let Some(state) = ai.state(actor) else {
            continue;
        };
        let anchor = home.unwrap_or_else(|| state.movement_anchor());
        let target_distance = state.movement_target_distance(now_us);
        let movement = ai.native_movement(actor);
        let previous_goal = movement.map_or(G::Stand, |movement| movement.goal);
        let previous_sequence = movement.map_or(0, |movement| movement.goal_sequence);
        let retained = movement.and_then(|movement| movement.path.clone());
        let can_move = movement.is_some_and(|movement| {
            crate::movement::speed::runtime_creature_speed(
                runtime,
                actor,
                &movement.source,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now_us),
            )
            .is_some()
        });
        let perceived = candidates.iter().any(|candidate| {
            crate::ai_think::targeting::sees(candidate.position, before.position())
        });
        let facts = crate::ai_think::CreatureThinkFacts {
            position: before.position(),
            health,
            has_active_condition: runtime
                .actor_conditions(actor, None)
                .is_ok_and(|store| !store.instances().is_empty()),
            players: candidates,
            now_us,
            root,
            revisions,
        };
        let mut prepared = None;
        let mut reselect_target = false;
        let report = {
            let mut context = (
                &*runtime,
                &mut map,
                &mut *budget,
                content,
                revisions,
                retained,
                &mut prepared,
                &mut reselect_target,
            );
            ai.think_with_current_goal_facts(
                actor,
                &facts,
                &mut context,
                move |context, input, report| {
                    Box::pin(async move {
                        let (
                            runtime,
                            map,
                            budget,
                            content,
                            revisions,
                            retained,
                            prepared,
                            reselect_target,
                        ) = context;
                        let runtime = *runtime;
                        let map = &mut **map;
                        let budget = &mut **budget;
                        let content = *content;
                        let revisions = *revisions;
                        let retained = retained.take();
                        let prepared = &mut **prepared;
                        let reselect_target = &mut **reselect_target;
                        let goal = if !can_move || !input.behavior.movement.can_walk {
                            G::Stand
                        } else if let Some(target) = report.target {
                            if report.fleeing {
                                G::Flee { target }
                            } else {
                                G::Chase {
                                    target,
                                    distance: target_distance,
                                }
                            }
                        } else if perceived {
                            match input.behavior.movement.wander {
                                Some(wander) => G::Wander {
                                    anchor,
                                    interval_ms: wander.interval_ms,
                                    radius_tiles: wander.radius_tiles,
                                },
                                None => G::Stand,
                            }
                        } else if let Some(home) = home.filter(|home| *home != before.position()) {
                            G::WalkBack { home }
                        } else {
                            G::Stand
                        };
                        let goal_sequence = if goal == previous_goal {
                            Some(previous_sequence)
                        } else {
                            previous_sequence.checked_add(1)
                        };
                        let mut movement = match goal_sequence {
                            Some(goal_sequence) => {
                                prepare_native_movement_path(
                                    map,
                                    runtime,
                                    actor,
                                    goal,
                                    goal_sequence,
                                    input.occurrence.sequence,
                                    retained,
                                    revisions,
                                    budget,
                                    now_us,
                                    player_waiting,
                                )
                                .await
                            }
                            None => PreparedNativeMovement {
                                goal: G::Stand,
                                goal_sequence: previous_sequence,
                                request_sequence: input.occurrence.sequence,
                                path: None,
                                search_pending: false,
                                searched: false,
                            },
                        };
                        if matches!(goal, G::WalkBack { .. }) {
                            report.idle = movement.goal == G::Stand;
                        }
                        *reselect_target = movement.searched
                            && movement.goal == G::Stand
                            && goal.target().is_some();
                        // Intrinsic children have no wild home. Their native role and removal-aware
                        // Movement commit remain authoritative; this schedules no second summon pass.
                        let Some(record) = current_native_creature_record(runtime, content, actor)
                        else {
                            *prepared = Some(movement);
                            return Ok(None);
                        };
                        let Some(catalog) = content
                            .summon_catalog(&record.profile.target)
                            .filter(|_| input.behavior.summons.is_some())
                        else {
                            *prepared = Some(movement);
                            return Ok(None);
                        };
                        let mut current = catalog.facts(runtime, input.occurrence, None);
                        if !current.is_summon
                            && let Some(target) = input.target
                        {
                            let reachable = if map.path.is_some() {
                                map.current_target_reachable(runtime, actor, target.actor, stamp)
                            } else if movement.searched
                                || movement.search_pending
                                || movement.path.is_some()
                            {
                                // The one Movement request already ran (or is pending). Unknown cannot
                                // start a second search in this occurrence just for summon admission.
                                None
                            } else {
                                movement.searched = true;
                                map.prepare_target(
                                    runtime,
                                    actor,
                                    target.actor,
                                    budget,
                                    now_us,
                                    player_waiting,
                                )
                                .await
                            };
                            current.target_with_path = match reachable {
                                Some(true) => Some(target.actor),
                                Some(false) => None,
                                None => {
                                    *prepared = Some(movement);
                                    return Err(ScheduleError::InvalidSummonFacts);
                                }
                            };
                            if current.target_with_path.is_some() {
                                map.read_placement_cells(runtime, before.position()).await;
                            }
                        }
                        *prepared = Some(movement);
                        Ok(Some(current))
                    })
                },
            )
            .await
        };
        let Some(Ok(report)) = report else {
            continue;
        };
        // No await from the sole profile schedule completion through child policy / AI
        // registration, exact rollback and final memo. The same actual TX guard is live.
        if !map.current(runtime, stamp) {
            return Err(CarrierError::WrongScope);
        }
        if !report.summon_proposals.is_empty()
            && let Some(record) = current_native_creature_record(runtime, content, actor)
            && let Some(catalog) = content.summon_catalog(&record.profile.target)
        {
            for proposal in &report.summon_proposals {
                if !content.creature_profiles().records.iter().any(|record| {
                    record.profile.target == proposal.creature && record.behavior.is_some()
                }) {
                    continue;
                }
                let _ = summons.execute_current_owner(
                    runtime,
                    stamp,
                    catalog,
                    proposal,
                    &mut map,
                    |runtime, child| {
                        register_native_summon_child(
                            runtime,
                            ai,
                            content,
                            child,
                            &proposal.creature,
                            now_us,
                        )
                    },
                );
            }
        }
        if let Some(prepared) = prepared {
            install_native_movement_goal(runtime, ai, stamp, actor, prepared, now_us);
            if reselect_target {
                ai.forget_unreachable_target(actor);
            }
        }
        reports.push((actor, report));
    }
    Ok((reports, ai.earliest_due()))
}

#[cfg(test)]
/// Target selection, schedule and movement run once. The returned proposals are then admitted
/// and registered synchronously under the same Channel lock and its actual owner stamp.
/// The existing map owner must supply current Item/field facts as well as terrain; this
/// consumer performs no path search and cannot substitute an empty map for an absent read.
#[allow(clippy::too_many_arguments)]
fn run_native_monster_thinks(
    runtime: &mut ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    summons: &mut NativeSummonOwner,
    content: &crate::content::native_gameplay::NativeGameplayState,
    now_us: u64,
    root: &GameplayDecisionRoot,
    revisions: &crate::ability::RevisionSet,
    candidates: &[TargetCandidate],
    walkable: impl Fn(MovementLocalPosition) -> bool,
    mut map: Option<&mut dyn SummonLocationPolicy>,
) -> Result<NativeThinkPass, CarrierError> {
    let stamp = runtime.issue_owner_work()?;
    let (reports, mut next) = ai.run_due_thinks_with_profile_facts_until_next(
        runtime,
        now_us,
        root,
        revisions,
        candidates,
        walkable,
        |runtime, input| {
            use crate::ai_think::profile_schedule::ScheduleError;
            if input.behavior.summons.is_none() {
                return Ok(None);
            }
            let Some(record) =
                current_native_creature_record(runtime, content, input.occurrence.actor)
            else {
                return Ok(None);
            };
            let Some(catalog) = content.summon_catalog(&record.profile.target) else {
                return Ok(None);
            };
            let mut facts = catalog.facts(runtime, input.occurrence, None);
            if !facts.is_summon
                && let Some(target) = input.target
            {
                let map = map
                    .as_deref_mut()
                    .ok_or(ScheduleError::InvalidSummonFacts)?;
                facts.target_with_path = match map.current_target_reachable(
                    runtime,
                    input.occurrence.actor,
                    target.actor,
                    stamp,
                ) {
                    Some(true) => Some(target.actor),
                    Some(false) => None,
                    None => return Err(ScheduleError::InvalidSummonFacts),
                };
            }
            Ok(Some(facts))
        },
    );
    for (parent, report) in &reports {
        if report.summon_proposals.is_empty() {
            continue;
        }
        let Some(record) = current_native_creature_record(runtime, content, *parent) else {
            continue;
        };
        let Some(catalog) = content.summon_catalog(&record.profile.target) else {
            continue;
        };
        for proposal in &report.summon_proposals {
            let Some(map) = map.as_deref_mut() else {
                continue;
            };
            // The immutable catalog establishes identity/health; registration additionally
            // needs this child's actual behavior and installed native policy.
            if !content.creature_profiles().records.iter().any(|record| {
                record.profile.target == proposal.creature && record.behavior.is_some()
            }) {
                continue;
            }
            let result = summons.execute_current_owner(
                runtime,
                stamp,
                catalog,
                proposal,
                map,
                |runtime, child| {
                    register_native_summon_child(
                        runtime,
                        ai,
                        content,
                        child,
                        &proposal.creature,
                        now_us,
                    )
                },
            );
            if result.is_ok_and(|receipt| receipt.newly_created) {
                // Newly registered children are due now, after the pass computed its next due.
                // Leave them to the existing bounded next owner window, not a second think.
                next = Some(next.map_or(now_us, |due| due.min(now_us)));
            }
        }
    }
    Ok((reports, next))
}

fn register_native_summon_child(
    runtime: &mut ChannelRuntimeV1,
    ai: &mut CreatureAiTable,
    content: &crate::content::native_gameplay::NativeGameplayState,
    child: ExactActorRef,
    creature: &ProjectV2DefinitionRef,
    now_us: u64,
) -> Result<(), CarrierError> {
    runtime.install_creature_policy(child, &creature.key)?;
    let record = current_native_creature_record(runtime, content, child)
        .ok_or(CarrierError::ProfileMissing)?;
    if record.profile.target != *creature {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    // Atomic AI admission is the last fallible step. A refusal has no AI entry; the summon
    // owner retires the physical child, movement position and installed policy together.
    admit_native_creature_ai(runtime, ai, content, child, now_us)
}

/// CREATURE-MOVE-1 (CREATURE-AI-0 §5.3, R2): whether a creature may step onto `cell`. It is a
/// Walkable cell of the qualified room, no door cell (open or closed) covers it, it is neither a
/// protection-zone nor a floor-change tile, and it is no bundle teleport (WORLD-INTERACTION-0
/// §6.5: monsters never enter one). A creature never opens a door and never changes floors by a
/// step. The check reads only; the step is committed by the Channel owner.
///
/// Retained only for the static-cell regression tests. This helper does not read durable Items;
/// current production movement uses NativeSummonMap under the actual Item-owner transaction.
#[cfg(test)]
fn creature_cell_admissible(
    cells: &crate::content::NativeEntryMovementCells,
    doors: &std::collections::BTreeSet<LogicalCell>,
    cell: MovementLocalPosition,
) -> bool {
    let target = LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    let tile = match cells.spell_tiles().lookup(cells.scope(), target) {
        Ok(tile) => Some((tile.flags().protection_zone, tile.flags().floor_change)),
        // An accepted entry room without a tile document (ENTRY-SPELL-TILES-1 gave
        // `native_entry_room.json` one) answers every lookup `Unknown`. Its Movement owner admits
        // a player step on terrain and doors alone ("accepted entry movement",
        // `field_step_ingress`), so a creature step follows the same rule.
        Err(crate::content::SpellTileLookupError::Unknown) if accepted_entry_movement(cells) => {
            Some((false, false))
        }
        // Unknown cells of a tile-qualified map, an absent tile or another scope refuse.
        Err(_) => None,
    };
    cell_open(
        cells.index().lookup(cells.scope(), target) == Ok(crate::content::CollisionClass::Walkable),
        doors.contains(&target),
        crate::map::floor::is_teleport(cells, target),
        tile,
    )
}

/// Whether `cells` move by the accepted entry movement: the entry-room collision index (not a
/// source world, not a bundle) outside the source profile. Its tiles are qualified from the room's
/// own optional tile document, so `Unknown` there means only "no document"; a source-qualified
/// map's `Unknown` names a genuinely unknown cell and a bundle's lookup is another scope.
#[cfg(test)]
fn accepted_entry_movement(cells: &crate::content::NativeEntryMovementCells) -> bool {
    matches!(
        cells.index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Entry(_)
    ) && !crate::movement::source_floor_change::is_source_profile(cells)
}

/// BUNDLE-MONSTER-AI-1: whether `cells` are a World Bundle's.
fn bundle_cells(cells: &crate::content::NativeEntryMovementCells) -> bool {
    matches!(
        cells.index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
    )
}

/// The step rule over the facts of one cell. `tile` is `(protection_zone, floor_change)`, or
/// `None` when the tile facts are unknown, which refuses (fail closed, like the think pass).
#[cfg(test)]
const fn cell_open(walkable: bool, door: bool, teleport: bool, tile: Option<(bool, bool)>) -> bool {
    match tile {
        Some((protection_zone, floor_change)) => {
            walkable && !door && !teleport && !protection_zone && !floor_change
        }
        None => false,
    }
}

/// A present dead actor still has an observable vitals state until respawn. It must
/// not occupy the nearest-candidate slot and prevent attacks on a living player.
fn current_player_target<'a>(
    runtime: &crate::foundation::ChannelRuntimeV1,
    states: &'a super::actor_spell::ChannelSpellStates,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Option<&'a crate::spell::cast::PlayerSpellState> {
    if states.is_dead(actor)
        || states.has_pending_spell_commit(actor, session)
        || runtime.assert_actor_spell_unreserved(actor).is_err()
    {
        return None;
    }
    let state = states.get(runtime, actor, session)?;
    (state.vitals().health > 0).then_some(state)
}

/// SPAWN-1a (CREATURE-AI-0 §6.3): runs the respawn occurrences due at `now` with the MOVE-RL-11
/// reference view. It needs no melee content. A refusal leaves the occurrences pending.
fn drive_owner_spawns(runtime: &mut ChannelRuntimeV1, now: u64) {
    let _ = runtime.drive_spawn_respawns(now, |observer, cell| {
        use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
        match (
            VisibilityPosition::new(observer.x, observer.y, observer.floor),
            VisibilityPosition::new(cell.x, cell.y, cell.floor),
        ) {
            (Ok(observer), Ok(cell)) => VisibilitySettings::REFERENCE.can_see(observer, cell),
            _ => false,
        }
    });
}

/// One monster the first playable melee dispatch serves: a wild hostile Creature whose bound
/// profile has a native melee schedule.
struct QualifiedMelee<'a> {
    behavior: &'a ProjectV2BehaviorAuthoring,
    definition: MeleeDefinition,
    race_key: String,
}

fn qualified_melee<'a>(
    runtime: &ChannelRuntimeV1,
    content: &'a crate::content::native_gameplay::NativeGameplayState,
    actor: ExactActorRef,
) -> Option<QualifiedMelee<'a>> {
    let snapshot = runtime.companion_snapshot(actor).ok()?;
    if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
        return None;
    }
    let record = content.creature_profiles().records.iter().find(|record| {
        record.profile.target.key == snapshot.state.policy.definition_key
            && record.profile.target.revision == snapshot.state.policy.definition_revision
    })?;
    let melee = record.monster_melee.as_ref()?;
    let ProjectV2AuthoringProfileData::Behavior(behavior) = &record.behavior.as_ref()?.data else {
        return None;
    };
    if !behavior.targeting.hostile || !behavior.targeting.can_target {
        return None;
    }
    // CREATURE-AI-1 §1.5: the swing's `AI_ATTACK` draw index is the melee entry's index in the
    // profile's `attacks[]`.
    let entry_index = behavior
        .attacks
        .iter()
        .position(|a| {
            a.ability == melee.ability
                && a.interval_ms == melee.interval_ms
                && a.chance_ppm == melee.chance_ppm
        })
        .and_then(|index| u16::try_from(index).ok())?;
    let definition = MeleeDefinition::new(
        content.source_digest(),
        melee.interval_ms,
        melee.chance_ppm,
        melee.minimum,
        melee.maximum,
    )?
    .with_entry_index(entry_index);
    Some(QualifiedMelee {
        behavior,
        definition,
        race_key: snapshot.state.policy.definition_key.clone(),
    })
}

/// One melee pass's player candidates from the melee census, sorted by placement: each with its
/// position, session, invisibility and protection. A player whose control was lost is a
/// candidate only while its in-fight deadline holds it in the world (ATTACK0-RL-03).
pub(in crate::gameplay_transport) fn melee_targets(
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &ChannelAttackStates,
    census: &[(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<GameSessionId>,
    )],
    now: u64,
) -> Vec<(
    ExactActorRef,
    MovementLocalPosition,
    GameSessionId,
    bool,
    bool,
)> {
    let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    let mut targets = census
        .iter()
        .filter_map(|(actor, position, session)| {
            let session = (*session)?;
            let state = current_player_target(runtime, states, *actor, session)?;
            let invisible = state.owned_invisible_at(now).ok()?;
            let protected =
                attack.creature_target_protection(runtime, *actor, session, semantic_now)?;
            let p = position.position();
            let tile = room
                .movement_cells()
                .spell_tiles()
                .lookup(
                    room.movement_cells().scope(),
                    LogicalCell {
                        x: p.x,
                        y: p.y,
                        z: i32::from(p.floor),
                    },
                )
                .ok()?;
            Some((
                *actor,
                p,
                session,
                invisible,
                protected || tile.flags().protection_zone,
            ))
        })
        .collect::<Vec<_>>();
    targets.sort_by_key(|(actor, ..)| actor.placement_identity());
    targets
}

/// Preserve census identity/order while bounding only eligible local perception.
fn local_target_indices(
    origin: MovementLocalPosition,
    sense_invisible: bool,
    targets: impl IntoIterator<Item = (MovementLocalPosition, bool, bool)>,
) -> Option<Vec<usize>> {
    let mut indices = Vec::new();
    for (index, (position, invisible, protected)) in targets.into_iter().enumerate() {
        // CREATURE-AI-1 §2.1 item 2: perception is VIS-1's `can_see` from the player.
        if protected
            || (invisible && !sense_invisible)
            || position.floor != origin.floor
            || !crate::ai_think::targeting::sees(position, origin)
        {
            continue;
        }
        if indices.len() == ResourceLimit::PerceptionCandidates.maximum() {
            return None;
        }
        indices.push(index);
    }
    Some(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::RuntimeWorkStamp;
    fn position(x: i32, floor: i16) -> MovementLocalPosition {
        MovementLocalPosition { x, y: 0, floor }
    }

    struct CurrentOwnerSummonFixture {
        runtime: ChannelRuntimeV1,
        content: crate::content::native_gameplay::NativeGameplayState,
        parent: ProjectV2DefinitionRef,
        proposal: crate::ai_think::profile_schedule::ProfileSummonProposal,
    }

    #[allow(clippy::expect_used)]
    fn current_owner_summon_fixture(
        capacity: usize,
        install_policies: bool,
    ) -> CurrentOwnerSummonFixture {
        use crate::foundation::{ChannelContentPin, WorldId};
        let (content, parent) = crate::content::native_gameplay::tests::decoded_summon_fixture();
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            content.source_digest(),
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
        current_owner_summon_fixture_with_content(capacity, install_policies, content, parent, pin)
    }

    #[allow(clippy::expect_used)]
    fn current_owner_summon_fixture_with_content(
        capacity: usize,
        install_policies: bool,
        content: crate::content::native_gameplay::NativeGameplayState,
        parent: ProjectV2DefinitionRef,
        pin: crate::foundation::ChannelContentPin,
    ) -> CurrentOwnerSummonFixture {
        use crate::foundation::{ChannelId, NodeId};
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            pin.world_id(),
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
            pin,
        )
        .expect("current Channel owner");
        if install_policies {
            content
                .install_companion_policies(&mut runtime)
                .expect("decoded policies");
        }
        let health = content
            .creature_profiles()
            .records
            .iter()
            .find_map(|record| match &record.profile.data {
                ProjectV2AuthoringProfileData::Creature(profile)
                    if record.profile.target == parent =>
                {
                    profile.health.and_then(|health| i64::try_from(health).ok())
                }
                _ => None,
            })
            .expect("qualified parent health");
        let actor = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &parent.key,
                health,
            )
            .expect("source parent");
        let target = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 105,
                    y: 100,
                    floor: 7,
                },
                &parent.key,
                health,
            )
            .expect("live target");
        let proposal = crate::ai_think::profile_schedule::ProfileSummonProposal {
            occurrence: crate::ai_think::ThinkOccurrence { actor, sequence: 0 },
            entry_index: 0,
            revisions: crate::ability::RevisionSet::new(
                "rules-r1",
                "content-r1",
                "policy-r1",
                "definition-r1",
                "sim-r1",
            )
            .expect("revisions"),
            creature: parent.clone(),
            target_with_path: target,
            maximum_total: 2,
            maximum_of_creature: 2,
        };
        CurrentOwnerSummonFixture {
            runtime,
            content,
            parent,
            proposal,
        }
    }

    /// Explicit physical fixture: known cells in one small square, plus the actual carrier's
    /// current actor positions and owner stamp. This does not qualify production map/Item reads.
    struct CurrentOwnerSummonMap;

    impl SummonLocationPolicy for CurrentOwnerSummonMap {
        fn current_target_reachable(
            &mut self,
            runtime: &ChannelRuntimeV1,
            parent: ExactActorRef,
            target: ExactActorRef,
            stamp: RuntimeWorkStamp,
        ) -> Option<bool> {
            runtime.read_actor_position(parent).ok()?;
            runtime.read_actor_position(target).ok()?;
            Some(runtime.owner_fence().ok()?.accepts_stamp(stamp))
        }

        fn current_cell_admits(
            &mut self,
            runtime: &ChannelRuntimeV1,
            _parent: ExactActorRef,
            _child: &ProjectV2DefinitionRef,
            position: MovementLocalPosition,
            stamp: RuntimeWorkStamp,
        ) -> Option<bool> {
            if !runtime.owner_fence().ok()?.accepts_stamp(stamp)
                || position.floor != 7
                || !(90..=110).contains(&position.x)
                || !(90..=110).contains(&position.y)
            {
                return None;
            }
            Some(true)
        }
    }

    fn execute_current_owner_summon_fixture(
        fixture: &mut CurrentOwnerSummonFixture,
        states: &mut ChannelSpellStates,
        now_us: u64,
    ) -> Result<crate::monster_summon::SummonReceipt, crate::monster_summon::SummonError> {
        use crate::monster_summon::SummonError;
        let stamp = fixture
            .runtime
            .issue_owner_work()
            .map_err(SummonError::Carrier)?;
        let catalog = fixture
            .content
            .summon_catalog(&fixture.parent)
            .ok_or(SummonError::InvalidSource)?;
        let ai = &mut states.monster_melee.creature_ai;
        states.native_summons.execute_current_owner(
            &mut fixture.runtime,
            stamp,
            catalog,
            &fixture.proposal,
            &mut CurrentOwnerSummonMap,
            |runtime, child| {
                register_native_summon_child(
                    runtime,
                    ai,
                    &fixture.content,
                    child,
                    &fixture.proposal.creature,
                    now_us,
                )
            },
        )
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn current_owner_summon_registers_policy_and_one_ai_entry_without_replaying_admission() {
        let mut fixture = current_owner_summon_fixture(8, true);
        let mut states = ChannelSpellStates::default();
        let receipt = execute_current_owner_summon_fixture(&mut fixture, &mut states, 17)
            .expect("admit and register");
        assert!(receipt.newly_created);
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            1
        );
        let snapshot = fixture
            .runtime
            .companion_snapshot(receipt.child)
            .expect("native combat policy");
        assert_eq!(snapshot.state.policy.definition_key, fixture.parent.key);
        assert_eq!(
            snapshot.state.policy.definition_revision,
            fixture.parent.revision
        );
        assert_eq!(states.monster_melee.creature_ai.len(), 1);
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );
        let replay = execute_current_owner_summon_fixture(&mut fixture, &mut states, 99)
            .expect("historical receipt");
        assert!(!replay.newly_created);
        assert_eq!(replay.child, receipt.child);
        assert_eq!(states.monster_melee.creature_ai.len(), 1);
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            1
        );
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn activated_outer_source_summon_rejects_inner_catalog_and_registers_once() {
        use crate::foundation::{ChannelContentPin, WorldId};
        use crate::monster_summon::SummonError;
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let (controller, pin, parent) =
            crate::content::native_gameplay::tests::activated_summon_fixture(
                world,
                include_bytes!(
                    "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                ),
            );
        let content = controller
            .active()
            .expect("actual outer activation")
            .native_gameplay()
            .expect("activated native content")
            .clone();
        let room = pin.qualified_room();
        let compiled = room.compiled();
        let outer = crate::content::native_source_world_carrier::decode(
            &compiled.server_artifact,
            &compiled.client_artifact,
        )
        .expect("same qualified outer artifact pair");
        let inner = crate::content::native_gameplay::decode(outer.baseline_server)
            .expect("actual retained inner artifact")
            .state;
        assert_ne!(inner.source_digest(), content.source_digest());
        assert_eq!(inner.creature_profiles(), content.creature_profiles());
        assert_eq!(
            content.source_digest(),
            pin.identity().server_artifact_digest()
        );
        let old_catalog = inner.summon_catalog(&parent).expect("old inner catalog");
        let start = room.entry_start();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        // The existing square around (100,100,7) and CurrentOwnerSummonMap are explicit
        // physical component-test inputs. Real Thalom geometry/Map/SQL is covered by the
        // separate PostgreSQL cases; this case qualifies outer content and owner registration.
        let mut fixture =
            current_owner_summon_fixture_with_content(8, true, content, parent, channel_pin);
        let parent = fixture.proposal.occurrence.actor;
        let mut states = ChannelSpellStates::default();
        let stamp = fixture.runtime.issue_owner_work().expect("current owner");
        let mut registrations = 0;
        let old = states.native_summons.execute_current_owner(
            &mut fixture.runtime,
            stamp,
            old_catalog,
            &fixture.proposal,
            &mut CurrentOwnerSummonMap,
            |_, _| {
                registrations += 1;
                Ok(())
            },
        );
        assert_eq!(old, Err(SummonError::InvalidSource));
        assert_eq!(registrations, 0);
        assert_eq!(fixture.runtime.native_summon_count(parent, None), 0);
        assert!(states.monster_melee.creature_ai.is_empty());

        let receipt = execute_current_owner_summon_fixture(&mut fixture, &mut states, 17)
            .expect("outer catalog and nested physical spec admit and register");
        assert!(receipt.newly_created);
        assert_eq!(
            fixture.runtime.native_summon_role(receipt.child),
            Ok(Some(parent))
        );
        let child = fixture
            .runtime
            .companion_snapshot(receipt.child)
            .expect("child policy");
        assert_eq!(child.state.policy.definition_key, fixture.parent.key);
        assert_eq!(
            child.state.policy.definition_revision,
            fixture.parent.revision
        );
        assert_eq!(fixture.runtime.native_summon_count(parent, None), 1);
        assert_eq!(states.monster_melee.creature_ai.len(), 1);
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );

        let replay = execute_current_owner_summon_fixture(&mut fixture, &mut states, 99)
            .expect("final outer receipt replays");
        assert!(!replay.newly_created);
        assert_eq!(replay.child, receipt.child);
        assert_eq!(fixture.runtime.native_summon_count(parent, None), 1);
        assert_eq!(states.monster_melee.creature_ai.len(), 1);
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn current_owner_summon_movement_refuses_foreign_stamp_and_retains_ai_due() {
        let mut fixture = current_owner_summon_fixture(8, true);
        let mut states = ChannelSpellStates::default();
        let receipt = execute_current_owner_summon_fixture(&mut fixture, &mut states, 17)
            .expect("registered child");
        let before = fixture
            .runtime
            .read_actor_position(receipt.child)
            .expect("live child position");
        let mut foreign_id = [0; 16];
        foreign_id[6] = 0x70;
        foreign_id[8] = 0x80;
        foreign_id[15] = 42;
        let (mut foreign, ..) = crate::monster_summon::summon_execution_fixture_for_world(
            crate::foundation::WorldId::decode(&foreign_id).expect("independent foreign World"),
        );
        let stamp = foreign
            .issue_owner_work()
            .expect("independent actual owner stamp");
        let destination = [
            crate::movement::CardinalStep::North,
            crate::movement::CardinalStep::East,
            crate::movement::CardinalStep::South,
            crate::movement::CardinalStep::West,
        ]
        .into_iter()
        .filter_map(|step| path_destination(before.position(), step))
        .find(|position| fixture.runtime.native_summon_cell_free(*position))
        .expect("a current free cardinal neighbour");
        assert_eq!(
            fixture
                .runtime
                .commit_native_summon_cardinal_from_current_owner(
                    stamp,
                    receipt.child,
                    before,
                    destination,
                ),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            fixture.runtime.read_actor_position(receipt.child),
            Ok(before)
        );
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            1
        );
        let current = fixture
            .runtime
            .issue_owner_work()
            .expect("same runtime's current stamp");
        let moved = fixture
            .runtime
            .commit_native_summon_cardinal_from_current_owner(
                current,
                receipt.child,
                before,
                destination,
            )
            .expect("all non-stamp facts permit the same move");
        assert!(matches!(
            moved,
            crate::foundation::NativeSummonMovementOutcome::Moved(_)
        ));
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(17)
        );
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn current_owner_summon_registration_failure_retires_child_and_replay_cannot_resurrect_it() {
        use crate::monster_summon::SummonError;
        let mut fixture = current_owner_summon_fixture(8, false);
        let mut states = ChannelSpellStates::default();
        // Physical source admission remains valid. Its actual native registration lacks the
        // installed policy table, so the child and link must be retired in this same call.
        let refused = execute_current_owner_summon_fixture(&mut fixture, &mut states, 17);
        assert_eq!(
            refused,
            Err(SummonError::Carrier(CarrierError::InvalidCreatureTarget))
        );
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            0
        );
        assert!(states.monster_melee.creature_ai.is_empty());
        fixture
            .content
            .install_companion_policies(&mut fixture.runtime)
            .expect("policy now available");
        assert_eq!(
            execute_current_owner_summon_fixture(&mut fixture, &mut states, 99),
            refused
        );
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            0
        );
        assert!(states.monster_melee.creature_ai.is_empty());
        fixture.proposal.occurrence.sequence += 1;
        let child = execute_current_owner_summon_fixture(&mut fixture, &mut states, 100)
            .expect("a new occurrence may retry")
            .child;
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            1
        );
        assert_eq!(states.monster_melee.creature_ai.due(child), Some(100));
    }

    #[test]
    fn current_owner_summon_capacity_refusal_has_no_child_or_ai_registration() {
        use crate::monster_summon::SummonError;
        let mut fixture = current_owner_summon_fixture(2, true);
        let mut states = ChannelSpellStates::default();
        assert_eq!(
            execute_current_owner_summon_fixture(&mut fixture, &mut states, 17),
            Err(SummonError::Carrier(CarrierError::CapacityExceeded)),
        );
        assert_eq!(
            fixture
                .runtime
                .native_summon_count(fixture.proposal.occurrence.actor, None),
            0
        );
        assert!(states.monster_melee.creature_ai.is_empty());
        assert_eq!(states.monster_melee.creature_ai.earliest_due(), None);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn current_owner_summon_one_pass_refuses_missing_map_then_registers_once_with_current_facts() {
        use crate::ai_think::profile_schedule::ScheduleError;
        let mut fixture = current_owner_summon_fixture(8, true);
        let mut states = ChannelSpellStates::default();
        let parent = fixture.proposal.occurrence.actor;
        fixture
            .runtime
            .install_creature_policy(parent, &fixture.parent.key)
            .expect("source parent policy");
        admit_native_creature_ai(
            &fixture.runtime,
            &mut states.monster_melee.creature_ai,
            &fixture.content,
            parent,
            17,
        )
        .expect("one source-bound native AI table");
        assert!(
            states
                .monster_melee
                .creature_ai
                .state(parent)
                .expect("parent AI")
                .profile()
                .behavior()
                .summons
                .is_some()
        );
        let candidates = [TargetCandidate {
            actor: fixture.proposal.target_with_path,
            position: fixture
                .runtime
                .read_actor_position(fixture.proposal.target_with_path)
                .expect("actual target position")
                .position(),
            eligible: true,
            invisible: false,
            health: 20,
            damage: 0,
        }];
        let root = GameplayDecisionRoot::from_bytes(fixture.content.source_digest());
        let (unknown, _) = run_native_monster_thinks(
            &mut fixture.runtime,
            &mut states.monster_melee.creature_ai,
            &mut states.native_summons,
            &fixture.content,
            17,
            &root,
            &fixture.proposal.revisions,
            &candidates,
            |_| false,
            None,
        )
        .expect("ordinary think with missing current map");
        assert_eq!(unknown.len(), 1);
        assert_eq!(
            unknown[0].1.refusal,
            Some(ScheduleError::InvalidSummonFacts)
        );
        assert!(unknown[0].1.summon_proposals.is_empty());
        assert_eq!(fixture.runtime.native_summon_count(parent, None), 0);
        assert_eq!(states.monster_melee.creature_ai.len(), 1);

        let now_us = 1_000_017;
        let (reports, next) = run_native_monster_thinks(
            &mut fixture.runtime,
            &mut states.monster_melee.creature_ai,
            &mut states.native_summons,
            &fixture.content,
            now_us,
            &root,
            &fixture.proposal.revisions,
            &candidates,
            |_| false,
            Some(&mut CurrentOwnerSummonMap),
        )
        .expect("single native think with explicit physical fixture facts");
        assert_eq!(
            reports.len(),
            1,
            "the new child is not thought in a second pass"
        );
        assert_eq!(reports[0].1.sequence, 2);
        assert_eq!(reports[0].1.refusal, None);
        assert_eq!(reports[0].1.summon_proposals.len(), 1);
        assert_eq!(fixture.runtime.native_summon_count(parent, None), 1);
        assert_eq!(states.monster_melee.creature_ai.len(), 2);
        assert_eq!(
            next,
            Some(now_us),
            "the same owner will service the new child's due"
        );
        fixture.proposal = reports[0].1.summon_proposals[0].clone();
        let receipt = execute_current_owner_summon_fixture(&mut fixture, &mut states, now_us + 10)
            .expect("one-pass result is already memoized");
        assert!(!receipt.newly_created);
        assert_eq!(
            states.monster_melee.creature_ai.due(receipt.child),
            Some(now_us)
        );
        assert_eq!(states.monster_melee.creature_ai.len(), 2);
    }

    /// Real PostgreSQL scope/Item reads remain held through the one native caller. This
    /// reuses the registered actual-owner harness and an explicitly admitted test source;
    /// it does not reinterpret that fixture source as production summon content.
    #[test]
    fn current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope()
    -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, configured_admin, debug, id};
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        let executor = crate::bestiary_postgres_harness::runtime()?;
        let harness = executor.block_on(Harness::create(admin, "summon_current_map", false))?;
        // The body future and its transactions unwind before the retained harness is cleaned.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            executor.block_on(async {
                let world = WorldId::decode(&id(crate::bestiary_postgres_harness::WORLD)).map_err(debug)?;
                let channel = ChannelId::decode(&id(crate::bestiary_postgres_harness::CHANNEL)).map_err(debug)?;
                let (controller, pin, definition) = crate::content::native_gameplay::tests::activated_summon_fixture(
                    world,
                    include_bytes!("../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"),
                );
                let content = controller.active().ok_or("active fixture generation")?
                    .native_gameplay().ok_or("decoded fixture content")?;
                let room = pin.qualified_room();
                assert!(room.source_world().is_some());
                let cell = |x, y| MovementLocalPosition { x, y, floor: -7 };
                // These actual Thalom cells are outside protection zones and carry qualified
                // source ground. The room's original entry is a different, protected cell.
                let start = cell(2, 12);
                let channel_pin = ChannelContentPin::from_activation(
                    world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(), pin.frame_binding().digest(),
                    room.map_revision_digest(), (start.x, start.y, start.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    world, channel, NodeId::decode(harness.node.fact().node_id().as_bytes()).map_err(debug)?,
                    1, 1, 1, "runtime-scope-assignment:1", 8, channel_pin,
                ).map_err(debug)?;
                content.install_companion_policies(&mut runtime).map_err(debug)?;
                let health = content.creature_profiles().records.iter().find_map(|record| {
                    match &record.profile.data {
                        ProjectV2AuthoringProfileData::Creature(profile) if record.profile.target == definition =>
                            profile.health.and_then(|health| i64::try_from(health).ok()),
                        _ => None,
                    }
                }).ok_or("qualified creature health")?;
                let parent = runtime.admit_source_pinned_lab_creature(cell(2, 12), &definition.key, health).map_err(debug)?;
                let target = runtime.admit_source_pinned_lab_creature(cell(4, 12), &definition.key, health).map_err(debug)?;
                runtime.install_creature_policy(parent, &definition.key).map_err(debug)?;
                runtime.install_creature_policy(target, &definition.key).map_err(debug)?;
                let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
                let generation = runtime.binding().scope_generation();
                let object_fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
                    scope, generation,
                    crate::world_runtime::ReferenceContentGeneration::from_content(room.door()).map_err(debug)?,
                );
                let objects = crate::world_runtime::bind_native_entry_door(room.door(), &object_fence, scope, generation)
                    .map_err(debug)?;
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let recovery = harness.root.open_character_authority(&seal).await.map_err(debug)?;
                let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(world, channel).await;
                let mut tx = harness.pool.begin().await?;
                let authority = item_tx::assert_spell_item_scope_in_transaction(
                    &mut tx, &permit, &harness.root, &recovery, &harness.node, scope, generation.get(),
                ).await.map_err(debug)?;
                let stamp = runtime.issue_owner_work().map_err(debug)?;
                let mut budget = crate::combat_attack::ChaseSearchBudget::default();
                let full_budget = budget.remaining(0);
                {
                    let mut map = NativeSummonMap {
                        tx: &mut tx, authority: &authority, room, objects: &objects, stamp,
                        tiles: BTreeMap::new(), path: None,
                    };
                    assert_eq!(map.read_tile(&runtime, cell(3, 12)).await, Some(true), "actual empty Item tile");
                    for (position, speed) in [(start, 150), (cell(3, 12), 150), (cell(4, 12), 110)] {
                        let tile = map.read_tile_facts(&runtime, position).await
                            .ok_or("actual source-qualified ground and current Item tile")?;
                        assert!(tile.walkable);
                        assert_eq!(tile.ground_speed, speed, "speed is actual source data, never a default");
                    }
                    assert_eq!(map.read_tile(&runtime, cell(3, 10)).await, Some(false), "actual blocked terrain");
                    assert_eq!(map.prepare_target(&runtime, parent, target, &mut budget, 0, false).await, Some(true));
                    let charged = budget.remaining(0);
                    assert_eq!(full_budget - charged, crate::combat_attack::UNITS_PER_SEARCH);
                    assert_eq!(map.prepare_target(&runtime, parent, target, &mut budget, 0, false).await, Some(true));
                    assert_eq!(budget.remaining(0), charged, "same transaction revalidates without another search");
                    let before_target = runtime.read_actor_position(target).map_err(debug)?;
                    runtime.borrow_movement_position().commit_cardinal(before_target, cell(4, 11)).map_err(debug)?;
                    assert_eq!(map.current_target_reachable(&runtime, parent, target, stamp), None, "moved target invalidates retained route");
                    assert_eq!(budget.remaining(0), charged, "stale binding cannot trigger another search");
                }
                let target_now = runtime.read_actor_position(target).map_err(debug)?;
                runtime.borrow_movement_position().commit_cardinal(target_now, cell(4, 12)).map_err(debug)?;
                let mut states = ChannelSpellStates::default();
                admit_native_creature_ai(&runtime, &mut states.monster_melee.creature_ai, content, parent, 0).map_err(debug)?;
                let candidates = [TargetCandidate {
                    actor: target, position: cell(4, 12), eligible: true, invisible: false, health: 20, damage: 0,
                }];
                let revisions = crate::ability::RevisionSet::new("rules-r1", "content-r1", "world-r1", "formula-r1", "sim-r1")
                    .map_err(debug)?;
                let decision_root = GameplayDecisionRoot::from_bytes(content.source_digest());
                while budget.charge_creature(0, crate::combat_attack::UNITS_PER_SEARCH, false) {}
                let parent_before = runtime.read_actor_position(parent).map_err(debug)?;
                let (deferred, deferred_next) = run_current_native_monster_thinks(
                    &mut runtime, &mut states.monster_melee.creature_ai, &mut states.native_summons,
                    content, 0, &decision_root, &revisions, &candidates, room, &objects,
                    &mut tx, &authority, &mut budget, false,
                ).await.map_err(debug)?;
                assert_eq!(deferred.len(), 1);
                assert_eq!(deferred[0].1.sequence, 1);
                assert_eq!(deferred[0].1.step, None, "think selects a goal, never a greedy step");
                assert_eq!(deferred[0].1.refusal, Some(crate::ai_think::profile_schedule::ScheduleError::InvalidSummonFacts));
                assert!(deferred[0].1.summon_proposals.is_empty());
                assert_eq!(deferred_next, Some(50_000));
                assert_eq!(runtime.read_actor_position(parent).map_err(debug)?, parent_before);
                assert_eq!(runtime.native_summon_count(parent, None), 0);
                let movement = states.monster_melee.creature_ai.native_movement(parent).ok_or("bound movement")?;
                assert!(movement.search_pending, "Unknown summon facts retain the actual think's pending movement request");
                assert_eq!(movement.request_sequence, 1);
                assert!(movement.path.is_none());
                let pending = states.monster_melee.creature_ai.due_native_steps(&runtime, 50_000);
                assert_eq!(pending.len(), 1);
                let step_stamp = runtime.issue_owner_work().map_err(debug)?;
                // Poll into the actual SQL reader, then cancel. The same existing timer and
                // selected goal remain; no physical/AI/pacer mutation occurred across await.
                {
                    use std::future::Future;
                    let mut work = std::pin::pin!(run_native_movement_steps(
                        &mut runtime, &mut states.monster_melee.creature_ai, content, 50_000,
                        &decision_root, &revisions, &candidates, room, &objects, &mut tx,
                        &authority, &mut budget, false, step_stamp,
                    ));
                    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
                    assert!(work.as_mut().poll(&mut cx).is_pending(), "actual transaction read yields before native commit");
                }
                assert_eq!(runtime.read_actor_position(parent).map_err(debug)?, parent_before);
                assert_eq!(states.monster_melee.creature_ai.due_native_steps(&runtime, 50_000), pending);
                assert_eq!(states.monster_melee.creature_ai.due(parent), Some(1_000_000));
                assert!(states.monster_melee.creature_ai.native_movement(parent)
                    .ok_or("retained movement")?.cadence.next_due().is_none());
                let (step_only, _) = run_current_native_monster_thinks(
                    &mut runtime, &mut states.monster_melee.creature_ai, &mut states.native_summons,
                    content, 50_000, &decision_root, &revisions, &candidates, room, &objects,
                    &mut tx, &authority, &mut budget, false,
                ).await.map_err(debug)?;
                assert!(step_only.is_empty(), "step input does not run a second think or profile schedule");
                let moved = runtime.read_actor_position(parent).map_err(debug)?;
                assert_eq!(moved.position(), cell(3, 12));
                let movement = states.monster_melee.creature_ai.native_movement(parent).ok_or("paced movement")?;
                assert!(!movement.search_pending);
                assert!(movement.path.as_ref().is_some_and(|path| path.current(
                    &runtime, parent, movement.goal, movement.goal_sequence, &revisions,
                )), "successful first step retains its exact route binding");
                let step_due = movement.cadence.next_due().ok_or("moved-only deadline")?.get();
                let current_stamp = runtime.issue_owner_work().map_err(debug)?;
                let expected_duration = {
                    let mut actual_map = NativeSummonMap {
                        tx: &mut tx, authority: &authority, room, objects: &objects,
                        stamp: current_stamp, tiles: BTreeMap::new(), path: None,
                    };
                    assert_eq!(actual_map.read_tile(&runtime, moved.position()).await, Some(true));
                    crate::movement::speed::creature_step_duration(
                        &runtime, parent, &movement.source, &actual_map, logical_cell(moved.position()),
                        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(50_000),
                    ).ok_or("same actual source speed and destination ground")?
                };
                assert_eq!(step_due, 50_000 + u64::try_from(expected_duration.as_micros())?);
                let charged = budget.remaining(50_000);
                let repeated_stamp = runtime.issue_owner_work().map_err(debug)?;
                run_native_movement_steps(
                    &mut runtime, &mut states.monster_melee.creature_ai, content, 50_000,
                    &decision_root, &revisions, &candidates, room, &objects, &mut tx,
                    &authority, &mut budget, false, repeated_stamp,
                ).await.map_err(debug)?;
                assert_eq!(runtime.read_actor_position(parent).map_err(debug)?, moved);
                assert_eq!(budget.remaining(50_000), charged, "retained input / cadence replay does not search again");
                assert_eq!(states.monster_melee.creature_ai.earliest_native_step_due(), Some(step_due));

                let (reports, next) = run_current_native_monster_thinks(
                    &mut runtime, &mut states.monster_melee.creature_ai, &mut states.native_summons,
                    content, 1_000_000, &decision_root, &revisions, &candidates, room, &objects,
                    &mut tx, &authority, &mut budget, false,
                ).await.map_err(debug)?;
                assert_eq!(reports.len(), 1, "new child does not run a second think pass");
                assert_eq!(reports[0].1.sequence, 2);
                assert_eq!(reports[0].1.step, None);
                assert_eq!(reports[0].1.refusal, None);
                assert_eq!(reports[0].1.summon_proposals.len(), 1);
                assert_eq!(runtime.native_summon_count(parent, None), 1);
                assert_eq!(states.monster_melee.creature_ai.len(), 2);
                assert_eq!(next, Some(1_000_000));
                let proposal = &reports[0].1.summon_proposals[0];
                let current = runtime.issue_owner_work().map_err(debug)?;
                {
                    let mut map = NativeSummonMap {
                        tx: &mut tx, authority: &authority, room, objects: &objects, stamp: current,
                        tiles: BTreeMap::new(), path: None,
                    };
                    let replay = states.native_summons.execute_current_owner(
                        &mut runtime, current, content.summon_catalog(&definition).ok_or("fixture catalog")?,
                        proposal, &mut map, |_, _| Err(CarrierError::ProfileInvalid),
                    ).map_err(debug)?;
                    assert!(!replay.newly_created, "replay does not invoke registration");
                    assert_eq!(states.monster_melee.creature_ai.due(replay.child), Some(1_000_000));
                    assert!(map.tiles.is_empty(), "historical receipt consumes no new map read");
                }
                tx.rollback().await?;
                let mut other_tx = harness.pool.begin().await?;
                {
                    let mut wrong_transaction = NativeSummonMap {
                        tx: &mut other_tx, authority: &authority, room, objects: &objects, stamp: current,
                        tiles: BTreeMap::new(), path: None,
                    };
                    assert_eq!(wrong_transaction.read_tile(&runtime, cell(3, 12)).await, None,
                        "formerly valid scope read cannot authorize a different physical transaction");
                }
                other_tx.rollback().await?;
                assert_eq!(runtime.native_summon_count(parent, None), 1);
                assert_eq!(states.monster_melee.creature_ai.len(), 2);

                // A real durable Item write invalidates the formerly empty cell. The cause
                // is explicitly a test fixture; no source-map receipt or movement flags are
                // invented. Generic loot MINT has no qualified movement flags, so this
                // current read must be Unknown and refuse the next summon occurrence.
                use crate::durability::item_mint::{
                    ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
                };
                let parent_before_item = runtime.read_actor_position(parent).map_err(debug)?;
                let ground = crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
                    room, &runtime, parent_before_item.position(),
                ).map_err(debug)?;
                let mut candidate = harness.root.freeze_item_mint(&recovery, &harness.node, ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        world, channel, generation, 900, 1,
                        TypedDefinitionRef {
                            family: "LootTable".into(), production_key: "fixture:loot.current-map".into(),
                            revision_ref: "r1".into(),
                        },
                        "fixture:purpose.current-map-unknown".into(), 0,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(), production_key: "fixture:item.current-map-unknown".into(),
                        revision_ref: "r1".into(),
                    },
                    quantity: 1,
                    ground: ground.placement(id(90).to_vec()),
                    content_revision: ground.content_revision.clone(),
                    ruleset_revision: "rules-r1".into(), sim_revision: "sim-r1".into(),
                }).await.map_err(debug)?;
                let mint_permit = candidate.fresh_death_lane_permit().await;
                assert!(matches!(harness.root.commit_item_mint(
                    &mint_permit, &recovery, &harness.node, &mut candidate,
                ).await.map_err(debug)?, ItemMintOutcome::Committed(_)));
                let mut current_tx = harness.pool.begin().await?;
                let current_authority = item_tx::assert_spell_item_scope_in_transaction(
                    &mut current_tx, &permit, &harness.root, &recovery, &harness.node, scope, generation.get(),
                ).await.map_err(debug)?;
                {
                    let mut current_map = NativeSummonMap {
                        tx: &mut current_tx, authority: &current_authority, room, objects: &objects,
                        stamp: current, tiles: BTreeMap::new(), path: None,
                    };
                    assert_eq!(current_map.read_tile(&runtime, parent_before_item.position()).await, None,
                        "actual Item without admitted flags cannot retain an empty-tile answer");
                }
                let (after_item, _) = run_current_native_monster_thinks(
                    &mut runtime, &mut states.monster_melee.creature_ai, &mut states.native_summons,
                    content, 2_000_000, &decision_root, &revisions,
                    &candidates, room, &objects, &mut current_tx, &current_authority, &mut budget, false,
                ).await.map_err(debug)?;
                let refused = after_item.iter().find(|(actor, _)| *actor == parent)
                    .ok_or("parent's next real think")?;
                assert_eq!(refused.1.refusal, Some(crate::ai_think::profile_schedule::ScheduleError::InvalidSummonFacts));
                assert!(refused.1.summon_proposals.is_empty());
                assert_eq!(runtime.read_actor_position(parent).map_err(debug)?, parent_before_item);
                assert_eq!(runtime.native_summon_count(parent, None), 1);
                assert_eq!(states.monster_melee.creature_ai.len(), 2);
                current_tx.rollback().await?;
                Ok(())
            })
        }));
        let cleanup = executor.block_on(harness.cleanup());
        match result {
            Ok(result) => {
                cleanup?;
                result
            }
            Err(payload) => {
                if cleanup.is_err() {
                    eprintln!("summon current-map fixture cleanup failed after test panic");
                }
                std::panic::resume_unwind(payload)
            }
        }
    }

    /// The existing bundle spawn adapter qualifies this explicit test-authored Table against
    /// the installed production rat. Thalom remains the independently current Map/Item reader;
    /// the bundle fixture supplies only source spawn data, never Map authority.
    #[test]
    fn current_owner_spawn_home_walkback_and_leash_use_actual_source_cell()
    -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, configured_admin, debug, id};
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        let executor = crate::bestiary_postgres_harness::runtime()?;
        let harness = executor.block_on(Harness::create(admin, "native_source_home", false))?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            executor.block_on(async {
                let world =
                    WorldId::decode(&id(crate::bestiary_postgres_harness::WORLD)).map_err(debug)?;
                let channel = ChannelId::decode(&id(crate::bestiary_postgres_harness::CHANNEL))
                    .map_err(debug)?;
                let (controller, pin) =
                    crate::content::native_gameplay::tests::activated_with_loot_tables(
                        world,
                        Some(include_bytes!(
                            "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                        )),
                    );
                let content = controller
                    .active()
                    .ok_or("active native generation")?
                    .native_gameplay()
                    .ok_or("decoded native content")?;
                let room = pin.qualified_room();
                assert!(room.source_world().is_some());
                let cell = |x, y| MovementLocalPosition { x, y, floor: -7 };
                let start = cell(2, 12);
                let channel_pin = ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    world,
                    channel,
                    NodeId::decode(harness.node.fact().node_id().as_bytes()).map_err(debug)?,
                    1,
                    1,
                    1,
                    "runtime-scope-assignment:1",
                    8,
                    channel_pin,
                )
                .map_err(debug)?;
                content
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let key = "oteryn:creature.rat";
                let maximum_health = runtime.companion_policy(key).map_err(debug)?.maximum_health;
                let table = Table {
                    creatures: vec![Creature {
                        key: key.into(),
                        period: Period::All,
                    }],
                    sources: vec![Source {
                        key: "oteryn:spawn/current-home-fixture".into(),
                        floor: 7,
                        x: 4,
                        y: 12,
                        points: vec![Point {
                            creature: 0,
                            x: 4,
                            y: 12,
                            direction: Direction::South,
                            respawn_ms: 30_000,
                        }],
                    }],
                };
                let bundle = crate::map::boot::tests::door_world()
                    .map_err(debug)?
                    .with_spawns_for_test(table);
                assert_eq!(
                    crate::node::serve::realize_bundle_spawns(
                        &mut runtime,
                        &bundle,
                        &oteryn_foundation::CancellationToken::new(),
                    )
                    .await
                    .map_err(debug)?,
                    Some((1, 1))
                );
                let (_, home, live) = runtime.spawn_point_creatures()[0];
                let actor = live.ok_or("realized exact creature")?;
                assert_eq!(home, cell(4, 12));
                assert!(runtime.companion_snapshot(actor).is_ok());
                // Stage an away position through the existing physical position owner before
                // AI admission. Its admission anchor is deliberately different from source home.
                for destination in [cell(3, 12), cell(2, 12)] {
                    let before = runtime.read_actor_position(actor).map_err(debug)?;
                    runtime
                        .borrow_movement_position()
                        .commit_cardinal(before, destination)
                        .map_err(debug)?;
                }
                let mut states = ChannelSpellStates::default();
                admit_native_creature_ai(
                    &runtime,
                    &mut states.monster_melee.creature_ai,
                    content,
                    actor,
                    0,
                )
                .map_err(debug)?;
                assert_eq!(
                    states
                        .monster_melee
                        .creature_ai
                        .state(actor)
                        .ok_or("AI state")?
                        .movement_anchor(),
                    cell(2, 12)
                );
                let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
                let generation = runtime.binding().scope_generation();
                let object_fence =
                    crate::world_runtime::ScopeContentGenerationFence::for_activation(
                        scope,
                        generation,
                        crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                            .map_err(debug)?,
                    );
                let objects = crate::world_runtime::bind_native_entry_door(
                    room.door(),
                    &object_fence,
                    scope,
                    generation,
                )
                .map_err(debug)?;
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let recovery = harness
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                    world, channel,
                )
                .await;
                let mut tx = harness.pool.begin().await?;
                let authority = item_tx::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    &permit,
                    &harness.root,
                    &recovery,
                    &harness.node,
                    scope,
                    generation.get(),
                )
                .await
                .map_err(debug)?;
                let root = GameplayDecisionRoot::from_bytes(content.source_digest());
                let revisions = crate::ability::RevisionSet::new(
                    "rules-r1",
                    "content-r1",
                    "world-r1",
                    "formula-r1",
                    "sim-r1",
                )
                .map_err(debug)?;
                let mut budget = crate::combat_attack::ChaseSearchBudget::default();
                let (chosen, next) = run_current_native_monster_thinks(
                    &mut runtime,
                    &mut states.monster_melee.creature_ai,
                    &mut states.native_summons,
                    content,
                    0,
                    &root,
                    &revisions,
                    &[],
                    room,
                    &objects,
                    &mut tx,
                    &authority,
                    &mut budget,
                    false,
                )
                .await
                .map_err(debug)?;
                assert_eq!(chosen.len(), 1);
                assert!(
                    !chosen[0].1.idle,
                    "walk-back remains awake despite no perceived player"
                );
                assert_eq!(chosen[0].1.step, None);
                assert_eq!(next, Some(0));
                let movement = states
                    .monster_melee
                    .creature_ai
                    .native_movement(actor)
                    .ok_or("source-bound movement")?;
                assert_eq!(movement.home, Some(home));
                assert_eq!(
                    movement.goal,
                    crate::ai_think::NativeCreatureGoal::WalkBack { home }
                );
                assert_eq!(
                    movement
                        .path
                        .as_ref()
                        .ok_or("adopted home route")?
                        .steps()
                        .count(),
                    2
                );
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    cell(2, 12)
                );
                let full = budget.remaining(50_000);
                let (stepped, _) = run_current_native_monster_thinks(
                    &mut runtime,
                    &mut states.monster_melee.creature_ai,
                    &mut states.native_summons,
                    content,
                    50_000,
                    &root,
                    &revisions,
                    &[],
                    room,
                    &objects,
                    &mut tx,
                    &authority,
                    &mut budget,
                    false,
                )
                .await
                .map_err(debug)?;
                assert!(stepped.is_empty());
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    cell(3, 12)
                );
                assert_eq!(
                    budget.remaining(50_000),
                    full,
                    "retained source-home route consumes no second search"
                );
                let movement = states
                    .monster_melee
                    .creature_ai
                    .native_movement(actor)
                    .ok_or("retained movement")?;
                assert_eq!(
                    movement
                        .path
                        .as_ref()
                        .ok_or("remaining route")?
                        .steps()
                        .count(),
                    1
                );
                let due = movement.cadence.next_due().ok_or("next paced step")?.get();
                let (_, next) = run_current_native_monster_thinks(
                    &mut runtime,
                    &mut states.monster_melee.creature_ai,
                    &mut states.native_summons,
                    content,
                    due,
                    &root,
                    &revisions,
                    &[],
                    room,
                    &objects,
                    &mut tx,
                    &authority,
                    &mut budget,
                    false,
                )
                .await
                .map_err(debug)?;
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    home
                );
                assert_eq!(next, None);
                assert_eq!(states.monster_melee.creature_ai.due(actor), None);
                assert_eq!(
                    states.monster_melee.creature_ai.earliest_native_step_due(),
                    None
                );

                for blocked_home in [false, true] {
                    // This deliberately stages the out-of-leash boundary through real owner
                    // position commits. It is not evidence of legal Map traversal out of Thalom.
                    for x in 5..=55 {
                        let before = runtime.read_actor_position(actor).map_err(debug)?;
                        runtime
                            .borrow_movement_position()
                            .commit_cardinal(before, cell(x, 12))
                            .map_err(debug)?;
                    }
                    let far = runtime.read_actor_position(actor).map_err(debug)?;
                    if blocked_home {
                        let _ = runtime
                            .admit_source_pinned_lab_creature(home, key, maximum_health)
                            .map_err(debug)?;
                    }
                    let now = due.saturating_add(if blocked_home { 2_000_000 } else { 1_000_000 });
                    admit_native_creature_ai(
                        &runtime,
                        &mut states.monster_melee.creature_ai,
                        content,
                        actor,
                        now,
                    )
                    .map_err(debug)?;
                    let (_, next) = run_current_native_monster_thinks(
                        &mut runtime,
                        &mut states.monster_melee.creature_ai,
                        &mut states.native_summons,
                        content,
                        now,
                        &root,
                        &revisions,
                        &[],
                        room,
                        &objects,
                        &mut tx,
                        &authority,
                        &mut budget,
                        false,
                    )
                    .await
                    .map_err(debug)?;
                    let after = runtime.read_actor_position(actor).map_err(debug)?;
                    if blocked_home {
                        assert_eq!(after, far, "occupied actual home idles in place");
                    } else {
                        assert_eq!(
                            after.position(),
                            home,
                            "one real leash relocation to authored home"
                        );
                    }
                    assert_eq!(next, None);
                    assert_eq!(
                        states.monster_melee.creature_ai.earliest_native_step_due(),
                        None
                    );
                    assert_eq!(runtime.spawn_point_creatures()[0].1, home);
                }
                tx.rollback().await?;
                Ok(())
            })
        }));
        let cleanup = executor.block_on(harness.cleanup());
        match result {
            Ok(result) => {
                cleanup?;
                result
            }
            Err(payload) => {
                if cleanup.is_err() {
                    eprintln!("native source-home fixture cleanup failed after test panic");
                }
                std::panic::resume_unwind(payload)
            }
        }
    }

    /// SPAWN-1a (Codex 4239269255): with no native gameplay manifest the fixture rats still
    /// respawn on the owner turn; the drive reads no melee content.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_fixture_rat_respawns_without_native_gameplay_content() {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )
        .expect("runtime");
        let source = room.spawn_source().expect("D116 spawn source");
        assert_eq!(
            runtime.realize_activation_spawns(vec![source.activation_facts()]),
            Ok(2)
        );
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        assert!(runtime.kill_creature_for_test(live.expect("live rat")));
        drive_owner_spawns(&mut runtime, 0);
        let due = runtime.pending_respawn(point).expect("pending").due_micros;
        drive_owner_spawns(&mut runtime, due);
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        // Admission runs 4,200 ms after the warning (CREATURE-AI-0 §6.3).
        drive_owner_spawns(&mut runtime, due + 4_200_000);
        assert!(runtime.spawn_point_creatures()[0].2.is_some());
    }

    /// End to end through the activation path: the bundle's spawn family is realized by
    /// `realize_bundle_spawns` against the production generation's served rat, a real lethal
    /// native hit kills the creature, and the owner-turn drive (no session involved) warns at
    /// the respawn deadline and admits the replacement 4,200 ms later.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_bundle_spawned_rat_killed_by_a_lethal_hit_respawns_on_the_owner_turn() {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        // The production generation on the Thalom source world; its rat is the served policy.
        let (controller, pin) = crate::content::native_gameplay::tests::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 11, -7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            channel_pin,
        )
        .expect("runtime");
        native
            .install_companion_policies(&mut runtime)
            .expect("policies");

        // One bundle source with one rat point on an open Thalom cell (native floor 7 is z -7).
        let table = Table {
            creatures: vec![Creature {
                key: "oteryn:creature.rat".into(),
                period: Period::All,
            }],
            sources: vec![Source {
                key: "oteryn:spawn/e2e".into(),
                floor: 7,
                x: 1,
                y: 11,
                points: vec![Point {
                    creature: 0,
                    x: 1,
                    y: 11,
                    direction: Direction::South,
                    respawn_ms: 30_000,
                }],
            }],
        };
        let bundle = crate::map::boot::tests::door_world()
            .expect("bundle world")
            .with_spawns_for_test(table);
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("executor");
        let realized = executor.block_on(crate::node::serve::realize_bundle_spawns(
            &mut runtime,
            &bundle,
            &oteryn_foundation::CancellationToken::new(),
        ));
        assert_eq!(realized.ok(), Some(Some((1, 1))));
        let (point, _, live) = runtime.spawn_point_creatures()[0];
        let rat = live.expect("the realized rat is live");
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .expect("rat policy")
            .maximum_health;

        // A real lethal native hit (the target identity is read from the slot).
        runtime
            .commit_monster_lab_damage(rat, b"e2e-lethal-hit", health)
            .expect("lethal hit");
        assert!(runtime.spawn_point_creatures()[0].2.is_none());

        drive_owner_spawns(&mut runtime, 0);
        let due = runtime.pending_respawn(point).expect("pending").due_micros;
        assert_eq!(due, 30_000_000);
        drive_owner_spawns(&mut runtime, due);
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        drive_owner_spawns(&mut runtime, due + 4_200_000);
        let replacement = runtime.spawn_point_creatures()[0].2.expect("respawned rat");
        assert_ne!(replacement, rat);
    }

    /// SPAWN-CEILINGS-1 (Codex 4243189401): thinks left due by `CREATUREAI0-RL-05` resume in the
    /// next owner window, not one think interval later, and only one pass per interval swings.
    #[test]
    fn a_think_backlog_resumes_in_the_next_owner_window_and_swings_keep_their_interval() {
        use crate::ai_think::{
            CREATURE_THINK_INTERVAL_MILLIS, CREATUREAI0_THINK_WINDOW_MICROS,
            CREATUREAI0_THINKS_PER_WINDOW,
        };
        let interval = CREATURE_THINK_INTERVAL_MILLIS * 1_000;
        // No think pending sooner: the next swing pass.
        assert_eq!(
            next_monster_ai_pass_us(1_234_567, 2_234_567, None),
            2_234_567
        );
        assert_eq!(
            next_monster_ai_pass_us(1_234_567, 2_234_567, Some(2_234_567)),
            2_234_567
        );
        // A think still due now: the start of the next owner window.
        assert_eq!(
            next_monster_ai_pass_us(1_234_567, 2_234_567, Some(1_000_000)),
            1_250_000
        );
        assert_eq!(
            next_monster_ai_pass_us(1_250_000, 2_234_567, Some(1_250_000)),
            1_300_000
        );
        // A think due before the next swing pass: its own time.
        assert_eq!(
            next_monster_ai_pass_us(1_234_567, 2_234_567, Some(1_600_000)),
            1_600_000
        );
        // N thinks due at once run 1,024 per owner window, oldest due first; each think is due
        // again one interval later. Returns the passes until every first think ran, the last
        // pass's time, the swing passes and the next pass after it.
        let simulate = |total: usize| {
            let mut due = std::collections::BTreeMap::from([(0_u64, total)]);
            let (mut first, mut now, mut swing_due) = (total, 0_u64, 0_u64);
            let (mut passes, mut swings) = (0_usize, 0_usize);
            loop {
                passes += 1;
                if now >= swing_due {
                    swing_due = now + interval;
                    swings += 1;
                }
                let mut budget = CREATUREAI0_THINKS_PER_WINDOW;
                while budget > 0 {
                    let Some(mut entry) = due.first_entry() else {
                        break;
                    };
                    if *entry.key() > now {
                        break;
                    }
                    let (key, taken) = (*entry.key(), (*entry.get()).min(budget));
                    *entry.get_mut() -= taken;
                    if *entry.get() == 0 {
                        entry.remove();
                    }
                    budget -= taken;
                    if key == 0 {
                        first -= taken;
                    }
                    *due.entry(now + interval).or_default() += taken;
                }
                let next = next_monster_ai_pass_us(now, swing_due, due.keys().next().copied());
                if first == 0 {
                    return (passes, now, swings, next);
                }
                assert_eq!(next, now + CREATUREAI0_THINK_WINDOW_MICROS);
                now = next;
            }
        };
        // 83,000 (an activation's worth): ceil(83,000 / 1,024) = 82 windows, about 4 s, not
        // 82 s; swings stay one pass per interval (0, 1, 2, 3 and 4 s) and, with more due than
        // RL-05 serves in an interval, the backlog keeps the next pass in the next window.
        let (passes, now, swings, next) = simulate(83_000);
        assert_eq!(passes, 83_000_usize.div_ceil(CREATUREAI0_THINKS_PER_WINDOW));
        assert_eq!(now, 81 * CREATUREAI0_THINK_WINDOW_MICROS);
        assert_eq!(swings, 5);
        assert_eq!(next, now + CREATUREAI0_THINK_WINDOW_MICROS);
        // 15,000: 15 windows, one swing, then the next pass is the first cohort's next think.
        let (passes, now, swings, next) = simulate(15_000);
        assert_eq!(passes, 15);
        assert_eq!(now, 14 * CREATUREAI0_THINK_WINDOW_MICROS);
        assert_eq!(swings, 1);
        assert_eq!(next, interval);
    }

    /// SPAWN-CEILINGS-1 (Codex 4242381774): a pass swings every targeting monster up to
    /// `CREATUREAI0-RL-05`, not the old 8, and above it the cursor serves the rest next.
    #[test]
    fn a_pass_swings_every_targeting_monster_up_to_the_think_budget() {
        use crate::ai_think::CREATUREAI0_THINKS_PER_WINDOW;
        assert_eq!(swing_window(0, 5), (0, 0));
        for len in [65, 256, 257, 300] {
            assert_eq!(swing_window(len, 0), (0, len));
            assert_eq!(swing_window(len, len + 3), (3, len));
        }
        assert_eq!(
            swing_window(CREATUREAI0_THINKS_PER_WINDOW, 0),
            (0, CREATUREAI0_THINKS_PER_WINDOW)
        );
        let len = CREATUREAI0_THINKS_PER_WINDOW + 1;
        let (cursor, count) = swing_window(len, 0);
        assert_eq!((cursor, count), (0, CREATUREAI0_THINKS_PER_WINDOW));
        // The monster left over swings first in the next pass.
        assert_eq!(
            swing_window(len, cursor + count),
            (CREATUREAI0_THINKS_PER_WINDOW, CREATUREAI0_THINKS_PER_WINDOW)
        );
    }

    /// CREATURE-MOVE-1: a creature steps only onto an open walkable ordinary cell.
    #[test]
    fn creature_steps_are_admitted_only_onto_open_walkable_cells() {
        assert!(cell_open(true, false, false, Some((false, false))));
        assert!(
            !cell_open(false, false, false, Some((false, false))),
            "blocked"
        );
        assert!(
            !cell_open(true, true, false, Some((false, false))),
            "closed door"
        );
        // `doors` holds every door cell, open or closed, so an open doorway is refused too: a
        // creature standing there would not count in the door-close occupancy.
        assert!(
            !cell_open(true, true, false, Some((false, false))),
            "open door cell (CREATURE-DOOR-1)"
        );
        assert!(
            !cell_open(true, false, false, Some((true, false))),
            "protection zone"
        );
        assert!(
            !cell_open(true, false, false, Some((false, true))),
            "floor change"
        );
        // WORLD-INTERACTION-0 §6.5: monsters never enter a teleport tile.
        assert!(
            !cell_open(true, false, true, Some((false, false))),
            "teleport"
        );
        assert!(!cell_open(true, false, false, None), "unknown tile facts");
    }

    /// NEWHAVEN-TELEPORT-1 (WORLD-INTERACTION-0 §6.5): on a bundle World a creature never steps
    /// onto a teleport tile, while an ordinary grass tile of the same bundle admits it.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_creature_never_steps_onto_a_bundle_teleport() {
        use crate::foundation::{ChannelId, WorldId};
        use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let (bytes, load) = teleport_bundle();
        let booted = crate::map::boot::boot(&bytes, &floor_pins(load), world, channel, floor_items)
            .expect("boot");
        let cells = booted
            .movement_cells(room.movement_cells())
            .expect("bundle cells");
        let none = std::collections::BTreeSet::new();
        let at = |x, y| MovementLocalPosition { x, y, floor: 8 };
        assert!(creature_cell_admissible(&cells, &none, at(2, 0)));
        assert!(!creature_cell_admissible(&cells, &none, at(0, 1)));
        assert!(crate::map::floor::is_teleport(
            &cells,
            LogicalCell { x: 0, y: 1, z: 8 }
        ));
        assert!(!crate::map::floor::is_teleport(
            &cells,
            LogicalCell { x: 2, y: 0, z: 8 }
        ));
    }

    /// CREATURE-MOVE-1: a cell outside the qualified room is refused whatever the doors say.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_cell_outside_the_room_is_refused() {
        use crate::foundation::WorldId;
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let outside = MovementLocalPosition {
            x: 100_000,
            y: 0,
            floor: 0,
        };
        assert!(!creature_cell_admissible(
            room.movement_cells(),
            &std::collections::BTreeSet::new(),
            outside
        ));
    }

    /// The accepted entry room's real cells and its real door runtime, bound as `node/serve.rs`
    /// binds it at activation.
    #[allow(clippy::expect_used)]
    fn entry_room_and_door(
        world: crate::foundation::WorldId,
        channel: crate::foundation::ChannelId,
    ) -> (
        QualifiedNativeEntryRoom,
        crate::world_runtime::LocalObjectRuntime,
    ) {
        let room = crate::content::qualify_native_entry_room(world).expect("entry room");
        let door_content = room.door().clone();
        let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
        let generation = crate::foundation::ScopeOwnershipGeneration::new(1).expect("generation");
        let fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
            scope,
            generation,
            crate::world_runtime::ReferenceContentGeneration::from_content(&door_content)
                .expect("door content generation"),
        );
        let door =
            crate::world_runtime::bind_native_entry_door(&door_content, &fence, scope, generation)
                .expect("door runtime");
        (room, door)
    }

    fn cell(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 0 }
    }

    /// CREATURE-MOVE-1 (Codex 4242014304), ENTRY-SPELL-TILES-1: the accepted entry room qualifies
    /// a tile for every placed cell, none a protection-zone or floor-change tile; its ordinary
    /// Walkable cells admit a creature step, while its Blocked cell, its door cell (closed or
    /// open) and cells outside it refuse.
    #[test]
    #[allow(clippy::expect_used)]
    fn the_accepted_entry_room_admits_creature_steps_onto_its_ordinary_cells() {
        use crate::foundation::{ChannelId, WorldId};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let (room, mut door) = entry_room_and_door(world, channel);
        let cells = room.movement_cells();
        assert!(accepted_entry_movement(cells));
        let east = LogicalCell { x: 1, y: 0, z: 0 };
        let flags = cells
            .spell_tiles()
            .lookup(cells.scope(), east)
            .expect("the accepted entry room qualifies its tiles")
            .flags();
        assert!(!flags.protection_zone && !flags.floor_change);
        assert_eq!(
            cells
                .spell_tiles()
                .lookup(cells.scope(), LogicalCell { x: 3, y: 0, z: 0 })
                .err(),
            Some(crate::content::SpellTileLookupError::Absent),
            "a cell outside the room has no tile"
        );
        let closed = door.collision_cells().clone();
        assert_eq!(
            closed.iter().copied().collect::<Vec<_>>(),
            vec![LogicalCell { x: 1, y: -1, z: 0 }],
            "the entry door cell"
        );
        for (x, y) in [(0, 0), (1, 0), (2, 0), (2, -1)] {
            assert!(
                creature_cell_admissible(cells, &closed, cell(x, y)),
                "ordinary cell ({x}, {y})"
            );
        }
        assert!(
            !creature_cell_admissible(cells, &closed, cell(0, -1)),
            "Blocked"
        );
        assert!(
            !creature_cell_admissible(cells, &closed, cell(1, -1)),
            "closed door"
        );
        assert!(
            !creature_cell_admissible(cells, &closed, cell(3, 0)),
            "outside"
        );
        // Opening the door leaves its collision footprint, the set the AI pass reads, unchanged.
        assert!(matches!(
            door.attempt_use(0, &std::collections::BTreeSet::new()),
            Ok(crate::world_runtime::LocalObjectUseOutcome::Committed { .. })
        ));
        let open = door.collision_cells().clone();
        assert_eq!(open, closed);
        assert!(
            !creature_cell_admissible(cells, &open, cell(1, -1)),
            "open door cell (CREATURE-DOOR-1)"
        );
        // Without the door set the doorway is ordinary Walkable terrain: the door set alone
        // refuses it.
        assert!(creature_cell_admissible(
            cells,
            &std::collections::BTreeSet::new(),
            cell(1, -1)
        ));
    }

    /// BUNDLE-MONSTER-AI-1: on a bundle World the step rule reads the bundle's tile facts and its
    /// own collision index: an ordinary walkable cell admits a creature step, while a protection
    /// zone, water, a wall, a floor-change stair and cells outside the bundle refuse. The
    /// bundle cells are recognised, the entry room's are not.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_bundle_admits_creature_steps_only_onto_ordinary_walkable_cells() {
        use crate::map::spell_tiles::tests::{id, world};
        let world_id = crate::foundation::WorldId::decode(&id(1)).expect("world");
        let entry = crate::content::qualify_native_entry_room(world_id).expect("entry room");
        let cells = world()
            .movement_cells(entry.movement_cells())
            .expect("bundle cells");
        assert!(bundle_cells(&cells) && !bundle_cells(entry.movement_cells()));
        let none = std::collections::BTreeSet::new();
        let at = |x| MovementLocalPosition { x, y: 0, floor: 7 };
        for x in [1, 13] {
            assert!(creature_cell_admissible(&cells, &none, at(x)), "grass {x}");
        }
        for (x, why) in [
            (2, "protection zone"),
            (3, "water"),
            (4, "wall"),
            (5, "floor-change stair"),
            (0, "outside"),
            (14, "outside"),
        ] {
            assert!(!creature_cell_admissible(&cells, &none, at(x)), "{why}");
        }
    }

    /// CREATURE-MOVE-1 (Codex 4242014304): in the accepted entry room the spawned rats, thinking
    /// with the room's own rat behaviour, commit a real chase step through `run_due_thinks`: the
    /// den rat steps west onto the east cell; the den-north rat's chase step is the door cell and
    /// is refused, so it stays.
    #[test]
    #[allow(clippy::expect_used)]
    fn a_spawned_rat_commits_a_chase_step_in_the_accepted_entry_room() {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        let (room, door) = entry_room_and_door(world, channel);
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )
        .expect("runtime");
        let source = room.spawn_source().expect("D116 spawn source");
        assert_eq!(
            runtime.realize_activation_spawns(vec![source.activation_facts()]),
            Ok(2)
        );
        let rat_at = |runtime: &ChannelRuntimeV1, at: MovementLocalPosition| {
            runtime
                .spawn_point_creatures()
                .into_iter()
                .find_map(|(_, _, live)| {
                    live.filter(|rat| {
                        runtime
                            .read_actor_position(*rat)
                            .is_ok_and(|p| p.position() == at)
                    })
                })
                .expect("a spawned rat on the cell")
        };
        let den = rat_at(&runtime, cell(2, 0));
        let den_north = rat_at(&runtime, cell(2, -1));
        let session =
            GameSessionId::decode(&[1, 0, 0, 0, 0, 4, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 4])
                .expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let player = runtime.commit_fresh_session(reservation).expect("player");
        runtime
            .initialize_pinned_test_position(player, cell(0, 0))
            .expect("player on the entry start");

        // The room's own hostile rat behaviour, its swing cleared as the AI pass clears it.
        let room_source: serde_json::Value =
            serde_json::from_slice(crate::content::NATIVE_ENTRY_ROOM_SOURCE).expect("room source");
        let profile = room_source["authoring_profiles"]
            .as_array()
            .expect("authoring profiles")
            .iter()
            .find(|profile| profile["target"]["key"] == "oteryn:behavior/rat-hostile")
            .expect("rat behaviour");
        let mut behavior: ProjectV2BehaviorAuthoring =
            serde_json::from_value(profile["data"]["profile"].clone()).expect("behaviour");
        behavior.attacks.clear();
        behavior.defenses.clear();
        let mut table = crate::ai_think::CreatureAiTable::default();
        for rat in [den, den_north] {
            table
                .admit_creature(
                    &runtime,
                    rat,
                    Some(&behavior),
                    &std::collections::BTreeMap::new(),
                    0,
                )
                .expect("admitted");
        }
        let candidates = [TargetCandidate {
            actor: player,
            position: cell(0, 0),
            eligible: true,
            invisible: false,
            health: 0,
            damage: 0,
        }];
        let revisions = crate::ability::RevisionSet::new(
            "ruleset:ai-v1",
            "content:test",
            "world:ai-v1",
            "formula:source-melee-baseline",
            "simulation:v1",
        )
        .expect("revisions");
        let doors = door.collision_cells().clone();
        let cells = room.movement_cells();
        let reports = table.run_due_thinks(
            &mut runtime,
            0,
            &GameplayDecisionRoot::from_bytes([9; 32]),
            &revisions,
            &candidates,
            |destination| creature_cell_admissible(cells, &doors, destination),
        );
        assert_eq!(reports.len(), 2, "both rats thought");
        let step = |rat| {
            reports
                .iter()
                .find(|(actor, _)| *actor == rat)
                .and_then(|(_, report)| report.step)
                .map(|step| step.destination)
        };
        assert_eq!(step(den), Some(cell(1, 0)), "chase step west");
        assert_eq!(step(den_north), None, "the chase step is the door cell");
        assert_eq!(
            runtime
                .read_actor_position(den)
                .expect("den rat")
                .position(),
            cell(1, 0),
            "the step was committed"
        );
        assert_eq!(
            runtime
                .read_actor_position(den_north)
                .expect("den-north rat")
                .position(),
            cell(2, -1)
        );
    }

    #[test]
    fn remote_players_do_not_consume_local_perception_budget() {
        let mut targets = vec![(position(100, 7), false, false); 65];
        targets.push((position(1, 7), false, false));
        assert_eq!(
            local_target_indices(position(0, 7), false, targets),
            Some(vec![65])
        );
        assert!(
            local_target_indices(
                position(0, 7),
                false,
                vec![(position(1, 7), false, false); 65]
            )
            .is_none()
        );
    }
    #[test]
    fn protected_nearest_and_unseen_players_do_not_block_legal_target() {
        let targets = vec![
            (position(1, 7), false, true),
            (position(1, 7), true, false),
            (position(1, 8), false, false),
            (position(2, 7), false, false),
        ];
        assert_eq!(
            local_target_indices(position(0, 7), false, targets.clone()),
            Some(vec![3])
        );
        assert_eq!(
            local_target_indices(position(0, 7), true, targets),
            Some(vec![1, 3])
        );
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn lethal_player_remains_observable_but_cannot_hide_the_next_living_target() {
        use crate::ability::creature_bite::CreatureBiteVitals;
        use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests as player};
        let (mut runtime, dead, dead_session) = player::runtime_with_player(86);
        let living_session = player::runtime_with_player(87).2;
        let reservation = runtime
            .reserve_fresh_session(living_session)
            .expect("second player");
        let living = runtime
            .commit_fresh_session(reservation)
            .expect("live actor");
        runtime
            .initialize_movement_test_position(dead, position(1, 7))
            .expect("first position");
        runtime
            .initialize_movement_test_position(living, position(2, 7))
            .expect("second position");
        let mut states = ChannelSpellStates::default();
        let now = SemanticTimeMicros::from_micros(0);
        let initialized_at = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0);
        for (actor, session) in [(dead, dead_session), (living, living_session)] {
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    player::FACTS,
                    (0, 0),
                    initialized_at,
                )
                .expect("actual current vitals owner");
        }
        let hit = states
            .apply_creature_damage(
                &mut runtime,
                dead,
                dead_session,
                player::FACTS.max_health,
                now,
            )
            .expect("actual lethal vitals write");
        assert_eq!(hit.damage.health_after, 0);
        assert!(hit.death.is_some());
        assert!(
            states.get(&runtime, dead, dead_session).is_some(),
            "death remains observable"
        );
        assert!(current_player_target(&runtime, &states, dead, dead_session).is_none());
        assert!(current_player_target(&runtime, &states, living, living_session).is_some());
        let eligible = [(dead, dead_session), (living, living_session)]
            .into_iter()
            .filter(|(actor, session)| {
                current_player_target(&runtime, &states, *actor, *session).is_some()
            })
            .map(|(actor, _)| {
                (
                    runtime
                        .read_actor_position(actor)
                        .expect("current position")
                        .position(),
                    false,
                    false,
                )
            });
        assert_eq!(
            local_target_indices(position(0, 7), false, eligible),
            Some(vec![0])
        );
    }
}
