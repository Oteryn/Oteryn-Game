//! Bounded AI-3 -> AI-4 integration for source-qualified adjacent physical melee.
//! Ranged attacks, defences, areas and custom callbacks are explicitly unsupported.
//! Uses the existing AI issuer, commit fences and D54 non-lethal player HP floor.
//! Source intervals/chances are retained; missed owner turns coalesce without bursts.
//! CREATURE-AI-1 §1.5: the swing is the profile's one melee entry; its chance is drawn in ppm
//! with purpose `AI_ATTACK` bound to (creature, swing sequence, entry index).
use crate::ability::creature_bite::{
    AppliedBite, BiteRejection, CREATURE_BITE_LEDGER_MAX, CreatureBiteDefinition,
    CreatureBiteLedger, CreatureBiteVitals, ReentryProtection, commit_ai_bite,
};
use crate::ability::{AiAbilityAdapter, RevisionSet};
use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::{
    AttackReadiness, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId, ThinkOccurrence,
    ThinkOutcome, chebyshev_distance, step_towards,
};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_simulation_determinism::deterministic_decision_u64;

/// A qualified physical melee entry; the active artifact supplies its digest.
/// This value is content, never an issuer, current actor or mutation grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MeleeDefinition {
    content_digest: [u8; 32],
    interval_us: u64,
    chance_ppm: u32,
    entry_index: u16,
    minimum: u32,
    maximum: u32,
}
impl MeleeDefinition {
    pub(crate) fn new(
        content_digest: [u8; 32],
        interval_ms: u64,
        chance_ppm: u32,
        minimum: u32,
        maximum: u32,
    ) -> Option<Self> {
        if interval_ms == 0 || chance_ppm > 1_000_000 || maximum == 0 || minimum > maximum {
            return None;
        }
        Some(Self {
            content_digest,
            interval_us: interval_ms.checked_mul(1_000)?,
            chance_ppm,
            entry_index: 0,
            minimum,
            maximum,
        })
    }

    /// The melee entry's index in the profile's `attacks[]`: the `AI_ATTACK` draw index.
    #[must_use]
    pub(crate) const fn with_entry_index(mut self, entry_index: u16) -> Self {
        self.entry_index = entry_index;
        self
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch {
    Decision(ThinkOutcome),
    Bite(Result<AppliedBite, BiteRejection>),
    ZeroDamage,
}
#[derive(Debug, Clone)]
struct Entry {
    actor: ExactActorRef,
    definition: MeleeDefinition,
    due_us: u64,
    last: Option<(u64, Dispatch)>,
}
#[derive(Debug, Default)]
pub(crate) struct MonsterMeleeOwner {
    entries: Vec<Entry>,
    bites: CreatureBiteLedger,
}
impl MonsterMeleeOwner {
    /// One scheduled owner think, at most one existing typed bite. The selected
    /// player id resolves to current actor/session facts supplied by the owner;
    /// the bite independently revalidates both generations and protection.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn think(
        &mut self,
        runtime: &ChannelRuntimeV1,
        vitals: &mut dyn CreatureBiteVitals,
        issuer: ExactActorRef,
        sequence: u64,
        definition: MeleeDefinition,
        mut input: CreatureThinkInput,
        perceived: &[PerceivedPlayer],
        resolve_target: impl Fn(
            PerceivedPlayerId,
        ) -> Option<(ExactActorRef, GameSessionId, ReentryProtection)>,
        revisions: RevisionSet,
        now: SemanticTimeMicros,
    ) -> Result<Dispatch, BiteRejection> {
        runtime
            .owner_fence()
            .map_err(|_| BiteRejection::StaleIssuer)?;
        if !runtime.contains_live_creature(issuer) {
            return Err(BiteRejection::StaleIssuer);
        }
        if definition.content_digest != runtime.content_pin().server_artifact_digest()
            || sequence == 0
        {
            return Err(BiteRejection::InvalidPlan);
        }
        self.entries
            .retain(|e| runtime.contains_live_creature(e.actor));
        let index = match self.entries.iter().position(|e| e.actor == issuer) {
            Some(index) => index,
            None => {
                if self.entries.len() >= CREATURE_BITE_LEDGER_MAX {
                    return Err(BiteRejection::LedgerFull);
                }
                self.entries
                    .try_reserve(1)
                    .map_err(|_| BiteRejection::LedgerFull)?;
                self.entries.push(Entry {
                    actor: issuer,
                    definition,
                    due_us: now.get(),
                    last: None,
                });
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[index];
        if entry.definition != definition {
            return Err(BiteRejection::OccurrenceConflict);
        }
        if let Some((previous, result)) = entry.last {
            if sequence < previous {
                return Err(BiteRejection::OccurrenceSuperseded);
            }
            if sequence == previous {
                return Ok(result);
            }
        }
        let due = now.get() >= entry.due_us;
        input.attack = AttackReadiness {
            off_cooldown: due,
            chance_percent: u8::try_from(definition.chance_ppm / 10_000)
                .map_err(|_| BiteRejection::InvalidPlan)?,
        };
        let decision = swing_decision(&input, perceived, issuer, sequence, &definition)?;
        if due {
            entry.due_us = now
                .get()
                .checked_add(definition.interval_us)
                .ok_or(BiteRejection::InvalidPlan)?;
        }
        let result = match decision {
            ThinkOutcome::AttackIntent(id) => {
                let Some((target, session, protection)) = resolve_target(id) else {
                    let result = Dispatch::Bite(Err(BiteRejection::StaleTarget));
                    entry.last = Some((sequence, result));
                    return Ok(result);
                };
                let draw = deterministic_decision_u64(
                    &input.decision_root,
                    input.occurrence,
                    "ai.monster.melee.magnitude",
                    0,
                )
                .map_err(|_| BiteRejection::InvalidPlan)?;
                let width = u64::from(definition.maximum) - u64::from(definition.minimum) + 1;
                let magnitude = definition.minimum
                    + u32::try_from(draw % width).map_err(|_| BiteRejection::InvalidPlan)?;
                if magnitude == 0 {
                    Dispatch::ZeroDamage
                } else {
                    let bite = CreatureBiteDefinition::new(definition.interval_us, magnitude)
                        .ok_or(BiteRejection::InvalidPlan)?;
                    Dispatch::Bite(commit_ai_bite(
                        &mut self.bites,
                        runtime,
                        vitals,
                        AiAbilityAdapter::bite(issuer, sequence, target, session),
                        bite,
                        revisions,
                        protection,
                        now,
                    ))
                }
            }
            other => Dispatch::Decision(other),
        };
        entry.last = Some((sequence, result));
        Ok(result)
    }
}

/// §1.5: the swing's target is the nearest same-floor perceived player (Chebyshev, then id).
/// Adjacent, legal and due draws `AI_ATTACK`; not adjacent steps toward it; otherwise idle.
fn swing_decision(
    input: &CreatureThinkInput,
    perceived: &[PerceivedPlayer],
    issuer: ExactActorRef,
    sequence: u64,
    definition: &MeleeDefinition,
) -> Result<ThinkOutcome, BiteRejection> {
    let Some((distance, target)) = perceived
        .iter()
        .filter_map(|player| {
            chebyshev_distance(input.position, player.position).map(|distance| (distance, player))
        })
        .min_by_key(|(distance, player)| (*distance, player.id))
    else {
        return Ok(ThinkOutcome::Idle);
    };
    if distance > 1 {
        return Ok(step_towards(input.position, target.position)
            .map_or(ThinkOutcome::Idle, ThinkOutcome::ChaseStep));
    }
    if !(target.legal_attack_target && input.attack.off_cooldown) {
        return Ok(ThinkOutcome::Idle);
    }
    let draw = deterministic_decision_u64(
        &input.decision_root,
        decision_occurrence(ThinkOccurrence {
            actor: issuer,
            sequence,
        }),
        "AI_ATTACK",
        u64::from(definition.entry_index),
    )
    .map_err(|_| BiteRejection::InvalidPlan)?;
    Ok(if draw % 1_000_000 < u64::from(definition.chance_ppm) {
        ThinkOutcome::AttackIntent(target.id)
    } else {
        ThinkOutcome::Idle
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::creature_bite::{CreatureHit, creature_damage};
    use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, WorldId};
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

    struct Vitals {
        health: u32,
        revision: u64,
    }
    impl CreatureBiteVitals for Vitals {
        fn apply_creature_damage(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
            magnitude: u32,
            _: SemanticTimeMicros,
        ) -> Option<CreatureHit> {
            if self.health == 0 {
                return None;
            }
            let damage = creature_damage(self.health, magnitude);
            if damage.applied > 0 {
                self.health = damage.health_after;
                self.revision += 1;
            }
            Some(CreatureHit {
                damage,
                vitals_revision: self.revision,
                death: (damage.health_after == 0).then_some([7; 16]),
            })
        }
    }
    fn uuid(tag: u8) -> [u8; 16] {
        [1, 144, 0, 0, 0, tag, 112, 0, 128, 0, 0, 0, 0, 0, 0, tag]
    }
    type Fixture = (
        ChannelRuntimeV1,
        ExactActorRef,
        ExactActorRef,
        GameSessionId,
    );
    fn fixture() -> Result<Fixture, String> {
        let world = WorldId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?,
            NodeId::decode(&uuid(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let session = GameSessionId::decode(&uuid(4)).map_err(|error| format!("{error:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(|error| format!("{error:?}"))?;
        let target = runtime
            .commit_fresh_session(reservation)
            .map_err(|error| format!("{error:?}"))?;
        runtime
            .initialize_movement_test_position(target, position(1))
            .map_err(|error| format!("{error:?}"))?;
        let issuer = runtime
            .admit_test_creature(position(0))
            .map_err(|error| format!("{error:?}"))?;
        Ok((runtime, issuer, target, session))
    }
    fn position(x: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y: 0, floor: 7 }
    }
    fn input(seed: u8) -> CreatureThinkInput {
        CreatureThinkInput {
            provenance: crate::ai::AiProvenance::new(crate::ai::AiProvenanceInput {
                scope_id: 1,
                scope_generation: 1,
                actor_generation: 1,
                behavior_revision: 1,
                content_revision: 1,
                navigation_revision: 1,
                ruleset_revision: 1,
                determinism_profile_revision: 1,
            }),
            decision_root: GameplayDecisionRoot::from_bytes([seed; 32]),
            occurrence: DecisionOccurrenceId::from_bytes([seed; 16]),
            path_work_id: 1,
            position: position(0),
            home: position(0),
            // The owner replaces these caller values with qualified scheduling.
            attack: AttackReadiness {
                off_cooldown: false,
                chance_percent: 0,
            },
        }
    }
    fn perceived() -> [PerceivedPlayer; 1] {
        [PerceivedPlayer {
            id: PerceivedPlayerId::new(1),
            position: position(1),
            legal_attack_target: true,
        }]
    }
    fn revisions() -> Result<RevisionSet, String> {
        RevisionSet::new(
            "ruleset:ai-v1",
            "content:1",
            "world:ai-v1",
            "formula:bite-v1",
            "simulation:v1",
        )
        .map_err(|error| format!("{error:?}"))
    }
    fn definition(runtime: &ChannelRuntimeV1, chance: u32) -> Result<MeleeDefinition, String> {
        MeleeDefinition::new(
            runtime.content_pin().server_artifact_digest(),
            2000,
            chance,
            8,
            8,
        )
        .ok_or_else(|| "valid source melee definition was refused".to_owned())
    }
    #[test]
    fn typed_think_dispatch_pays_one_hit_replays_and_preserves_source_interval()
    -> Result<(), String> {
        let (runtime, issuer, target, session) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
        };
        let def = definition(&runtime, 1_000_000)?;
        let resolve = |_| {
            Some((
                target,
                session,
                ReentryProtection {
                    protected_until: None,
                },
            ))
        };
        let result = owner
            .think(
                &runtime,
                &mut vitals,
                issuer,
                1,
                def,
                input(1),
                &perceived(),
                resolve,
                revisions()?,
                SemanticTimeMicros::from_micros(0),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert!(matches!(result, Dispatch::Bite(Ok(_))));
        assert_eq!((vitals.health, vitals.revision), (12, 1));
        assert_eq!(
            owner
                .think(
                    &runtime,
                    &mut vitals,
                    issuer,
                    1,
                    def,
                    input(77),
                    &perceived(),
                    resolve,
                    revisions()?,
                    SemanticTimeMicros::from_micros(999)
                )
                .map_err(|error| format!("{error:?}"))?,
            result
        );
        assert_eq!((vitals.health, vitals.revision), (12, 1));
        let early = owner
            .think(
                &runtime,
                &mut vitals,
                issuer,
                2,
                def,
                input(2),
                &perceived(),
                resolve,
                revisions()?,
                SemanticTimeMicros::from_micros(1_999_999),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert!(!matches!(early, Dispatch::Bite(Ok(_))));
        assert_eq!(vitals.health, 12);
        let due = owner
            .think(
                &runtime,
                &mut vitals,
                issuer,
                3,
                def,
                input(3),
                &perceived(),
                resolve,
                revisions()?,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert!(matches!(due, Dispatch::Bite(Ok(_))));
        assert_eq!((vitals.health, vitals.revision), (4, 2));
        owner
            .think(
                &runtime,
                &mut vitals,
                issuer,
                4,
                def,
                input(4),
                &perceived(),
                resolve,
                revisions()?,
                SemanticTimeMicros::from_micros(9_000_000),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(vitals.health, 0, "DEATH-2: a creature bite may be lethal");
        assert_eq!(
            owner.entries[0].due_us, 11_000_000,
            "missed turns never burst"
        );
        Ok(())
    }
    #[test]
    fn failed_chance_and_protected_targets_never_damage_or_retry_later() -> Result<(), String> {
        let (runtime, issuer, target, session) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
        };
        let def = definition(&runtime, 0)?;
        let result = owner
            .think(
                &runtime,
                &mut vitals,
                issuer,
                1,
                def,
                input(1),
                &perceived(),
                |_| {
                    Some((
                        target,
                        session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                revisions()?,
                SemanticTimeMicros::from_micros(0),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(result, Dispatch::Decision(ThinkOutcome::Idle));
        assert_eq!(owner.entries[0].due_us, 2_000_000);
        assert_eq!((vitals.health, vitals.revision), (20, 0));
        let mut protected = MonsterMeleeOwner::default();
        let def = definition(&runtime, 1_000_000)?;
        let result = protected
            .think(
                &runtime,
                &mut vitals,
                issuer,
                1,
                def,
                input(1),
                &perceived(),
                |_| {
                    Some((
                        target,
                        session,
                        ReentryProtection {
                            protected_until: Some(SemanticTimeMicros::from_micros(1000)),
                        },
                    ))
                },
                revisions()?,
                SemanticTimeMicros::from_micros(0),
            )
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(result, Dispatch::Bite(Err(BiteRejection::TargetProtected)));
        assert_eq!(
            protected
                .think(
                    &runtime,
                    &mut vitals,
                    issuer,
                    1,
                    def,
                    input(2),
                    &perceived(),
                    |_| Some((
                        target,
                        session,
                        ReentryProtection {
                            protected_until: None
                        }
                    )),
                    revisions()?,
                    SemanticTimeMicros::from_micros(2000)
                )
                .map_err(|error| format!("{error:?}"))?,
            result
        );
        assert_eq!((vitals.health, vitals.revision), (20, 0));
        Ok(())
    }
    #[test]
    fn wrong_content_and_player_issuers_do_not_allocate_schedule_state() -> Result<(), String> {
        let (runtime, issuer, target, _) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
        };
        let mut def = definition(&runtime, 1_000_000)?;
        def.content_digest[0] ^= 1;
        assert_eq!(
            owner.think(
                &runtime,
                &mut vitals,
                issuer,
                1,
                def,
                input(1),
                &perceived(),
                |_| None,
                revisions()?,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(BiteRejection::InvalidPlan)
        );
        assert_eq!(
            owner.think(
                &runtime,
                &mut vitals,
                target,
                1,
                definition(&runtime, 1_000_000)?,
                input(1),
                &perceived(),
                |_| None,
                revisions()?,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(BiteRejection::StaleIssuer)
        );
        assert!(owner.entries.is_empty());
        assert_eq!((vitals.health, vitals.revision), (20, 0));
        Ok(())
    }
    #[test]
    fn source_intervals_and_chances_refuse_lossy_or_invalid_inputs() -> Result<(), String> {
        let d = MeleeDefinition::new([3; 32], 2000, 1_000_000, 0, 8)
            .ok_or("valid source interval/chance was refused")?;
        assert_eq!(d.interval_us, 2_000_000);
        assert_eq!(d.chance_ppm, 1_000_000);
        assert_eq!(d.entry_index, 0);
        assert_eq!(d.with_entry_index(2).entry_index, 2);
        assert!(MeleeDefinition::new([3; 32], 0, 1_000_000, 0, 8).is_none());
        assert!(MeleeDefinition::new([3; 32], u64::MAX, 1_000_000, 0, 8).is_none());
        assert!(MeleeDefinition::new([3; 32], 2000, 1_000_001, 0, 8).is_none());
        assert!(MeleeDefinition::new([3; 32], 2000, 123_456, 0, 8).is_some());
        assert!(MeleeDefinition::new([3; 32], 2000, 1_000_000, 9, 8).is_none());
        assert!(MeleeDefinition::new([3; 32], 2000, 0, 0, 0).is_none());
        Ok(())
    }
}
