//! Attached GazHaragothHeal: one real source marker and one retained 7s event.
use super::*;
use crate::content::native_monster_callbacks::QualifiedGazEvents;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::CallbackSpeech;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Heal,
}
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        128
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
#[derive(Debug)]
struct Work {
    key: u64,
    actor: ExactActorRef,
}
#[derive(Debug, Default)]
pub(super) struct NativeGaz {
    source: Option<QualifiedGazEvents>,
    timers: Option<OwnerTimerLane<Family, u64>>,
    work: Vec<Work>,
    seen: Vec<crate::foundation::ThinkOccurrence>,
    next: u64,
}
impl NativeGaz {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.timers
            .as_ref()
            .and_then(OwnerTimerLane::earliest_due)
            .map(|time| time.get())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one actual reserved occurrence shares current source, condition and physical HP owners"
    )]
    pub(super) fn think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        occurrences: &[crate::foundation::ThinkOccurrence],
        now: u64,
    ) -> Result<(), ()> {
        if self.work.is_empty()
            && !occurrences.iter().any(|o| {
                runtime.matches_live_creature_identity(o.actor, b"oteryn:creature.gaz_haragoth")
            })
        {
            return Ok(());
        }
        if !matches!(
            room.movement_cells().index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        ) {
            return Err(());
        }
        if self.source.is_none() {
            self.source = Some(native.gaz_events_source().map_err(|_| ())?);
        }
        let source = self.source.as_ref().ok_or(())?;
        if source.content() != native.source_digest()
            || source.content() != runtime.content_pin().server_artifact_digest()
        {
            return Err(());
        }
        if self.timers.is_none() {
            self.timers = Some(
                OwnerTimerLane::for_generation(
                    runtime.binding().runtime_scope(),
                    runtime.binding().scope_generation(),
                    [(
                        Family::Heal,
                        FamilyPolicy {
                            max_pending: 128,
                            catch_up: CatchUpPolicy::DeadlineState,
                        },
                    )],
                )
                .map_err(|_| ())?,
            );
        }
        for occurrence in occurrences {
            if !runtime
                .matches_live_creature_identity(occurrence.actor, source.parent().key.as_bytes())
                || self
                    .seen
                    .iter()
                    .any(|old| old.actor == occurrence.actor && old.sequence >= occurrence.sequence)
            {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(occurrence.actor)
                .map_err(|_| ())?;
            if snapshot.state.policy.definition_revision != source.parent().revision {
                return Err(());
            }
            let marker =
                oteryn_gameplay_domain::domain::condition::ConflictKey::SpellRegeneration(88888);
            if i128::from(snapshot.health) * 8 < i128::from(snapshot.maximum_health)
                && runtime
                    .actor_conditions_at(occurrence.actor, None, now)
                    .map_err(|_| ())?
                    .get(marker)
                    .is_none()
            {
                use crate::foundation::{
                    ActorConditionTransition, ApplicationFacts, ConditionDefinition,
                    ConditionSource, ConditionSourceKind, ConditionValues,
                };
                let definition = ConditionDefinition::new(
                    "source.monster.gaz.regeneration-marker.88888",
                    1,
                    ConditionValues::SpellRegeneration {
                        duration_ms: 7000,
                        sub_id: 88888,
                        health_gain: 0,
                        health_interval_ms: 7000,
                        mana_gain: 0,
                        mana_interval_ms: 1000,
                    },
                )
                .ok_or(())?;
                let digest = Sha256::new()
                    .chain_update(b"oteryn:gaz-source-heal:v1")
                    .chain_update(occurrence.actor.placement_identity())
                    .chain_update(occurrence.actor.actor_local_generation().to_be_bytes())
                    .chain_update(occurrence.sequence.to_be_bytes())
                    .finalize();
                let mut id = [0; 16];
                id.copy_from_slice(&digest[..16]);
                let root = GameplayDecisionRoot::from_bytes(native.source_digest());
                let condition_at =
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
                let condition = runtime
                    .prepare_actor_condition(
                        occurrence.actor,
                        None,
                        ActorConditionTransition::Apply {
                            definition: &definition,
                            source: ConditionSource {
                                actor: occurrence.actor,
                                session: None,
                                kind: ConditionSourceKind::SelfUse,
                            },
                            immunities: &[],
                            facts: ApplicationFacts {
                                now,
                                base_speed: 0,
                                mana_shield_capacity: 0,
                                target_reentry_protected: false,
                                source_reentry_protected: false,
                                target_is_player: false,
                                decision_root: &root,
                                occurrence: DecisionOccurrenceId::from_bytes(id),
                            },
                        },
                        condition_at,
                    )
                    .map_err(|_| ())?;
                if self.work.len() >= 128 {
                    return Err(());
                }
                self.work.try_reserve(1).map_err(|_| ())?;
                let key = self.next.checked_add(1).ok_or(())?;
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                let mut timers = self.timers.as_ref().ok_or(())?.clone();
                timers
                    .schedule(
                        runtime.owner_fence().map_err(|_| ())?,
                        stamp,
                        Family::Heal,
                        key,
                        Some(occurrence.actor),
                        SemanticTimeMicros::from_micros(now.checked_add(7_000_000).ok_or(())?),
                    )
                    .map_err(|_| ())?;
                let speech =
                    CallbackSpeech::gaz_heal(runtime, occurrence.actor, false).map_err(|_| ())?;
                runtime
                    .commit_actor_condition(&condition, condition_at)
                    .map_err(|_| ())?;
                self.next = key;
                self.work.push(Work {
                    key,
                    actor: occurrence.actor,
                });
                self.timers = Some(timers);
                spawns::emit_speech(room, vec![speech]);
            }
            self.seen.retain(|old| {
                runtime.contains_live_creature(old.actor) && old.actor != occurrence.actor
            });
            if self.seen.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                return Err(());
            }
            self.seen.push(*occurrence);
        }
        let mut preview = self.timers.as_ref().ok_or(())?.clone();
        for due in preview.drain_due(&Now(now), runtime.owner_fence().map_err(|_| ())?, |_| true) {
            let Some(index) = self.work.iter().position(|work| work.key == due.occurrence) else {
                continue;
            };
            let actor = self.work[index].actor;
            if runtime.contains_live_creature(actor) {
                let snapshot = runtime.companion_snapshot(actor).map_err(|_| ())?;
                let speech = CallbackSpeech::gaz_heal(runtime, actor, true).map_err(|_| ())?;
                let at = visual_position(
                    runtime
                        .read_actor_position(actor)
                        .map_err(|_| ())?
                        .position(),
                )
                .ok_or(())?;
                use crate::map::boot::BundleCollisionIndexExt;
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                    room.movement_cells().index()
                else {
                    return Err(());
                };
                runtime
                    .commit_source_creature_heal_batch(
                        native.source_digest(),
                        &[(
                            actor,
                            snapshot.state.policy.definition_key.clone(),
                            snapshot.maximum_health as u64,
                            300000,
                        )],
                    )
                    .map_err(|_| ())?;
                index.map().facts.emit_movement_effect(at, 13);
                spawns::emit_speech(room, vec![speech]);
            }
            self.timers
                .as_mut()
                .ok_or(())?
                .cancel(Family::Heal, due.occurrence);
            self.work.remove(index);
        }
        Ok(())
    }
}
