//! The hallowed axe acts on the four retained weak points in the Demon Oak arena.
use super::*;
use crate::gameplay_transport::{item_ref_admission::pinned_index, world_map::MapObjectTarget};
use oteryn_gameplay_quests::quest::demon_oak as rules;
use oteryn_protocol_oteryn::{
    world_map::MapDefinition,
    world_object::{UseIntent, UseTarget},
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

const WEAK_POINTS: [(u16, u16, u8, &str); 4] = [
    (32716, 32349, 1, "oteryn:item.tibia.i914"),
    (32714, 32351, 2, "oteryn:item.tibia.i915"),
    (32718, 32351, 1, "oteryn:item.tibia.i916"),
    (32716, 32352, 1, "oteryn:item.tibia.i917"),
];
const SUMMON_POINTS: [(i32, i32); 8] = [
    (32714, 32348),
    (32712, 32349),
    (32711, 32351),
    (32713, 32354),
    (32716, 32354),
    (32719, 32354),
    (32721, 32351),
    (32719, 32348),
];

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

fn weak_point(map: &BundleMap, resolved: &MapObjectTarget<()>) -> Option<(usize, &'static str)> {
    let at = resolved.pos;
    if at.floor != -7 || map.facts.house_tile(at) {
        return None;
    }
    WEAK_POINTS
        .iter()
        .enumerate()
        .find_map(|(point, &(x, y, ordinal, key))| {
            (at.x == x && at.y == y && resolved.ordinal == ordinal).then_some((point, key))
        })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_demon_oak_axe(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        resolved: &MapObjectTarget<()>,
        intent: &UseIntent,
        source: Option<[u8; 16]>,
    ) -> Option<UseOutcome> {
        let (point, definition) = weak_point(map, resolved)?;
        let session = command.game_session_id;
        let target = resolved.pos;
        let target_entry = {
            let tile = map.overlay.base().tile(target.x, target.y, target.floor)?;
            let (id, _) = tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(resolved.ordinal))?;
            map.facts.base_entry(target, resolved.ordinal, *id)?
        };
        let revision = map.facts.object_revision(resolved.placement_key);
        let _permit = self.spell_lane_permit().await?;
        let (index, expected) = {
            let runtime = self.runtime.lock().await;
            let index = pinned_index(&runtime, self.active_generation)?;
            let MapDefinition::Item(reference) = target_entry.definition else {
                return None;
            };
            if index.definition_at(reference)?.0 != definition {
                return None;
            }
            let expected = runtime.read_actor_position(actor).ok()?;
            let position = expected.position();
            if position.floor != 7
                || position
                    .x
                    .abs_diff(i32::from(target.x))
                    .max(position.y.abs_diff(i32::from(target.y)))
                    > 1
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Some(outcome(UseDisposition::NotPossible));
            }
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, session)
                || !self.states.player_trades.physical(session, None)
            {
                return Some(UseOutcome::rejected());
            }
            (index, expected)
        };
        let fence = self.current_quest_fence(session).await.ok()??;
        if command.item_fence != Some(fence.item_fence()) {
            return Some(UseOutcome::rejected());
        }
        let axe = index.definition_ref("Item", rules::AXE, "definition-r1")?;
        if let UseTarget::ItemByDefinition(wanted) = intent.target
            && wanted != axe
        {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence.item_fence())
            .await
            .ok()?;
        let has_axe = inventory
            .equipment
            .iter()
            .chain(
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item)),
            )
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .any(|item| {
                index.definition_ref(
                    &item.definition.family,
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                ) == Some(axe)
                    && match intent.target {
                        UseTarget::Item(_) => source == Some(item.item_instance_id),
                        UseTarget::ItemByDefinition(wanted) => wanted == axe,
                        _ => false,
                    }
            });
        if !has_axe {
            return Some(outcome(UseDisposition::NothingToUse));
        }
        let catalogue = self.quest_catalogue.as_ref()?;
        let prior = self
            .root
            .read_character_quest_state(self.character, fence.character_id)
            .await
            .ok()?;
        let progress =
            rules::CUT_TRACKS.map(|track| prior.tracks().get(track).copied().unwrap_or(0));
        let defeated = progress.iter().all(|&cuts| cuts == rules::FINISHED_CUTS);
        let command = CommandRef::new(session, CommandId::new(command.command_id).ok()?);
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let kick = MovementLocalPosition {
            x: 32716,
            y: 32340,
            floor: 7,
        };
        let teleport = {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(expected)
                || map.facts.base_hidden(resolved.placement_key)
                || map.facts.object_revision(resolved.placement_key) != revision
            {
                return Some(UseOutcome::rejected());
            }
            if defeated {
                let occupied = runtime
                    .current_live_creature_census()
                    .ok()?
                    .into_iter()
                    .any(|monster| {
                        runtime.native_summon_master(monster).is_none()
                            && runtime
                                .companion_snapshot(monster)
                                .map_or(true, |snapshot| snapshot.state.master.is_none())
                            && runtime.read_actor_position(monster).is_ok_and(|snapshot| {
                                let at = snapshot.position();
                                at.floor == 7
                                    && at.x.abs_diff(32716) <= 9
                                    && at.y.abs_diff(32351) <= 6
                            })
                    });
                if occupied {
                    self.states
                        .chat
                        .push_quest_feedback(session, "Clear the remaining monsters first.");
                    return Some(outcome(UseDisposition::NotPossible));
                }
                Some(
                    crate::map::floor::prepare_scripted_quest_teleport_facing(
                        &runtime,
                        self.movement_cells(),
                        actor,
                        session,
                        expected,
                        kick,
                        false,
                        &closed,
                        None,
                        true,
                    )
                    .ok()?,
                )
            } else {
                if progress[point] >= rules::FINISHED_CUTS {
                    map.facts.emit_movement_effect(target, 3);
                    return Some(outcome(UseDisposition::NotPossible));
                }
                None
            }
        };
        let committed = slot
            .commit_quest_transition_with_admission(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: if defeated {
                        rules::FINISH
                    } else {
                        rules::CUT_TRANSITIONS[point]
                    }
                    .into(),
                    cause: QuestCause::Use(command),
                },
                std::sync::Arc::clone(catalogue),
                &self.premium_coordinator,
            )
            .await;
        drop(slot);
        let receipt = match committed {
            Ok(QuestTransitionOutcome::Committed(receipt)) => receipt,
            Ok(QuestTransitionOutcome::AlreadyCommitted(_)) => {
                return Some(outcome(UseDisposition::Committed));
            }
            _ => return Some(UseOutcome::rejected()),
        };
        self.apply_committed_quest_copy(session, true, &receipt);
        self.schedule_quest_refresh(session);
        let mut runtime = self.runtime.lock().await;
        if runtime.read_actor_position(actor) == Ok(expected)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            if let Some(teleport) = teleport {
                if runtime.commit_source_step(teleport).is_ok() {
                    self.states
                        .chat
                        .push_quest_feedback(session, "Report your victory to Oldrak.");
                }
            } else {
                let final_cut = progress.iter().sum::<i64>() == 4 * rules::FINISHED_CUTS - 1;
                Self::spawn_oak_wave(
                    &mut runtime,
                    map,
                    actor,
                    command,
                    point,
                    progress[point].max(1),
                    final_cut,
                );
                map.facts.emit_movement_effect(target, 1);
                let mut states = self.states.spell_states.lock().await;
                if let Some(position) = native(expected.position()) {
                    super::area_damage::oak_earth_hit(
                        &runtime,
                        &mut states,
                        actor,
                        session,
                        map,
                        position,
                        (170, 210),
                        oak_draw(
                            runtime.content_pin().server_artifact_digest(),
                            actor,
                            session,
                            command.command_id().get(),
                            "demon_oak.cut_damage",
                            0,
                        ),
                        self.owner_now(),
                    );
                }
            }
        }
        Some(outcome(UseDisposition::Committed))
    }

    fn spawn_oak_wave(
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        actor: ExactActorRef,
        command: CommandRef,
        point: usize,
        cut: i64,
        final_cut: bool,
    ) {
        let root = runtime.content_pin().server_artifact_digest();
        let draw = |index| {
            oak_draw(
                root,
                actor,
                command.game_session_id(),
                command.command_id().get(),
                "demon_oak.retaliation",
                index,
            )
        };
        let Some(roll) = draw(0) else { return };
        let wave: &[&str] = match (point, cut, final_cut) {
            (_, 10, true) => &["oteryn:creature.demon"],
            (0, 5, _) => &[
                "oteryn:creature.spectre",
                "oteryn:creature.blightwalker",
                "oteryn:creature.braindeath",
                "oteryn:creature.demon",
            ],
            (0, 10, _) => &["oteryn:creature.betrayed_wraith"; 2],
            (1, 5, _) => &[
                "oteryn:creature.plaguesmith",
                "oteryn:creature.plaguesmith",
                "oteryn:creature.blightwalker",
            ],
            (1, 10, _) => &[
                "oteryn:creature.dark_torturer",
                "oteryn:creature.blightwalker",
            ],
            (2, 5, _) => &[
                "oteryn:creature.banshee",
                "oteryn:creature.plaguesmith",
                "oteryn:creature.hellhound",
            ],
            (2, 10, _) => &["oteryn:creature.grim_reaper"],
            (3, 5, _) => &[
                "oteryn:creature.plaguesmith",
                "oteryn:creature.hellhound",
                "oteryn:creature.hellhound",
            ],
            (3, 10, _) => &[
                "oteryn:creature.undead_dragon",
                "oteryn:creature.hand_of_cursed_fate",
            ],
            _ if crate::spell::uniform_draw(roll, 1, 100) >= 90 => {
                &["oteryn:creature.bonebeast"; 4]
            }
            _ => &[],
        };
        for (index, &key) in (1..).zip(wave) {
            let Some(value) = draw(index) else { break };
            let (x, y) = SUMMON_POINTS[(value & 7) as usize];
            if map.overlay.base().tile(x as u16, y as u16, -7).is_none() {
                continue;
            }
            let Ok(policy) = runtime.companion_policy(key) else {
                continue;
            };
            let at = MovementLocalPosition { x, y, floor: 7 };
            let _ = runtime.commit_quest_creature_spawn(&[(key, policy.maximum_health, at)]);
        }
    }
}

pub(super) fn oak_draw(
    root: [u8; 32],
    actor: ExactActorRef,
    session: GameSessionId,
    ordinal: u64,
    domain: &str,
    draw: u64,
) -> Option<u64> {
    let root = GameplayDecisionRoot::from_bytes(root);
    let digest = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    deterministic_decision_u64(&root, DecisionOccurrenceId::from_bytes(bytes), domain, draw).ok()
}
