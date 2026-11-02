//! Bounded AI-3 -> AI-4 integration for source-qualified adjacent physical melee.
//! Ranged attacks, defences, areas and custom callbacks are explicitly unsupported.
//! Uses the existing AI issuer, commit fences and player vitals/death owner.
//! Native source indices consume the committed shared attack-window decision. Legacy adapter
//! intervals remain coalesced. Every occurrence is bound to the creature and source entry.
use std::collections::{BTreeMap, btree_map};

use crate::ability::creature_bite::{
    AppliedBite, BiteRejection, CREATURE_BITE_LEDGER_MAX, CreatureBiteDefinition,
    CreatureBiteLedger, CreatureBiteVitals, ReentryProtection, commit_ai_bite, meet_zero_bite,
};
use crate::ability::{AiAbilityAdapter, RevisionSet};
use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::{
    AttackReadiness, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId, ThinkOccurrence,
    ThinkOutcome, chebyshev_distance, step_towards,
};
use crate::creature_auto_attack::MeleeSource;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_simulation_determinism::deterministic_decision_u64;

pub use oteryn_gameplay_domain::domain::melee_definition::MeleeDefinition;

/// Legacy adapter cadence. Native Crystal's 1500 ms extra-swing guard belongs exclusively
/// to ProfileScheduleState (monster.cpp:1270-1274), not a cooldown after every regular cast.
pub const MONSTER_MELEE_THRESHOLD_MICROS: u64 = 1_500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    Decision(ThinkOutcome),
    Bite(Result<AppliedBite, BiteRejection>),
    ZeroDamage,
}
#[derive(Debug, Clone)]
struct Entry {
    definition: MeleeDefinition,
    due_us: u64,
    last: Option<(u64, Dispatch)>,
}
#[derive(Debug)]
struct ActorEntries {
    actor: ExactActorRef,
    legacy: Option<Entry>,
    native: BTreeMap<u16, Entry>,
}
#[derive(Debug, Default)]
pub struct MonsterMeleeOwner {
    /// Keyed by the issuer's slot (`actor_local_id`), at most one entry per carrier slot and so
    /// at most `CREATUREAI0-RL-04` (SPAWN-CEILINGS-1, Codex 4242381779: no longer the D57 64). A
    /// newer generation in the slot replaces the dead issuer's entry in place.
    entries: BTreeMap<u32, ActorEntries>,
    bites: CreatureBiteLedger,
    /// CREATURE-AI-1 §2.1: the Channel's creature thinks, which decide each swing's target.
    pub creature_ai: crate::ai_think::CreatureAiTable,
    /// The first owner time of the next pass that swings (SPAWN-CEILINGS-1, Codex 4243189401):
    /// swings keep one pass per `CREATURE_THINK_INTERVAL_MILLIS`, while the passes between them
    /// only run thinks left due by `CREATUREAI0-RL-05`.
    pub next_swing_pass_us: u64,
    /// The players of the last swing pass's census, by actor and session (SPAWN-CEILINGS-1,
    /// Codex 4243397472). A pass between swing passes reads only these players' current
    /// positions instead of a census of every actor; at most one entry per present player.
    pub think_players: Vec<(ExactActorRef, GameSessionId)>,
}
impl MonsterMeleeOwner {
    /// One scheduled owner think, at most one existing typed bite. The selected
    /// player id resolves to current actor/session facts supplied by the owner;
    /// the bite independently revalidates both generations and protection.
    #[allow(clippy::too_many_arguments)]
    pub fn think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn CreatureBiteVitals,
        issuer: ExactActorRef,
        sequence: u64,
        definition: MeleeDefinition,
        input: CreatureThinkInput,
        perceived: &[PerceivedPlayer],
        resolve_target: impl Fn(
            PerceivedPlayerId,
        ) -> Option<(ExactActorRef, GameSessionId, ReentryProtection)>,
        revisions: RevisionSet,
        now: SemanticTimeMicros,
    ) -> Result<Dispatch, BiteRejection> {
        self.think_with_native_source(
            runtime,
            vitals,
            issuer,
            sequence,
            definition,
            input,
            perceived,
            resolve_target,
            revisions,
            now,
            None,
        )
    }

    /// A privately resolved native source consumes exactly the committed AI occurrence and
    /// source index. The legacy call supplies None and keeps its original receipt/cadence.
    #[allow(clippy::too_many_arguments)]
    pub fn think_with_native_source(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
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
        native_source: Option<&MeleeSource>,
    ) -> Result<Dispatch, BiteRejection> {
        runtime
            .owner_fence()
            .map_err(|_| BiteRejection::StaleIssuer)?;
        if !runtime.contains_live_creature(issuer) {
            return Err(BiteRejection::StaleIssuer);
        }
        let selection = if let Some(source) = native_source {
            if !source.admits_bite_definition(runtime, issuer, definition) {
                return Err(BiteRejection::InvalidPlan);
            }
            Some(
                self.creature_ai
                    .native_melee_selection(issuer, usize::from(definition.entry_index))
                    .ok_or(BiteRejection::InvalidPlan)?,
            )
        } else {
            None
        };
        let sequence = selection.map_or(sequence, |selected| selected.occurrence.sequence);
        if selection.is_some() && sequence == 0 {
            return Ok(Dispatch::Decision(ThinkOutcome::Idle));
        }
        if definition.content_digest != runtime.content_pin().server_artifact_digest()
            || sequence == 0
        {
            return Err(BiteRejection::InvalidPlan);
        }
        let fresh = Entry {
            definition,
            due_us: now.get(),
            last: None,
        };
        let entries = self.entries.len();
        // The issuer is live, so another generation's entry in its slot is a dead issuer's.
        let actor_entries = match self.entries.entry(issuer.actor_local_id()) {
            btree_map::Entry::Occupied(occupied) => {
                let entry = occupied.into_mut();
                if entry.actor != issuer {
                    *entry = ActorEntries {
                        actor: issuer,
                        legacy: None,
                        native: BTreeMap::new(),
                    };
                }
                entry
            }
            btree_map::Entry::Vacant(vacant) => {
                if entries >= CREATURE_BITE_LEDGER_MAX {
                    return Err(BiteRejection::LedgerFull);
                }
                vacant.insert(ActorEntries {
                    actor: issuer,
                    legacy: None,
                    native: BTreeMap::new(),
                })
            }
        };
        let entry = if native_source.is_some() {
            actor_entries
                .native
                .entry(definition.entry_index)
                .or_insert(fresh)
        } else {
            actor_entries.legacy.get_or_insert(fresh)
        };
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
        // Crystal monster.cpp:1204/1266 suppresses melee while fleeing, but leaves
        // ranged spell schedules running. Healing can resume this same pending swing.
        if selection.is_none()
            && self
                .creature_ai
                .state(issuer)
                .is_some_and(|state| state.is_fleeing())
        {
            let result = Dispatch::Decision(ThinkOutcome::Idle);
            entry.last = Some((sequence, result));
            return Ok(result);
        }
        let due = selection.map_or(now.get() >= entry.due_us, |selected| {
            selected.target.is_some()
        });
        input.attack = AttackReadiness {
            off_cooldown: due,
            chance_percent: u8::try_from(definition.chance_ppm / 10_000)
                .map_err(|_| BiteRejection::InvalidPlan)?,
        };
        let decision = swing_decision(
            &input,
            perceived,
            issuer,
            sequence,
            &definition,
            selection.map(|selected| selected.target.is_some()),
        )?;
        if due && native_source.is_none() {
            entry.due_us = now
                .get()
                .checked_add(definition.interval_us.max(MONSTER_MELEE_THRESHOLD_MICROS))
                .ok_or(BiteRejection::InvalidPlan)?;
        }
        let result = match decision {
            ThinkOutcome::AttackIntent(id) => {
                let Some((target, session, protection)) = resolve_target(id) else {
                    let result = Dispatch::Bite(Err(BiteRejection::StaleTarget));
                    entry.last = Some((sequence, result));
                    return Ok(result);
                };
                if selection.is_some_and(|selected| selected.target != Some(target)) {
                    let result = Dispatch::Bite(Err(BiteRejection::InvalidPlan));
                    entry.last = Some((sequence, result));
                    return Ok(result);
                }
                let draw_occurrence = selection.map_or(input.occurrence, |selected| {
                    decision_occurrence(selected.occurrence)
                });
                let draw_index = if selection.is_some() {
                    u64::from(definition.entry_index)
                } else {
                    0
                };
                let draw = deterministic_decision_u64(
                    &input.decision_root,
                    draw_occurrence,
                    "ai.monster.melee.magnitude",
                    draw_index,
                )
                .map_err(|_| BiteRejection::InvalidPlan)?;
                // ARMOR-MITIGATION-1: the same occurrence draws the target's armor reduction.
                let armor_draw = deterministic_decision_u64(
                    &input.decision_root,
                    draw_occurrence,
                    "ai.monster.melee.armor",
                    draw_index,
                )
                .map_err(|_| BiteRejection::InvalidPlan)?;
                // SHIELD-DEFENCE-1: and the target's blocked-hit defence.
                let defence_draw = deterministic_decision_u64(
                    &input.decision_root,
                    draw_occurrence,
                    "ai.monster.melee.defence",
                    draw_index,
                )
                .map_err(|_| BiteRejection::InvalidPlan)?;
                let width = u64::from(definition.maximum) - u64::from(definition.minimum) + 1;
                let magnitude = definition.minimum
                    + u32::try_from(draw % width).map_err(|_| BiteRejection::InvalidPlan)?;
                if magnitude == 0 {
                    // ZERO-BITE-BLOCK-1: a 0 bite still meets the player's block.
                    meet_zero_bite(
                        runtime,
                        vitals,
                        AiAbilityAdapter::bite(
                            issuer,
                            sequence,
                            target,
                            session,
                            armor_draw,
                            defence_draw,
                        ),
                        protection,
                        now,
                    );
                    Dispatch::ZeroDamage
                } else {
                    let intent = AiAbilityAdapter::bite(
                        issuer,
                        sequence,
                        target,
                        session,
                        armor_draw,
                        defence_draw,
                    );
                    let bite = match native_source {
                        Some(source) => CreatureBiteDefinition::from_native_source(
                            source, runtime, intent, definition, magnitude,
                        ),
                        None => CreatureBiteDefinition::new(
                            definition.interval_us.max(MONSTER_MELEE_THRESHOLD_MICROS),
                            magnitude,
                        ),
                    }
                    .ok_or(BiteRejection::InvalidPlan)?;
                    Dispatch::Bite(commit_ai_bite(
                        &mut self.bites,
                        runtime,
                        vitals,
                        intent,
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

/// CREATURE-AI-1 §2.1: of `perceived`, only the creature think's `target` may be swung at; no
/// target leaves the swing idle.
pub fn think_target_only(
    target: Option<ExactActorRef>,
    perceived: impl IntoIterator<Item = (ExactActorRef, PerceivedPlayer)>,
) -> Vec<PerceivedPlayer> {
    perceived
        .into_iter()
        .filter(|(actor, _)| Some(*actor) == target)
        .map(|(_, player)| player)
        .collect()
}

/// §1.5: the swing's target is the nearest same-floor perceived player (Chebyshev, then id);
/// the production owner passes only the creature think's target (CREATURE-AI-1 §2.1).
/// Adjacent, legal and due draws `AI_ATTACK`; not adjacent steps toward it; otherwise idle.
fn swing_decision(
    input: &CreatureThinkInput,
    perceived: &[PerceivedPlayer],
    issuer: ExactActorRef,
    sequence: u64,
    definition: &MeleeDefinition,
    native_selected: Option<bool>,
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
    if let Some(selected) = native_selected {
        return Ok(if selected {
            ThinkOutcome::AttackIntent(target.id)
        } else {
            ThinkOutcome::Idle
        });
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
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use {
        crate::foundation::{ChannelId, MovementLocalPosition, NodeId, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };

    struct Vitals {
        health: u32,
        revision: u64,
        armor: u32,
        defence: Option<(u32, u32)>,
        protection: Option<oteryn_gameplay_content::content::ReferenceRationalPercent>,
    }
    impl CreatureBiteVitals for Vitals {
        fn apply_creature_damage(
            &mut self,
            _: &mut ChannelRuntimeV1,
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
        fn target_armor(&self, _: &ChannelRuntimeV1, _: ExactActorRef, _: GameSessionId) -> u32 {
            self.armor
        }
        fn target_defence(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
            _: SemanticTimeMicros,
        ) -> Option<(u32, u32)> {
            self.defence
        }
        fn target_physical_protection(
            &self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
        ) -> Option<oteryn_gameplay_content::content::ReferenceRationalPercent> {
            self.protection
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
        fixture_with(4)
    }
    fn fixture_with(capacity: usize) -> Result<Fixture, String> {
        let world = WorldId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?,
            NodeId::decode(&uuid(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
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
        let (mut runtime, issuer, target, session) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
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
                &mut runtime,
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
                    &mut runtime,
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
                &mut runtime,
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
                &mut runtime,
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
                &mut runtime,
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
            owner
                .entries
                .values()
                .next()
                .and_then(|entry| entry.legacy.as_ref())
                .map(|entry| entry.due_us),
            Some(11_000_000),
            "missed turns never burst"
        );
        Ok(())
    }
    #[test]
    fn the_targets_worn_armor_blocks_before_the_vitals_write() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
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
        let bite = |armor, runtime: &mut ChannelRuntimeV1| {
            let mut owner = MonsterMeleeOwner::default();
            let mut vitals = Vitals {
                health: 20,
                revision: 0,
                armor,
                defence: None,
                protection: None,
            };
            let result = owner
                .think(
                    runtime,
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
            Ok::<_, String>((result, vitals.health, vitals.revision))
        };
        let (result, health, revision) = bite(2, &mut runtime)?;
        let Dispatch::Bite(Ok(applied)) = result else {
            return Err(format!("armor 2 bite: {result:?}"));
        };
        assert_eq!((applied.requested, applied.damage.applied), (8, 7));
        assert_eq!((health, revision), (13, 1), "armor 1..=3 takes one point");
        let (result, health, revision) = bite(40, &mut runtime)?;
        let Dispatch::Bite(Ok(applied)) = result else {
            return Err(format!("armor 40 bite: {result:?}"));
        };
        assert_eq!((applied.requested, applied.damage.applied), (8, 0));
        assert_eq!(
            (health, revision),
            (20, 0),
            "a fully blocked hit writes nothing"
        );
        Ok(())
    }
    #[test]
    fn the_targets_worn_physical_protection_scales_the_hit_after_armor() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
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
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
            armor: 2,
            defence: None,
            protection: Some(oteryn_gameplay_content::content::ReferenceRationalPercent {
                numerator: 50,
                denominator: 1,
            }),
        };
        let result = owner
            .think(
                &mut runtime,
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
        let Dispatch::Bite(Ok(applied)) = result else {
            return Err(format!("protected bite: {result:?}"));
        };
        assert_eq!(
            (applied.requested, applied.damage.applied),
            (8, 4),
            "armor takes 8 to 7, then 50% truncates 3.5 off"
        );
        assert_eq!((vitals.health, vitals.revision), (16, 1));
        Ok(())
    }
    #[test]
    fn a_blocked_hit_meets_the_targets_defence_before_its_armor() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
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
        let bite = |armor, defence, runtime: &mut ChannelRuntimeV1| {
            let mut owner = MonsterMeleeOwner::default();
            let mut vitals = Vitals {
                health: 20,
                revision: 0,
                armor,
                defence,
                protection: None,
            };
            let result = owner
                .think(
                    runtime,
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
            Ok::<_, String>((result, vitals.health))
        };
        let (result, health) = bite(2, Some((3, 3)), &mut runtime)?;
        let Dispatch::Bite(Ok(applied)) = result else {
            return Err(format!("defended bite: {result:?}"));
        };
        assert_eq!((applied.requested, applied.damage.applied), (8, 4));
        assert_eq!(health, 16, "defence takes 3, then armor 2 one point");
        let (result, health) = bite(2, Some((8, 8)), &mut runtime)?;
        let Dispatch::Bite(Ok(applied)) = result else {
            return Err(format!("fully defended bite: {result:?}"));
        };
        assert_eq!((applied.damage.applied, health), (0, 20));
        Ok(())
    }
    #[test]
    fn failed_chance_and_protected_targets_never_damage_or_retry_later() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
        };
        let def = definition(&runtime, 0)?;
        let result = owner
            .think(
                &mut runtime,
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
        assert_eq!(
            owner
                .entries
                .values()
                .next()
                .and_then(|entry| entry.legacy.as_ref())
                .map(|entry| entry.due_us),
            Some(2_000_000)
        );
        assert_eq!((vitals.health, vitals.revision), (20, 0));
        let mut protected = MonsterMeleeOwner::default();
        let def = definition(&runtime, 1_000_000)?;
        let result = protected
            .think(
                &mut runtime,
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
                    &mut runtime,
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
        let (mut runtime, issuer, target, _) = fixture()?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
        };
        let mut def = definition(&runtime, 1_000_000)?;
        def.content_digest[0] ^= 1;
        assert_eq!(
            owner.think(
                &mut runtime,
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
        let def = definition(&runtime, 1_000_000)?;
        assert_eq!(
            owner.think(
                &mut runtime,
                &mut vitals,
                target,
                1,
                def,
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
    fn legacy_melee_adapter_retains_its_1500_ms_cadence() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
        let def = MeleeDefinition::new(
            runtime.content_pin().server_artifact_digest(),
            500,
            1_000_000,
            8,
            8,
        )
        .ok_or("threshold fixture")?;
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 100,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
        };
        for (sequence, now, expected_health) in [
            (1, 0, 92),
            (2, 500_000, 92),
            (3, 1_499_999, 92),
            (4, 1_500_000, 84),
            (5, 2_999_999, 84),
            (6, 3_000_000, 76),
        ] {
            let result = owner
                .think(
                    &mut runtime,
                    &mut vitals,
                    issuer,
                    sequence,
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
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(vitals.health, expected_health, "threshold at {now}");
            let replay = owner
                .think(
                    &mut runtime,
                    &mut vitals,
                    issuer,
                    sequence,
                    def,
                    input(2),
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
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(replay, result);
            assert_eq!(vitals.health, expected_health, "replay cannot double hit");
        }
        Ok(())
    }

    #[test]
    fn fleeing_melee_replays_suppression_and_resumes_after_healing() -> Result<(), String> {
        use crate::ai_think::behaviour_profile::CreatureBehaviourProfile;
        use crate::ai_think::targeting::TargetCandidate;
        use crate::ai_think::{CreatureAiState, CreatureThinkFacts};
        let (mut runtime, issuer, target, session) = fixture()?;
        let behavior = serde_json::from_value(serde_json::json!({
            "movement":{"can_walk":true,"pass_through":false,"pushable":false,
                "push_items":false,"push_creatures":false,"walks_on_energy":false,
                "walks_on_fire":false,"walks_on_poison":false},
            "targeting":{"hostile":true,"can_target":true,"sense_invisible":false,
                "target_distance_tiles":1,"static_attack_chance_ppm":1000000,"flee_health":5}
        }))
        .map_err(|error| error.to_string())?;
        let profile = CreatureBehaviourProfile::project(Some(&behavior), &BTreeMap::new())
            .map_err(|error| format!("{error:?}"))?;
        let mut owner = MonsterMeleeOwner::default();
        owner
            .creature_ai
            .admit(CreatureAiState::new(issuer, profile, position(0)), 0);
        let candidates = [TargetCandidate {
            actor: target,
            position: position(1),
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        }];
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let revisions = revisions()?;
        let mut facts = CreatureThinkFacts {
            position: position(0),
            health: 5,
            has_active_condition: false,
            players: &candidates,
            now_us: 0,
            root: &root,
            revisions: &revisions,
        };
        let report = owner
            .creature_ai
            .think(issuer, &facts)
            .ok_or("admitted AI state")?
            .map_err(|error| format!("{error:?}"))?;
        assert!(report.fleeing);
        let mut vitals = Vitals {
            health: 20,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
        };
        let definition = definition(&runtime, 1_000_000)?;
        let mut swing = |owner: &mut MonsterMeleeOwner, sequence, now| {
            owner
                .think(
                    &mut runtime,
                    &mut vitals,
                    issuer,
                    sequence,
                    definition,
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
                    revisions.clone(),
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|error| format!("{error:?}"))
        };
        assert_eq!(
            swing(&mut owner, 1, 0)?,
            Dispatch::Decision(ThinkOutcome::Idle)
        );
        facts.health = 6;
        facts.now_us = 1_000_000;
        let report = owner
            .creature_ai
            .think(issuer, &facts)
            .ok_or("admitted AI state")?
            .map_err(|error| format!("{error:?}"))?;
        assert!(!report.fleeing);
        assert_eq!(
            swing(&mut owner, 1, 1_000_000)?,
            Dispatch::Decision(ThinkOutcome::Idle)
        );
        assert!(matches!(
            swing(&mut owner, 2, 1_000_000)?,
            Dispatch::Bite(Ok(_))
        ));
        assert_eq!((vitals.health, vitals.revision), (12, 1));
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
    /// SPAWN-CEILINGS-1 (Codex 4242381779): past the D57 64 and the old `AI01-ACTIVE-ACTORS`
    /// 256, every live issuer keeps a melee entry and a bite entry and none is refused
    /// `LedgerFull`; a newer generation in a dead issuer's slot starts fresh.
    #[test]
    fn three_hundred_issuers_each_keep_their_entries_and_a_reused_slot_starts_fresh()
    -> Result<(), String> {
        const ISSUERS: usize = 300;
        // One player and 300 creatures fill every slot.
        let (mut runtime, first, target, session) = fixture_with(ISSUERS + 1)?;
        let mut issuers = vec![first];
        for index in 1..ISSUERS {
            let x = i32::try_from(index).map_err(|error| format!("{error:?}"))? * 3 + 10;
            issuers.push(
                runtime
                    .admit_test_creature(position(x))
                    .map_err(|error| format!("{error:?}"))?,
            );
        }
        let mut owner = MonsterMeleeOwner::default();
        let mut vitals = Vitals {
            health: 1_000,
            revision: 0,
            armor: 0,
            defence: None,
            protection: None,
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
        let mut swing = |runtime: &mut ChannelRuntimeV1, owner: &mut MonsterMeleeOwner, issuer| {
            owner.think(
                runtime,
                &mut vitals,
                issuer,
                1,
                def,
                input(1),
                &perceived(),
                resolve,
                revisions().map_err(|_| BiteRejection::InvalidPlan)?,
                SemanticTimeMicros::from_micros(0),
            )
        };
        for issuer in &issuers {
            // Only the first issuer stands next to the player; every other bite is out of range,
            // but each issuer's entries are made and kept.
            let dispatch =
                swing(&mut runtime, &mut owner, *issuer).map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                dispatch,
                Dispatch::Bite(Ok(_) | Err(BiteRejection::OutOfRange))
            ));
        }
        assert_eq!((owner.entries.len(), owner.bites.len()), (ISSUERS, ISSUERS));
        // The first issuer's bite landed. Its slot is then the only free one, so the next
        // creature takes it.
        let dead = first;
        runtime
            .remove_test_actor(dead)
            .map_err(|error| format!("{error:?}"))?;
        let newer = runtime
            .admit_test_creature(position(-5))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(newer.actor_local_id(), dead.actor_local_id());
        assert_ne!(newer, dead);
        // The same think sequence is the newer issuer's own, not a replay of the dead one's
        // landed bite.
        assert_eq!(
            swing(&mut runtime, &mut owner, newer),
            Ok(Dispatch::Bite(Err(BiteRejection::OutOfRange)))
        );
        // The melee entry is replaced in its slot; the bite ledger keys the exact generation and
        // keeps the dead one's entry until it is retired or the ledger is full.
        assert_eq!(
            (owner.entries.len(), owner.bites.len()),
            (ISSUERS, ISSUERS + 1)
        );
        assert_eq!(
            owner
                .entries
                .get(&newer.actor_local_id())
                .map(|entry| entry.actor),
            Some(newer)
        );
        Ok(())
    }
    struct CountedDefence {
        defences: u32,
        hits: u32,
    }
    impl CreatureBiteVitals for CountedDefence {
        fn apply_creature_damage(
            &mut self,
            _: &mut ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
            _: u32,
            _: SemanticTimeMicros,
        ) -> Option<CreatureHit> {
            self.hits += 1;
            None
        }
        fn target_armor(&self, _: &ChannelRuntimeV1, _: ExactActorRef, _: GameSessionId) -> u32 {
            0
        }
        fn target_defence(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
            _: SemanticTimeMicros,
        ) -> Option<(u32, u32)> {
            self.defences += 1;
            None
        }
    }
    #[test]
    fn a_zero_bite_meets_the_players_block_once_unless_protected() -> Result<(), String> {
        let (mut runtime, issuer, target, session) = fixture()?;
        let def = MeleeDefinition::new(
            runtime.content_pin().server_artifact_digest(),
            2000,
            1_000_000,
            0,
            1,
        )
        .ok_or("valid 0..1 melee definition was refused")?;
        let bite = |seed, protected_until, runtime: &mut ChannelRuntimeV1| {
            let mut owner = MonsterMeleeOwner::default();
            let mut vitals = CountedDefence {
                defences: 0,
                hits: 0,
            };
            let resolve = |_| Some((target, session, ReentryProtection { protected_until }));
            let mut results = Vec::new();
            for _ in 0..2 {
                results.push(
                    owner
                        .think(
                            runtime,
                            &mut vitals,
                            issuer,
                            1,
                            def,
                            input(seed),
                            &perceived(),
                            resolve,
                            revisions()?,
                            SemanticTimeMicros::from_micros(0),
                        )
                        .map_err(|error| format!("{error:?}"))?,
                );
            }
            Ok::<_, String>((results, vitals.defences, vitals.hits))
        };
        let mut zero_seed = None;
        for seed in 1..=32 {
            let (results, defences, hits) = bite(seed, None, &mut runtime)?;
            if results[0] == Dispatch::ZeroDamage {
                assert_eq!(
                    results[1],
                    Dispatch::ZeroDamage,
                    "the retry replays the swing"
                );
                assert_eq!(
                    (defences, hits),
                    (1, 0),
                    "a 0 bite spends one block and writes no damage"
                );
                zero_seed = Some(seed);
                break;
            }
        }
        let seed = zero_seed.ok_or("no seed in 1..=32 drew a 0 bite")?;
        let (results, defences, _) = bite(
            seed,
            Some(SemanticTimeMicros::from_micros(1000)),
            &mut runtime,
        )?;
        assert_eq!(results[0], Dispatch::ZeroDamage);
        assert_eq!(defences, 0, "a protected player's block is not met");
        Ok(())
    }

    mod native_tests;
}
