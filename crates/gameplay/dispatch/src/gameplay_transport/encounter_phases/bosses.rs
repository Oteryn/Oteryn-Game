//! Source creature phases bound to the same admission and exact actor generations as the room.
use super::*;
use crate::map::{boot::BundleMap, facts::MovementSpeech};
use oteryn_gameplay_entities::carrier::runtime_actor_companion::CompanionSnapshot;
use oteryn_gameplay_quests::quest::world_callbacks::Encounter;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct BossFight {
    room: Encounter,
    transformation: Option<ExactActorRef>,
    replacement: Option<ExactActorRef>,
    shield: Option<Shield>,
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    cells: crate::content::NativeEntryMovementCells,
}

#[derive(Debug)]
struct Shield {
    actor: ExactActorRef,
    prompt: Option<usize>,
    lives_used: u8,
    next_reply_us: u64,
}

impl BossFight {
    pub(in crate::gameplay_transport) fn bind(
        runtime: &ChannelRuntimeV1,
        room: &Encounter,
        actors: &[ExactActorRef],
        cells: crate::content::NativeEntryMovementCells,
    ) -> Result<Self, CarrierError> {
        let phases = room
            .boss_phases
            .as_ref()
            .ok_or(CarrierError::PlanConflict)?;
        let exact = |key: &str| {
            actors
                .iter()
                .copied()
                .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
                .ok_or(CarrierError::CreatureTargetMismatch)
        };
        let transformation = phases
            .transformation
            .as_ref()
            .map(|rule| {
                let _ = runtime.companion_policy(&rule.replacement)?;
                exact(&rule.creature)
            })
            .transpose()?;
        let replacement = phases
            .death_replacement
            .as_ref()
            .map(|rule| {
                let _ = runtime.companion_policy(&rule.replacement)?;
                exact(&rule.creature)
            })
            .transpose()?;
        let shield = phases
            .speech_shield
            .as_ref()
            .map(|rule| {
                for summon in &rule.summons {
                    let _ = runtime.companion_policy(summon)?;
                }
                Ok::<_, CarrierError>(Shield {
                    actor: exact(&rule.creature)?,
                    prompt: None,
                    lives_used: 0,
                    next_reply_us: 0,
                })
            })
            .transpose()?;
        Ok(Self {
            room: room.clone(),
            transformation,
            replacement,
            shield,
            scope: runtime.encounter_owner_scope(),
            cells,
        })
    }

    pub(in crate::gameplay_transport) fn active(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.scope == runtime.encounter_owner_scope()
            && [
                self.transformation,
                self.replacement,
                self.shield.as_ref().map(|shield| shield.actor),
            ]
            .into_iter()
            .flatten()
            .any(|actor| runtime.contains_live_creature(actor))
    }

    fn tick(&mut self, runtime: &mut ChannelRuntimeV1, map: &BundleMap) {
        let Some(phases) = &self.room.boss_phases else {
            return;
        };
        // Crystal library_area/creaturescripts_ghulosh.lua:19-36. Carry the
        // exact HP and position across the profile change. The replacement's
        // donor monster has no ghuloshThink registration.
        if let Some(rule) = &phases.transformation
            && let Some(actor) = self.transformation
            && let Ok(before) = runtime.companion_snapshot(actor)
            && below(&before, rule.at_percent)
            && let Ok(change) =
                runtime.prepare_creature_profile_transformation(&before, &rule.replacement)
            && runtime
                .commit_creature_profile_transformation(change)
                .is_ok()
        {
            self.transformation = None;
            say(runtime, map, actor, &rule.speech);
        }

        // Crystal grand_master_oberon.lua:125-136 and
        // grand_master_oberon_functions.lua:43-60.
        let (Some(shield), Some(rule)) = (&mut self.shield, &phases.speech_shield) else {
            return;
        };
        if shield.lives_used == rule.lives {
            return;
        }
        let Ok(before) = runtime.companion_snapshot(shield.actor) else {
            return;
        };
        if !below(&before, rule.at_percent) {
            return;
        }
        let draw = |label, count| choose(shield.actor, shield.lives_used, label, count);
        let prompt = draw(0, rule.prompts.len());
        let summon = &rule.summons[draw(1, rule.summons.len())];
        if runtime
            .set_boss_speech_shield(&before, Some(rule.immunity_effect))
            .is_err()
        {
            return;
        }
        shield.prompt = Some(prompt);
        shield.lives_used += 1;
        say(runtime, map, shield.actor, &rule.prompts[prompt].speech);
        // The donor asks and heals even when createMonster cannot allocate.
        let _ = spawn(runtime, summon, before.position);
    }

    fn projected_death(&mut self, runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
        if self.scope != runtime.encounter_owner_scope() || self.replacement != Some(actor) {
            return;
        }
        self.replacement = None;
        let Some(rule) = self
            .room
            .boss_phases
            .as_ref()
            .and_then(|p| p.death_replacement.as_ref())
        else {
            return;
        };
        // Crystal library_area/creaturescripts_ghulosh.lua:52-60. This birth
        // is immediate and belongs to the exact Book admitted by the lever.
        let Ok(dead) = runtime.companion_snapshot_including_dead(actor) else {
            return;
        };
        if dead.state.policy.definition_key == rule.creature
            && placement(&self.cells, runtime, dead.position)
        {
            let _ = spawn(runtime, &rule.replacement, dead.position);
        }
    }

    fn hear(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        speaker: MovementLocalPosition,
        text: &str,
        now: u64,
    ) {
        let (Some(shield), Some(rule)) = (
            &mut self.shield,
            self.room
                .boss_phases
                .as_ref()
                .and_then(|p| p.speech_shield.as_ref()),
        ) else {
            return;
        };
        if self.scope != runtime.encounter_owner_scope()
            || !super::super::quest_callbacks::room_contains(&self.room, speaker)
            || now < shield.next_reply_us
        {
            return;
        }
        let Ok(before) = runtime.companion_snapshot(shield.actor) else {
            return;
        };
        if !same_speech_area(speaker, before.position) {
            return;
        }
        // Crystal grand_master_oberon.lua:138-158 throttles all same-boss SAY
        // occurrences, and keeps the last Asking index after removing immunity.
        let response = shield.prompt.map(|prompt| {
            rule.prompts[prompt]
                .answers
                .iter()
                .any(|answer| text.eq_ignore_ascii_case(answer))
        });
        if response == Some(true)
            && runtime.boss_speech_shield_effect(shield.actor).is_some()
            && runtime.set_boss_speech_shield(&before, None).is_err()
        {
            return;
        }
        shield.next_reply_us = now.saturating_add(u64::from(rule.cooldown_ms) * 1_000);
        if let Some(accepted) = response {
            let speech = if accepted {
                &rule.accepted_speech
            } else {
                &rule.rejected_speech
            };
            say(runtime, map, shield.actor, speech);
        }
    }
}

fn same_speech_area(speaker: MovementLocalPosition, listener: MovementLocalPosition) -> bool {
    use oteryn_gameplay_foundation::foundation::speech::{ChatPosition, SpeechMode};
    use oteryn_gameplay_social::chat::line::local_listeners;
    let position = |p: MovementLocalPosition| {
        Some(ChatPosition {
            x: p.x,
            y: p.y,
            floor: p.floor.try_into().ok()?,
        })
    };
    let (Some(speaker), Some(listener)) = (position(speaker), position(listener)) else {
        return false;
    };
    !local_listeners(SpeechMode::Say, speaker, [((), listener)]).is_empty()
}

fn below(actor: &CompanionSnapshot, percent: u8) -> bool {
    actor.health > 0
        && i128::from(actor.health) * 100 <= i128::from(actor.maximum_health) * i128::from(percent)
}

fn choose(actor: ExactActorRef, life: u8, label: u8, count: usize) -> usize {
    let digest = Sha256::new()
        .chain_update(b"oteryn:encounter-boss-phase:v1")
        .chain_update(actor.placement_identity())
        .chain_update(actor.actor_local_generation().to_le_bytes())
        .chain_update([life, label])
        .finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    crate::spell::uniform_draw(u64::from_le_bytes(bytes), 0, count as i64 - 1) as usize
}

fn spawn(
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    at: MovementLocalPosition,
) -> Result<(), CarrierError> {
    let health = runtime.companion_policy(key)?.maximum_health;
    runtime.commit_quest_creature_spawn(&[(key, health, at)])?;
    Ok(())
}

fn say(runtime: &ChannelRuntimeV1, map: &BundleMap, actor: ExactActorRef, text: &str) {
    if let Some(speech) = MovementSpeech::capture_monster(runtime, actor, 36, text) {
        map.facts.emit_movement_speech(speech);
    }
}

impl Registry {
    pub(in crate::gameplay_transport) fn tick_bosses(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
    ) {
        if let Ok(mut fights) = self.bosses.lock() {
            fights.retain_mut(|fight| {
                if !fight.active(runtime) {
                    return false;
                }
                fight.tick(runtime, map);
                true
            });
        }
    }

    pub(super) fn boss_death(&self, runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
        if let Ok(mut fights) = self.bosses.lock() {
            for fight in fights.iter_mut() {
                fight.projected_death(runtime, actor);
            }
        }
    }

    pub(in crate::gameplay_transport) fn boss_hears(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        speaker: ExactActorRef,
        text: &str,
        now: u64,
    ) {
        let Ok(position) = runtime.read_actor_position(speaker) else {
            return;
        };
        if let Ok(mut fights) = self.bosses.lock() {
            for fight in fights.iter_mut() {
                fight.hear(runtime, map, position.position(), text, now);
            }
        }
    }
}
