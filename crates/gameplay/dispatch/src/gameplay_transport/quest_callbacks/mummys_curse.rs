//! Crystal's canopic jars and Horestis tomb, bound to the World owner.
use super::super::world_map::MapObjectTarget;
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_world_change::mummys_curse::{
    BROKEN_JAR, HORESTIS_SPAWNS, INTACT_JAR, JARS, JarOutcome,
};
use sha2::{Digest, Sha256};

const BOSS: &str = "oteryn:creature.horestis";
const WALL_X: std::ops::RangeInclusive<u16> = 32941..=32944;
const WALL_Y: u16 = 32754;
const WALL_FLOOR: i8 = -12;
const ORIGINAL_WALL: u16 = 3514;
const CREATED_WALL: u16 = 1603;

pub(super) struct TombTimers {
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    second_spawn: Option<u64>,
    original_wall: Option<u64>,
    created_wall: Option<u64>,
}

impl TombTimers {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        [self.second_spawn, self.original_wall, self.created_wall]
            .into_iter()
            .flatten()
            .min()
    }
}

fn entries(
    map: &BundleMap,
    pos: TilePos,
) -> impl Iterator<Item = (u64, crate::map::view::EntryFacts)> + '_ {
    map.overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .into_iter()
        .flat_map(|tile| tile.ids().iter().zip(tile.depths()))
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(move |(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            Some((
                placement_key(pos, ordinal)?,
                map.facts.base_entry(pos, ordinal, *id)?,
            ))
        })
}

pub(super) fn source_action(map: &BundleMap, resolved: &MapObjectTarget<()>) -> Option<u16> {
    let pos = resolved.pos;
    if map.facts.house_tile(pos) {
        return None;
    }
    let &(action, ..) = JARS.iter().find(|&&(action, x, y, floor)| {
        (pos.x, pos.y, pos.floor) == (x, y, floor)
            || map
                .facts
                .quest_callback_matches(resolved.placement_key, None, Some(action), None)
    })?;
    let (_, facts) = entries(map, pos).find(|(key, _)| *key == resolved.placement_key)?;
    matches!(facts.appearance_id, INTACT_JAR | BROKEN_JAR).then_some(action)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_mummys_curse(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &MapObjectTarget<()>,
        action: u16,
        revision: u64,
    ) -> UseOutcome {
        let session = command.game_session_id;
        let refusal = || {
            self.states
                .chat
                .push_quest_feedback(session, "Sorry, not possible.");
            UseOutcome::rejected()
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return refusal();
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return refusal();
        };
        if command.item_fence != Some(fence.item_fence()) {
            return refusal();
        }
        let _slot = self.revision_sequencer.acquire(fence.character_id).await;
        let mut runtime = self.runtime.lock().await;
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return refusal();
        };
        let Some(at) = native(expected.position()) else {
            return refusal();
        };
        let pos = target.pos;
        if at.floor != pos.floor || at.x.abs_diff(pos.x) > 1 || at.y.abs_diff(pos.y) > 1 {
            return UseOutcome {
                disposition: UseDisposition::TooFar,
                ..UseOutcome::rejected()
            };
        }
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || map.facts.object_revision(target.placement_key) != revision
            || !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return refusal();
        }
        let Some(after) = map
            .facts
            .quest_callback_replacement(target.placement_key, BROKEN_JAR)
        else {
            return refusal();
        };
        let context = QuestWorldContext {
            party: Vec::new(),
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                hex(&runtime.content_pin().server_artifact_digest())
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: self.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: expected.revision(),
        };
        let digest = Sha256::new()
            .chain_update(b"oteryn.mummys_curse.use.v1")
            .chain_update(session.as_bytes())
            .chain_update(command.command_id.to_be_bytes())
            .chain_update(action.to_be_bytes())
            .finalize();
        let mut sample = [0; 8];
        sample.copy_from_slice(&digest[..8]);
        let roll = (((u128::from(u64::from_be_bytes(sample)) * 10) >> 64) as u8) + 1;
        let Ok((outcome, progress)) = self
            .root
            .use_mummys_curse_jar(
                self.character,
                self.holder,
                &permit,
                fence,
                context,
                action,
                roll,
            )
            .await
        else {
            return refusal();
        };
        if action < JARS[0].0 + u16::from(progress)
            && entries(map, pos).any(|(key, facts)| {
                key == target.placement_key && facts.appearance_id == INTACT_JAR
            })
        {
            map.facts.apply_quest_callback(
                target.placement_key,
                revision,
                after,
                &[],
                &[],
                false,
                None,
            );
        }
        let disposition = match outcome {
            JarOutcome::Broken { .. } => UseDisposition::Committed,
            JarOutcome::FinalJar => {
                self.states.chat.push_quest_feedback(session, "The horestis tomb was broken, the boss of the tomb will be reborn in 45 seconds. The barrier will be built from here 2 minutes.");
                self.open_horestis_tomb(&mut runtime, map);
                self.states
                    .spell_states
                    .lock()
                    .await
                    .owner_wake
                    .notify_one();
                UseDisposition::Committed
            }
            JarOutcome::Failed { .. } => {
                self.states.chat.push_quest_feedback(
                    session,
                    if digest[8] & 1 == 0 {
                        "Waaaaaah"
                    } else {
                        "You are too afraid to destroy this object"
                    },
                );
                map.facts.emit_movement_effect(at, 3);
                UseDisposition::Committed
            }
            JarOutcome::CoolingDown => {
                self.states
                    .chat
                    .push_quest_feedback(session, "You need wait 30 minutes to use again.");
                UseDisposition::Rejected
            }
            JarOutcome::Prerequisite => {
                let message = match action {
                    50007 => "You need to break the first jar.",
                    50008 => "You need to break the second jar.",
                    50009 => "You need to break the third jar.",
                    _ => "You need to break the fourth jar.",
                };
                self.states.chat.push_quest_feedback(session, message);
                UseDisposition::Rejected
            }
            JarOutcome::Refused => return refusal(),
        };
        UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }
    }

    fn open_horestis_tomb(&self, runtime: &mut ChannelRuntimeV1, map: &BundleMap) {
        let now = self.owner_now().get();
        for x in WALL_X {
            let pos = TilePos {
                x,
                y: WALL_Y,
                floor: WALL_FLOOR,
            };
            if let Some((key, facts)) =
                entries(map, pos).find(|(_, facts)| facts.appearance_id == ORIGINAL_WALL)
            {
                // Hide the base barrier by putting it in the source batch's passage set.
                map.facts.apply_quest_callback(
                    key,
                    map.facts.object_revision(key),
                    facts,
                    &[key],
                    &[],
                    false,
                    None,
                );
            }
        }
        self.spawn_horestis(runtime, map, HORESTIS_SPAWNS[0]);
        if let Ok(mut pending) = self.states.quest_callbacks.mummys_curse.lock() {
            *pending = Some(TombTimers {
                scope: runtime.encounter_owner_scope(),
                second_spawn: now.checked_add(HORESTIS_SPAWNS[1].3 * 1_000_000),
                original_wall: now.checked_add(20 * 60 * 1_000_000),
                created_wall: now.checked_add((61_000 + 6 * 20 * 1800) * 1000),
            });
        }
    }

    fn spawn_horestis(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        (x, y, floor, _): (u16, u16, i8, u64),
    ) {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        if native.install_companion_policies(runtime).is_err() {
            return;
        }
        let at = local(Position { x, y, floor });
        if self.movement_cells.index().lookup(
            self.movement_cells.scope(),
            LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) != Ok(CollisionClass::Walkable)
            || map
                .facts
                .quest_callback_blocked(map.overlay.base(), TilePos { x, y, floor }, false)
            || runtime.positioned_melee_census().map_or(true, |census| {
                census.iter().any(|(_, pos, _)| pos.position() == at)
            })
        {
            return;
        }
        if let Ok(policy) = runtime.companion_policy(BOSS) {
            let _ = runtime.commit_quest_creature_spawn(&[(BOSS, policy.maximum_health, at)]);
        }
    }

    pub(super) fn drain_mummys_curse(&self, runtime: &mut ChannelRuntimeV1, map: &BundleMap) {
        let Ok(mut pending) = self.states.quest_callbacks.mummys_curse.lock() else {
            return;
        };
        let Some(timers) = pending.as_mut() else {
            return;
        };
        if timers.scope != runtime.encounter_owner_scope() {
            *pending = None;
            return;
        }
        let now = self.owner_now().get();
        if timers.second_spawn.is_some_and(|due| due <= now) {
            timers.second_spawn = None;
            self.spawn_horestis(runtime, map, HORESTIS_SPAWNS[1]);
        }
        for (deadline, appearance) in [
            (&mut timers.original_wall, ORIGINAL_WALL),
            (&mut timers.created_wall, CREATED_WALL),
        ] {
            if deadline.is_some_and(|due| due <= now) {
                *deadline = None;
                for x in WALL_X {
                    map.facts.create_movement_item(
                        TilePos {
                            x,
                            y: WALL_Y,
                            floor: WALL_FLOOR,
                        },
                        appearance,
                        0,
                        || true,
                    );
                }
            }
        }
        if timers.next_due_us().is_none() {
            *pending = None;
        }
    }
}
