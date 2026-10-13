//! First playable wild-monster dispatch: explicitly activated physical melee only.
//! Uses the actual positioned census, bound Creature policies and existing AI-4
//! owner. Summons, ranged/defensive/custom spells and movement remain disabled.
use super::ComposedFreshAdmission;
use super::actor_spell::ChannelSpellStates;
use super::attack::ChannelAttackStates;
use crate::ability::creature_bite::ReentryProtection;
use crate::ai::{AiProvenance, AiProvenanceInput, ResourceLimit};
use crate::ai_monster_melee::{Dispatch, MeleeDefinition, think_target_only};
use crate::ai_think::targeting::TargetCandidate;
use crate::ai_think::{AttackReadiness, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId};
use crate::combat::charm_effects::CharmHookEvent;
use crate::content::QualifiedNativeEntryRoom;
use crate::content::{LogicalCell, ProjectV2AuthoringProfileData, ProjectV2BehaviorAuthoring};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One coalesced creature think pass. The same Channel's scheduler is shared by
    /// all connections; player count never multiplies monster attack frequency.
    pub(in crate::gameplay_transport) async fn drain_monster_melee(&self) {
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        // SPAWN-1a (CREATURE-AI-0 §6.3): the respawn occurrences due on this owner turn run
        // before the creature think pass, whether or not melee content is active.
        drive_owner_spawns(&mut runtime, now.get());
        // BUNDLE-MONSTER-AI-1: the pass reads the combat room, the entry room on the fixture
        // World and its bundle view (bundle cells and map revision) on a bundle World.
        let Some(room) = self.combat_room else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
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
        // ATTACK-1b: lock order runtime, spell states, attack.
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
                let Some(monster) = qualified_melee(&runtime, content, *actor) else {
                    continue;
                };
                // The think projects targeting only: the swing stays this AI-4 owner's, and
                // native content carries no Ability authoring for the think's proposals yet.
                let mut targeting_only = monster.behavior.clone();
                targeting_only.attacks.clear();
                targeting_only.defenses.clear();
                let _ = melee_owner.creature_ai.admit_creature(
                    &runtime,
                    *actor,
                    Some(&targeting_only),
                    &std::collections::BTreeMap::new(),
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
        // CREATURE-MOVE-1: a think's one cardinal step (chase, flee or wander within the spawn
        // radius) is committed here when its destination is a walkable, open, ordinary cell.
        // Lock order: runtime, spell states, attack, then the door runtime (read only).
        // Every door cell, open or closed: a creature never stands in a doorway (CREATURE-DOOR-1).
        // The entry door is bound in the entry frame: it is no obstacle on a bundle World, whose
        // doors are Unknown tiles that refuse a step.
        let cells = room.movement_cells();
        let doors = if bundle_cells(cells) {
            std::collections::BTreeSet::new()
        } else {
            self.door.lock().await.collision_cells().clone()
        };
        let (_, next_think) = melee_owner.creature_ai.run_due_thinks_until_next(
            &mut runtime,
            now.get(),
            &GameplayDecisionRoot::from_bytes(content.source_digest()),
            &revisions,
            &candidates,
            |destination| creature_cell_admissible(cells, &doors, destination),
        );
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

/// CREATURE-MOVE-1 (CREATURE-AI-0 §5.3, R2): whether a creature may step onto `cell`. It is a
/// Walkable cell of the qualified room, no door cell (open or closed) covers it, and it is neither a
/// protection-zone nor a floor-change tile. A creature never opens a door and never changes
/// floors by a step. The check reads only; the step is committed by the Channel owner.
///
/// LIMIT (CREATURE-MOVE-1c): durable ground items and fields (Magic Wall, Wild Growth) are not
/// consulted here. No SQL, spell lane or durable-item snapshot enters the AI pass; the step uses
/// only what the Movement owner holds in memory. CREATURE-MOVE-1c adds an in-memory blocking
/// overlay owned by the Movement owner, revalidated atomically at the step commit.
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
        tile,
    )
}

/// Whether `cells` move by the accepted entry movement: the entry-room collision index (not a
/// source world, not a bundle) outside the source profile. Its tiles are qualified from the room's
/// own optional tile document, so `Unknown` there means only "no document"; a source-qualified
/// map's `Unknown` names a genuinely unknown cell and a bundle's lookup is another scope.
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
const fn cell_open(walkable: bool, door: bool, tile: Option<(bool, bool)>) -> bool {
    match tile {
        Some((protection_zone, floor_change)) => {
            walkable && !door && !protection_zone && !floor_change
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
    fn position(x: i32, floor: i16) -> MovementLocalPosition {
        MovementLocalPosition { x, y: 0, floor }
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
        assert!(cell_open(true, false, Some((false, false))));
        assert!(!cell_open(false, false, Some((false, false))), "blocked");
        assert!(!cell_open(true, true, Some((false, false))), "closed door");
        // `doors` holds every door cell, open or closed, so an open doorway is refused too: a
        // creature standing there would not count in the door-close occupancy.
        assert!(
            !cell_open(true, true, Some((false, false))),
            "open door cell (CREATURE-DOOR-1)"
        );
        assert!(
            !cell_open(true, false, Some((true, false))),
            "protection zone"
        );
        assert!(!cell_open(true, false, Some((false, true))), "floor change");
        assert!(!cell_open(true, false, None), "unknown tile facts");
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
