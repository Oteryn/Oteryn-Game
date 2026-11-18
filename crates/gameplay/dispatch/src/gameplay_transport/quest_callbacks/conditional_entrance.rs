//! Observe durable owners before taking the Channel lock. Check position and
//! the live Channel predicates in the same turn as the native callback.
use super::*;
use crate::durability::quest_world_callback::conditional_entrance::{Facts, ReadSet};
use oteryn_gameplay_quests::quest::world_callbacks::conditional_entrance::{
    EffectAt, Entrance, MessageKind, Predicate, PushBack, Refusal, StorageRange,
};
use oteryn_simulation_determinism::SemanticTimeMicros;

pub(super) enum Observation<'a> {
    None,
    Current {
        gate: &'a Entrance,
        expected: MovementPositionSnapshot,
        fence: CurrentCharacterGameplayFence,
        facts: Facts,
        premium: bool,
    },
    Unavailable,
}

pub(super) struct Unavailable;

pub(super) struct Failure<'a> {
    refusal: Option<&'a Refusal>,
    missing: Vec<&'a str>,
    unavailable: bool,
}

impl From<Unavailable> for Failure<'_> {
    fn from(_: Unavailable) -> Self {
        Self {
            refusal: None,
            missing: Vec::new(),
            unavailable: true,
        }
    }
}

struct Live<'a> {
    runtime: &'a ChannelRuntimeV1,
    level: u32,
    premium: bool,
    pvp_locked: bool,
}

impl Live<'_> {
    fn admits<'a>(
        &self,
        facts: &Facts,
        predicate: &'a Predicate,
        missing: &mut Vec<&'a str>,
    ) -> bool {
        let contains = |range: &StorageRange| {
            facts
                .storage(&range.source)
                .is_some_and(|value| range.contains(value))
        };
        match predicate {
            Predicate::All { predicates } => {
                predicates.iter().all(|p| self.admits(facts, p, missing))
            }
            Predicate::Any { predicates } => predicates
                .iter()
                .any(|p| self.admits(facts, p, &mut Vec::new())),
            Predicate::Storage { range } => contains(range),
            Predicate::DistinctFlags { minimum, flags } => {
                let incomplete = flags
                    .iter()
                    .filter(|flag| !contains(&flag.range))
                    .collect::<Vec<_>>();
                if flags.len() - incomplete.len() < *minimum {
                    missing.extend(incomplete.into_iter().map(|flag| flag.label.as_str()));
                    false
                } else {
                    true
                }
            }
            Predicate::Carried { item, count } => {
                facts.carried.get(item).copied().unwrap_or(0) >= i64::from(*count)
            }
            Predicate::Level { minimum } => self.level >= *minimum,
            Predicate::Premium => self.premium,
            Predicate::Unoccupied { minimum, maximum } => {
                self.runtime.positioned_melee_census().is_ok_and(|actors| {
                    actors.iter().all(|(_, position, session)| {
                        session.is_none()
                            || native(position.position()).is_some_and(|at| {
                                at.floor != minimum.floor
                                    || !(minimum.x..=maximum.x).contains(&at.x)
                                    || !(minimum.y..=maximum.y).contains(&at.y)
                            })
                    })
                })
            }
            Predicate::Calendar { start, end } => {
                facts.date.is_some_and(|date| date.in_window(*start, *end))
            }
            Predicate::NoPvpLock => !self.pvp_locked,
        }
    }
}

impl<'a> Observation<'a> {
    pub(super) fn check(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        current: MovementPositionSnapshot,
    ) -> Result<(), Failure<'a>> {
        let (gate, facts, premium) = match self {
            Self::None => return Ok(()),
            Self::Current {
                gate,
                expected,
                fence,
                facts,
                premium,
            } if *expected == current
                && runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    }) =>
            {
                (*gate, facts, *premium)
            }
            _ => return Err(Unavailable.into()),
        };
        let state = states
            .get(runtime, actor, session)
            .ok_or_else(|| Failure::from(Unavailable))?;
        let live = Live {
            runtime,
            level: state.character_facts().level,
            premium,
            pvp_locked: super::super::pvp_live::pz_blocked(states, actor, session, now),
        };
        for guard in &gate.guards {
            let mut missing = Vec::new();
            if !live.admits(facts, &guard.when, &mut missing) {
                return Err(Failure {
                    refusal: guard.refusal.as_ref(),
                    missing,
                    unavailable: false,
                });
            }
        }
        Ok(())
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "observe the live Character fence and exact native entrance"
    )]
    pub(super) async fn observe_conditional_entrance<'a>(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &'a Callback,
        fence: Option<CurrentCharacterGameplayFence>,
        step: Option<MovementPositionSnapshot>,
    ) -> Observation<'a> {
        let Some(gate) = callback.effects.conditional_entrance.as_ref() else {
            return Observation::None;
        };
        let Some(fence) = fence else {
            return Observation::Unavailable;
        };
        let Some((expected, context)) = self.conditional_entrance_context(actor, map, step).await
        else {
            return Observation::Unavailable;
        };
        let mut premium_required = false;
        let mut pvp_required = false;
        for guard in &gate.guards {
            guard.when.visit(&mut |predicate| {
                premium_required |= matches!(predicate, Predicate::Premium);
                pvp_required |= matches!(predicate, Predicate::NoPvpLock);
            });
        }
        if pvp_required && step.is_none() && !self.refresh_pvp_locks(actor, session).await {
            return Observation::Unavailable;
        }
        let premium =
            premium_required && self.session_premium_gate(session).await.premium_current();
        let reads = ReadSet::for_entrance(gate);
        let facts = if reads.is_empty() {
            Facts::default()
        } else {
            let Ok(facts) = self
                .root
                .read_conditional_entrance(
                    self.character,
                    self.holder,
                    fence,
                    self.quest_catalogue.clone(),
                    permit,
                    reads,
                    context,
                )
                .await
            else {
                return Observation::Unavailable;
            };
            facts
        };
        Observation::Current {
            gate,
            expected,
            fence,
            facts,
            premium,
        }
    }

    async fn conditional_entrance_context(
        &self,
        actor: ExactActorRef,
        map: &BundleMap,
        step: Option<MovementPositionSnapshot>,
    ) -> Option<(MovementPositionSnapshot, QuestWorldContext)> {
        let runtime = self.runtime.lock().await;
        let expected = runtime.read_actor_position(actor).ok()?;
        if step.is_some_and(|entered| entered != expected) {
            return None;
        }
        Some((
            expected,
            QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
                party: Vec::new(),
            },
        ))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "bind the opened map entry and motion to one exact Channel CAS"
    )]
    pub(super) fn commit_conditional_door(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: MovementPositionSnapshot,
        key: u64,
        revision: u64,
        after: crate::map::view::EntryFacts,
        blocked: bool,
        optional_pvp: Option<bool>,
    ) -> UseDisposition {
        let Some((at, _)) = crate::map::view::placement_of(key) else {
            return UseDisposition::Rejected;
        };
        let destination = local(Position {
            x: at.x,
            y: at.y,
            floor: at.floor,
        });
        let project = || {
            map.facts.apply_quest_callback(
                key,
                revision,
                after,
                &[],
                &[(key, after, true)],
                false,
                None,
            )
        };
        let movement = if map.facts.doors().contains(key) {
            let Ok(proof) = crate::map::floor::prepare_quest_door_opening(
                runtime,
                self.movement_cells(),
                map,
                actor,
                session,
                expected,
                key,
                blocked,
            ) else {
                return UseDisposition::NotPossible;
            };
            runtime.commit_source_step_and_project(proof, project)
        } else {
            let closed = crate::map::floor::BundleDoors(&map.facts);
            let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
                runtime,
                self.movement_cells(),
                actor,
                session,
                expected,
                destination,
                blocked,
                &closed,
                None,
                true,
            ) else {
                return UseDisposition::NotPossible;
            };
            runtime.commit_source_step_and_project(proof, project)
        };
        let Ok(snapshot) = movement else {
            return UseDisposition::Rejected;
        };
        let now = self.owner_now().get();
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            now,
            Some(expected.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            now,
            underwater(map, expected.position()),
            underwater(map, snapshot.position()),
        );
        UseDisposition::Committed
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "refusal preserves the exact actor, source, and entering snapshots"
    )]
    pub(super) fn refuse_conditional_entrance(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        expected: MovementPositionSnapshot,
        origin: Option<MovementPositionSnapshot>,
        source: TilePos,
        optional_pvp: Option<bool>,
        failure: Failure<'_>,
    ) {
        let refusal = failure.refusal;
        let destination = match refusal.and_then(|r| r.push_back) {
            Some(PushBack::Fixed { position }) => Some(local(position)),
            Some(PushBack::From { dx, dy }) => origin.map(|origin| {
                let at = origin.position();
                MovementLocalPosition {
                    x: at.x + i32::from(dx),
                    y: at.y + i32::from(dy),
                    ..at
                }
            }),
            // A quiet StepIn refusal still returns the player to the approach
            // tile (Crystal movements_holy_tible_tile.lua:14-16).
            None => origin.map(|origin| origin.position()),
        };
        let blocked = super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        let mut at = expected.position();
        if let Some(destination) = destination
            && let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
                runtime,
                self.movement_cells(),
                actor,
                session,
                expected,
                destination,
                blocked,
                &crate::map::floor::BundleDoors(&map.facts),
                None,
            )
            && let Ok(snapshot) = runtime.commit_source_step(proof)
        {
            at = snapshot.position();
            let now = self.owner_now().get();
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(expected.position()),
                at,
                optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                underwater(map, expected.position()),
                underwater(map, at),
            );
        }
        let actor_tile = native(at);
        if let Some(refusal) = refusal
            && let Some(effect) = refusal.effect
        {
            if matches!(refusal.effect_at, EffectAt::Source | EffectAt::Both) {
                let _ = map.facts.emit_movement_effect(source, u16::from(effect));
            }
            if let Some(at) = actor_tile
                && (refusal.effect_at == EffectAt::Actor
                    || refusal.effect_at == EffectAt::Both && at != source)
            {
                let _ = map.facts.emit_movement_effect(at, u16::from(effect));
            }
        }
        if let Some(text) = refusal
            .and_then(|r| r.message.as_deref())
            .or(failure.unavailable.then_some("Sorry, not possible."))
        {
            use oteryn_protocol_oteryn::zones_nologout::ServerMessage;
            let mut message = text.to_owned();
            for label in failure.missing {
                message.push('\n');
                message.push_str(label);
            }
            let kind = refusal.map_or(MessageKind::Status, |r| r.message_kind);
            match kind.monster_talk_type() {
                None => {
                    let notice = if refusal.is_some_and(|r| r.message_kind == MessageKind::Advance)
                    {
                        ServerMessage::Advance(message.clone())
                    } else {
                        ServerMessage::StatusText(message.clone())
                    };
                    if !self.states.server_messages.send_current(session, notice) {
                        self.states.chat.push_quest_feedback(session, &message);
                    }
                }
                Some(talk_type) => {
                    let position = refusal
                        .and_then(|r| r.speech_position)
                        .map(tile)
                        .or(actor_tile)
                        .unwrap_or(source);
                    let _ = map
                        .facts
                        .emit_movement_speech(crate::map::facts::MovementSpeech {
                            speaker: actor,
                            position,
                            talk_type,
                            text: message,
                            captured_name: None,
                        });
                }
            }
        }
    }
}
