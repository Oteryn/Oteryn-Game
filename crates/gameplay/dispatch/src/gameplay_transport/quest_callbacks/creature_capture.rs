//! The native monster target owner supplies exact actors; the Channel source
//! scheduler commits the quest and projects conditions and delayed relocation.
use super::*;
use crate::durability::{
    character_progression::CharacterProgressionError,
    quest_state::creature_capture::QualifiedCreatureCapture,
};
use oteryn_gameplay_domain::domain::condition::{ConditionValues, SpeedRange, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{ApplicationFacts, ConditionDefinition};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

const MAX_CAPTURE_WORK: usize = 64;

#[derive(Clone)]
struct Capture {
    source: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    attack_at: u64,
    teleport_at: Option<u64>,
    proof: QualifiedCreatureCapture,
    rule: world_callbacks::creature_capture::CreatureCapture,
}

#[derive(Default)]
pub(super) struct Captures {
    pending: Vec<Capture>,
    cooldowns: Vec<(ExactActorRef, u64)>,
}

impl Captures {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.pending
            .iter()
            .map(|capture| capture.teleport_at.unwrap_or(capture.attack_at))
            .min()
    }
}

fn visible_target(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    target: ExactActorRef,
    session: GameSessionId,
    now: u64,
) -> bool {
    !states.is_dead(target)
        && states.get(runtime, target, session).is_some_and(|player| {
            !player
                .owned_conditions()
                .has_status(StatusKind::Invisible, now)
        })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn queue_creature_capture(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) {
        let Ok(creature) = runtime.current_live_creature_identity(source) else {
            return;
        };
        let Some(callback) = self.callback_content().iter().find(|callback| {
            callback
                .effects
                .creature_capture
                .as_ref()
                .is_some_and(|rule| rule.creature.as_bytes() == creature)
        }) else {
            return;
        };
        let Some(rule) = &callback.effects.creature_capture else {
            return;
        };
        let Some(catalogue) = &self.quest_catalogue else {
            return;
        };
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return;
        };
        if native.source_digest() != runtime.content_pin().server_artifact_digest()
            || native
                .spell_appearances()
                .and_then(|a| a.for_look_type(u32::from(rule.look_type)))
                .is_none()
            || !visible_target(runtime, states, target, session, now)
            || self.bundle_map().is_none_or(|map| {
                map.overlay
                    .base()
                    .tile(rule.landing.x, rule.landing.y, rule.landing.floor)
                    .is_none()
            })
        {
            return;
        }
        let Ok(mut captures) = self.states.quest_callbacks.captures.lock() else {
            return;
        };
        captures
            .cooldowns
            .retain(|(caster, _)| runtime.contains_live_creature(*caster));
        if captures.pending.len() >= MAX_CAPTURE_WORK
            || captures
                .pending
                .iter()
                .any(|pending| pending.target == target)
        {
            return;
        }
        let Some(next) = now.checked_add(u64::from(rule.interval_ms) * 1_000) else {
            return;
        };
        if let Some(index) = captures
            .cooldowns
            .iter()
            .position(|(caster, _)| *caster == source)
        {
            if captures.cooldowns[index].1 > now {
                return;
            }
        } else {
            if captures.cooldowns.len() < MAX_CAPTURE_WORK {
                captures.cooldowns.push((source, next));
            }
            return;
        }
        let Some(proof) = QualifiedCreatureCapture::prepare(
            runtime, catalogue, callback, source, target, session, now,
        ) else {
            return;
        };
        if let Some((_, due)) = captures
            .cooldowns
            .iter_mut()
            .find(|(caster, _)| *caster == source)
        {
            *due = next;
        }
        captures.pending.push(Capture {
            source,
            target,
            session,
            attack_at: now,
            teleport_at: None,
            proof,
            rule: rule.clone(),
        });
        states.owner_wake.notify_one();
    }

    pub(super) async fn drain_creature_captures(&self) {
        let now = self.owner_now().get();
        let ready = self
            .states
            .quest_callbacks
            .captures
            .lock()
            .map(|captures| {
                captures
                    .pending
                    .iter()
                    .filter(|capture| capture.teleport_at.is_none_or(|due| due <= now))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for capture in ready {
            if let Some(due) = capture.teleport_at {
                if self.project_capture_landing(&capture, due).await {
                    self.update_capture(&capture, None);
                }
                continue;
            }
            let Some(catalogue) = self.quest_catalogue.clone() else {
                return;
            };
            let fence = match self.current_quest_fence(capture.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => {
                    self.update_capture(&capture, None);
                    continue;
                }
                Err(()) => return,
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let runtime = self.runtime.lock().await;
            let now = self.owner_now().get();
            let eligible = {
                let states = self.states.spell_states.lock().await;
                // Crystal mission05_web_terror.lua:88-103 and spider_queen_wrap.lua:23-30.
                // A queued cast resolves only against a current, visible combat target.
                visible_target(&runtime, &states, capture.target, capture.session, now)
                    && capture
                        .proof
                        .target_is_current(&runtime, &capture.rule, capture.source, now)
            };
            if !eligible {
                self.update_capture(&capture, None);
                continue;
            }
            let result = slot
                .commit_creature_capture(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    capture.proof.clone(),
                    catalogue,
                )
                .await;
            drop(runtime);
            drop(slot);
            match result {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => {
                    self.refresh_quest_session(capture.session, false).await;
                    self.project_capture_conditions(&capture).await;
                    self.update_capture(
                        &capture,
                        capture
                            .attack_at
                            .checked_add(u64::from(capture.rule.teleport_delay_ms) * 1_000),
                    );
                }
                Err(
                    CharacterProgressionError::Unavailable(_)
                    | CharacterProgressionError::CharacterRevisionMismatch,
                ) => return,
                _ => self.update_capture(&capture, None),
            }
        }
    }

    fn update_capture(&self, capture: &Capture, teleport_at: Option<u64>) {
        if let Ok(mut captures) = self.states.quest_callbacks.captures.lock()
            && let Some(index) = captures.pending.iter().position(|work| {
                work.target == capture.target && work.attack_at == capture.attack_at
            })
        {
            if let Some(due) = teleport_at {
                captures.pending[index].teleport_at = Some(due);
            } else {
                captures.pending.remove(index);
            }
        }
    }

    async fn project_capture_conditions(&self, capture: &Capture) {
        let Some(native_content) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return;
        };
        let Some(appearance) = native_content
            .spell_appearances()
            .and_then(|a| a.for_look_type(u32::from(capture.rule.look_type)))
        else {
            return;
        };
        let (Some(slow), Some(outfit)) = (
            ConditionDefinition::new(
                "oteryn:condition.quest-capture.slow",
                1,
                ConditionValues::Speed {
                    paralysis: true,
                    range: SpeedRange {
                        a_min: capture.rule.speed[0],
                        b_min: 0,
                        a_max: capture.rule.speed[1],
                        b_max: 0,
                    },
                    duration_ms: capture.rule.condition_ms,
                },
            ),
            ConditionDefinition::new(
                "oteryn:condition.quest-capture.outfit",
                1,
                ConditionValues::Outfit {
                    look_type: u32::from(capture.rule.look_type),
                    duration_ms: capture.rule.condition_ms,
                },
            )
            .and_then(|outfit| outfit.with_appearance(appearance.selection())),
        ) else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Some(base_speed) = states
            .get(&runtime, capture.target, capture.session)
            .and_then(|player| u16::try_from(player.owned_base_speed()).ok())
        else {
            return;
        };
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        let occurrence = capture.proof.occurrence();
        let decision_root = GameplayDecisionRoot::from_bytes(native_content.source_digest());
        let facts = ApplicationFacts {
            now: capture.attack_at,
            base_speed,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &decision_root,
            occurrence: DecisionOccurrenceId::from_bytes(occurrence),
        };
        let applied = states.native_composite_conditions(
            &runtime,
            capture.target,
            capture.session,
            &hex(&occurrence),
            capture.source,
            &[slow, outfit],
            &facts,
            &[],
            None,
            crate::player_lethal::PlayerHitKind::Mitigated,
            None,
            stamp,
        );
        if applied.is_some()
            && let Some(map) = self.bundle_map()
            && let (Ok(from), Ok(to)) = (
                runtime.read_actor_position(capture.source),
                runtime.read_actor_position(capture.target),
            )
            && let (Some(from), Some(to)) = (native(from.position()), native(to.position()))
        {
            use crate::map::facts::CreatureVisualCue;
            for cue in [
                CreatureVisualCue::effect(
                    &runtime,
                    stamp,
                    capture.source,
                    to,
                    capture.rule.combat_effect,
                ),
                CreatureVisualCue::missile(
                    &runtime,
                    stamp,
                    capture.source,
                    from,
                    to,
                    capture.rule.projectile,
                ),
            ]
            .into_iter()
            .flatten()
            {
                map.facts.emit_creature_visual(&runtime, cue);
            }
        }
        self.states
            .chat
            .push_quest_feedback(capture.session, &capture.rule.message);
    }

    async fn project_capture_landing(&self, capture: &Capture, now: u64) -> bool {
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let (actor, session) = (capture.target, capture.session);
        if states.is_dead(actor) || runtime.player_control_facts(actor, session).is_err() {
            return true;
        }
        let Ok(current) = runtime.read_actor_position(actor) else {
            return true;
        };
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            current,
            local(capture.rule.landing),
            false,
            &doors,
        ) else {
            return true;
        };
        let Ok(landed) = runtime.commit_source_step(proof) else {
            return true;
        };
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now,
            Some(current.position()),
            landed.position(),
            None,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now,
            underwater(map, current.position()),
            underwater(map, landed.position()),
        );
        true
    }
}
