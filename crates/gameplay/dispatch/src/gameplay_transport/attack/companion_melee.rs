//! Player-owned creatures use their own qualified melee and independent clocks against the
//! master's current PvE target. The shared damage ledger owns reward attribution.

use super::*;
use crate::combat::CapturedRewardPrincipal;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
use oteryn_gameplay_combat_rules::resolution::{
    creature_mitigated, creature_physical_response, creature_physically_immune,
    swing_draw as damage_draw,
};

impl ChannelAttackStates {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn drain_owned_companion_melee(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        constants: &AttackConstants,
        charms: &crate::content::charm_source::CanonicalCharmCatalogue,
        now: SemanticTimeMicros,
    ) {
        self.companion_melee_due
            .retain(|(actor, _, _)| runtime.contains_live_creature(*actor));
        if !self
            .entries
            .iter()
            .any(|entry| entry.state.target().is_some())
        {
            return;
        }
        let Ok(census) = runtime.positioned_actor_census() else {
            return;
        };
        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
        for (companion, _, session) in census {
            if session.is_some() {
                continue;
            }
            let Ok(source) = runtime.companion_snapshot(companion) else {
                continue;
            };
            let Some(master) = source.state.master else {
                continue;
            };
            let Some(index) = self.index(master.actor, master.session) else {
                continue;
            };
            let entry = &self.entries[index];
            let Some(target) = entry.state.target() else {
                continue;
            };
            if states.is_dead(master.actor)
                || states.has_pending_spell_commit(master.actor, master.session)
                || runtime.assert_actor_spell_unreserved(master.actor).is_err()
                || runtime
                    .actor_staff_visibility(master.actor)
                    .is_ok_and(|flags| flags.0)
                || states.player_hidden_from(runtime, master.actor, target, now)
                || pacified_applied_since(
                    runtime,
                    states,
                    master.actor,
                    master.session,
                    entry.target_set_at,
                )
                || source
                    .state
                    .conditions
                    .attack_target_gate(now.get())
                    .is_err()
            {
                continue;
            }
            let Ok(victim) = runtime.companion_snapshot(target) else {
                continue;
            };
            let Some(facts) =
                target_facts(runtime, room, master.actor, master.session, target, now)
            else {
                continue;
            };
            if !facts.present_and_visible
                || !facts.attackable_kind
                || !facts.alive
                || facts.attacker_in_protection_zone
                || facts.target_in_protection_zone
                || facts.attacker_reentry_protected
                || facts.target_reentry_protected
                || in_protection_zone(room, source.position) != Some(false)
                || !victim.state.policy.flags.attackable
                || source.position.floor != victim.position.floor
                || source
                    .position
                    .x
                    .abs_diff(victim.position.x)
                    .max(source.position.y.abs_diff(victim.position.y))
                    > 1
                || creature_physically_immune(&victim.state.policy)
                || !runtime
                    .native_health_event(target)
                    .is_ok_and(|event| event.is_none())
            {
                continue;
            }
            let Ok(lease) = runtime.bound_attacker_lease(master.actor, master.session) else {
                continue;
            };
            let Some(melees) =
                super::super::monster_ai_cycle::qualified_owned_melee(runtime, native, companion)
            else {
                continue;
            };
            let (Ok(defense), Some(armor)) = (
                runtime.effective_creature_defense(&victim),
                victim.state.policy.armor,
            ) else {
                continue;
            };

            for melee in melees {
                let Some(profile) = melee
                    .source
                    .filter(|profile| profile.conditions().is_empty())
                else {
                    continue;
                };
                let definition = melee.definition;
                let ordinal = definition.entry_index;
                let clock = self
                    .companion_melee_due
                    .iter_mut()
                    .find(|(actor, index, _)| *actor == companion && *index == ordinal);
                let next = now.get().saturating_add(definition.interval_us);
                if let Some((_, _, due)) = clock {
                    if now.get() < *due {
                        continue;
                    }
                    *due = next;
                } else {
                    self.companion_melee_due.push((companion, ordinal, next));
                }
                let attacker = companion.placement_identity();
                let occurrence = (*master.session.as_bytes(), now.get());
                if !damage_draw(
                    &root,
                    attacker,
                    occurrence,
                    ordinal,
                    b"companion-chance",
                    "attack.chance",
                    1,
                    1_000_000,
                )
                .is_some_and(|chance| chance <= i64::from(definition.chance_ppm))
                {
                    continue;
                }
                let Some(damage) = damage_draw(
                    &root,
                    attacker,
                    occurrence,
                    ordinal,
                    b"companion-damage",
                    "attack.damage",
                    i64::from(definition.minimum),
                    i64::from(definition.maximum),
                ) else {
                    continue;
                };
                let (check_defense, check_armor) = profile.block_checks();
                let blocked = if check_defense {
                    let index = self
                        .blocks
                        .iter()
                        .position(|(actor, _)| *actor == target)
                        .unwrap_or_else(|| {
                            self.blocks
                                .push((target, BlockBudget::settled(&constants.block, now)));
                            self.blocks.len() - 1
                        });
                    self.blocks[index].1.try_block(&constants.block, now)
                } else {
                    false
                };
                let Some(magnitude) = creature_mitigated(
                    &root,
                    constants,
                    attacker,
                    occurrence,
                    ordinal,
                    damage,
                    blocked,
                    defense,
                    if check_armor { armor } else { 0 },
                )
                .and_then(|damage| creature_physical_response(&victim.state.policy, damage))
                .and_then(|damage| u32::try_from(damage).ok())
                .filter(|damage| *damage > 0) else {
                    continue;
                };
                let mut stream = b"companion-melee:".to_vec();
                stream.extend_from_slice(&companion.actor_local_generation().to_be_bytes());
                stream.extend_from_slice(&ordinal.to_be_bytes());
                let Ok(hit) = runtime.commit_source_damage_at_us(
                    target,
                    companion,
                    master.session,
                    &stream,
                    now.get(),
                    magnitude,
                    now.get(),
                ) else {
                    continue;
                };
                if hit.committed_health_loss().is_none() {
                    continue;
                }
                let entry = &mut self.entries[index];
                entry.state.record_hit(now, constants.in_fight_micros());
                if shared_experience::aggressive_monster(runtime, native, target) {
                    entry.record_shared_experience_activity(runtime, now);
                }
                if hit.health_after == 0 {
                    entry.clear_target();
                    record_creature_kill(
                        runtime,
                        KillSink {
                            prey: Some(states),
                            queue: &self.kills,
                            rewards: native.reward_table(),
                            activity: &self.shared_experience_cells(),
                            charms: Some(charms),
                        },
                        target,
                        CapturedRewardPrincipal {
                            character: lease.character_id(),
                            lease_generation: lease.generation(),
                            session: master.session,
                            actor: master.actor,
                        },
                        now.get() / 1_000,
                        Some(now),
                    );
                    break;
                }
            }
        }
    }
}
