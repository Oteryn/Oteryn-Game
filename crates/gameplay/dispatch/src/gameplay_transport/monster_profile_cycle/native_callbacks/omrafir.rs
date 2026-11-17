//! Literal Omrafir callbacks, independent of the nine canonical Encounter casts.
use super::*;
use crate::content::native_omrafir::{
    OmrafirEventBirth, OmrafirSourceCallbackKind, QualifiedOmrafirEvents,
    QualifiedOmrafirSourceAttacks,
};
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::movement::source_placement::SourcePlacementPlan;
use crate::source_callback_spawn::CallbackSpawnOwner;
use oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::{
    CallbackSpeech, OmrafirSpeech,
};
use oteryn_gameplay_entities::{MovementFacing, PreparedNativeCreatureEventBirth};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Beam,
    Boom,
    Reform,
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
struct Birth {
    proof: PreparedNativeCreatureEventBirth,
    child: Ref,
    at: Option<MovementLocalPosition>,
    placement: Option<SourcePlacementPlan>,
    done: bool,
}
#[derive(Debug)]
enum WorkKind {
    Beam {
        parent: Box<ProfileAbilityProposal>,
        source: Option<Box<crate::creature_damage_spell::composed::ComposedSource>>,
    },
    Boom {
        actor: ExactActorRef,
        occurrence: crate::foundation::ThinkOccurrence,
        births: Vec<Birth>,
        started: bool,
    },
    Reform {
        occurrence: crate::foundation::ThinkOccurrence,
        birth: Option<Box<Birth>>,
        known_failure: bool,
    },
}
#[derive(Debug)]
struct Work {
    key: u64,
    family: Family,
    kind: WorkKind,
}
#[derive(Debug)]
pub(super) struct NativeOmrafir {
    // The pinned MonsterType does not attach OmrafirExplode. Only explicit
    // review fixtures can opt into its declared engine; spells stay separate.
    review_declared_think: bool,
    overlay: Option<QualifiedOmrafirSourceAttacks>,
    events: Option<QualifiedOmrafirEvents>,
    world: Option<crate::content::QualifiedPublishedWorldContext>,
    timers: Option<OwnerTimerLane<Family, u64>>,
    work: Vec<Work>,
    next: u64,
    storage: i64,
    seen: Vec<crate::foundation::ThinkOccurrence>,
    memos: ActorMemoTable<Ref, Memo>,
    terminal_failures: Vec<ExactActorRef>,
}
impl Default for NativeOmrafir {
    fn default() -> Self {
        Self {
            review_declared_think: false,
            overlay: None,
            events: None,
            world: None,
            timers: None,
            work: Vec::new(),
            next: 1,
            storage: -1,
            seen: Vec::new(),
            memos: Default::default(),
            terminal_failures: Vec::new(),
        }
    }
}
fn say(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    kind: OmrafirSpeech,
) -> Result<(), ()> {
    spawns::emit_speech(
        room,
        vec![CallbackSpeech::omrafir(runtime, actor, kind).map_err(|_| ())?],
    );
    Ok(())
}
fn effect(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    id: u16,
) -> Result<(), ()> {
    use crate::map::boot::BundleCollisionIndexExt;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
    else {
        return Err(());
    };
    let at = visual_position(
        runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position(),
    )
    .ok_or(())?;
    index.map().facts.emit_movement_effect(at, id);
    Ok(())
}
fn marked(runtime: &ChannelRuntimeV1, actor: ExactActorRef, now: u64) -> Result<bool, ()> {
    Ok(runtime
        .actor_conditions_at(actor, None, now)
        .map_err(|_| ())?
        .get(oteryn_gameplay_domain::domain::condition::ConflictKey::SpellRegeneration(88888))
        .is_some())
}
fn uniform(
    native: &NativeGameplayState,
    occurrence: &crate::foundation::ThinkOccurrence,
    domain: &str,
    n: u64,
    min: i64,
    max: i64,
) -> Result<i64, ()> {
    let digest = Sha256::new()
        .chain_update(b"oteryn:omrafir-source-event:v1")
        .chain_update(occurrence.actor.placement_identity())
        .chain_update(occurrence.actor.actor_local_generation().to_be_bytes())
        .chain_update(occurrence.sequence.to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(native.source_digest()),
        DecisionOccurrenceId::from_bytes(id),
        domain,
        n,
    )
    .map_err(|_| ())?;
    Ok(crate::spell::uniform_draw(draw, min, max))
}
impl NativeOmrafir {
    #[cfg(test)]
    pub(super) fn enable_declared_think_review(&mut self) {
        self.review_declared_think = true;
    }
    #[cfg(test)]
    pub(super) fn terminal_failure_for_test(&self, actor: ExactActorRef) -> Option<&'static str> {
        self.terminal_failures
            .contains(&actor)
            .then_some("unknown-monster-nil-then-lua-error")
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.timers
            .as_ref()
            .and_then(|t| t.earliest_due())
            .map(|t| t.get())
    }
    fn bind(&mut self, native: &NativeGameplayState) -> Result<(), ()> {
        if self.overlay.is_none() {
            let parent = native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == "oteryn:creature.omrafir")
                .ok_or(())?
                .profile
                .target
                .clone();
            self.overlay = Some(native.omrafir_source_attacks(&parent).ok_or(())?);
            self.events = Some(native.omrafir_events_source(&parent).ok_or(())?);
        }
        Ok(())
    }
    fn lane(&mut self, runtime: &ChannelRuntimeV1) -> Result<(), ()> {
        if self.timers.is_none() {
            self.timers = Some(
                OwnerTimerLane::for_generation(
                    runtime.binding().runtime_scope(),
                    runtime.binding().scope_generation(),
                    [Family::Beam, Family::Boom, Family::Reform].map(|f| {
                        (
                            f,
                            FamilyPolicy {
                                max_pending: 128,
                                catch_up: CatchUpPolicy::DeadlineState,
                            },
                        )
                    }),
                )
                .map_err(|_| ())?,
            );
        }
        Ok(())
    }
    fn token(&mut self) -> Result<u64, ()> {
        let id = self.next;
        self.next = id.checked_add(1).ok_or(())?;
        Ok(id)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "actual native think books the two literal independently anchored Lua events"
    )]
    pub(super) async fn think(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        occurrences: &[crate::foundation::ThinkOccurrence],
        now: u64,
    ) -> Result<(), ()> {
        if !self.review_declared_think {
            return Ok(());
        }
        if self.work.is_empty()
            && !occurrences.iter().any(|o| {
                runtime.matches_live_creature_identity(o.actor, b"oteryn:creature.omrafir")
            })
        {
            return Ok(());
        }
        self.bind(native)?;
        self.lane(runtime)?;
        if !self
            .world
            .as_ref()
            .is_some_and(|q| q.matches(room, runtime.content_pin()))
        {
            self.world = active.qualify_published_world_context(room, runtime.content_pin());
        }
        for occurrence in occurrences {
            if !runtime.matches_live_creature_identity(occurrence.actor, b"oteryn:creature.omrafir")
                || self
                    .seen
                    .iter()
                    .any(|p| p.actor == occurrence.actor && p.sequence >= occurrence.sequence)
            {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(occurrence.actor)
                .map_err(|_| ())?;
            let normal = i128::from(snapshot.health) * 100
                <= i128::from(snapshot.maximum_health) * 50
                && runtime.native_summon_count(occurrence.actor, None) < 4
                && self.storage < 2;
            let terminal =
                i128::from(snapshot.health) * 100 <= i128::from(snapshot.maximum_health) * 3;
            if (normal || terminal) && !marked(runtime, occurrence.actor, now)? {
                let events = self.events.as_ref().ok_or(())?.clone();
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                let world = self.world.as_ref().ok_or(())?;
                let at = world
                    .source_frame()
                    .physical_position([33586, 32379, 12])
                    .map(|(x, y, floor)| MovementLocalPosition { x, y, floor })
                    .ok_or(())?;
                self.work.try_reserve(2).map_err(|_| ())?;
                let mut births = Vec::new();
                births.try_reserve_exact(9).map_err(|_| ())?;
                for _ in 0..9 {
                    let token = events.spawn_source(OmrafirEventBirth::Hellfire);
                    births.push(Birth {
                        proof: runtime
                            .prepare_native_creature_event_birth(
                                native,
                                stamp,
                                occurrence.actor,
                                &token,
                            )
                            .map_err(|_| ())?,
                        child: events.hellfire().clone(),
                        at: None,
                        placement: None,
                        done: false,
                    });
                }
                let reform = if normal {
                    let token = events.spawn_source(OmrafirEventBirth::NormalReform);
                    Some(Birth {
                        proof: runtime
                            .prepare_native_creature_event_birth(
                                native,
                                stamp,
                                occurrence.actor,
                                &token,
                            )
                            .map_err(|_| ())?,
                        child: events.parent().clone(),
                        at: Some(at),
                        placement: None,
                        done: false,
                    })
                } else {
                    None
                };
                let boom = self.token()?;
                let after = self.token()?;
                let mut timers = self.timers.as_ref().ok_or(())?.clone();
                // Both deadlines originate from THIS onThink, never the later boom.
                timers
                    .schedule(
                        runtime.owner_fence().map_err(|_| ())?,
                        stamp,
                        Family::Boom,
                        boom,
                        Some(occurrence.actor),
                        SemanticTimeMicros::from_micros(now.checked_add(10_000).ok_or(())?),
                    )
                    .map_err(|_| ())?;
                timers
                    .schedule(
                        runtime.owner_fence().map_err(|_| ())?,
                        stamp,
                        Family::Reform,
                        after,
                        None,
                        SemanticTimeMicros::from_micros(now.checked_add(26_764_000).ok_or(())?),
                    )
                    .map_err(|_| ())?;
                self.work.push(Work {
                    key: boom,
                    family: Family::Boom,
                    kind: WorkKind::Boom {
                        actor: occurrence.actor,
                        occurrence: *occurrence,
                        births,
                        started: false,
                    },
                });
                self.work.push(Work {
                    key: after,
                    family: Family::Reform,
                    kind: WorkKind::Reform {
                        occurrence: *occurrence,
                        birth: reform.map(Box::new),
                        known_failure: !normal,
                    },
                });
                self.timers = Some(timers);
            }
            self.seen
                .retain(|p| runtime.contains_live_creature(p.actor) && p.actor != occurrence.actor);
            if self.seen.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                return Err(());
            }
            self.seen.push(*occurrence);
        }
        self.drain_events(runtime, room, native, map, ai, now, now)
            .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one native source event uses current physical map and the existing HP owner"
    )]
    pub(super) async fn execute(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        parties: &super::super::physical_spell::ReferenceParties,
        now: u64,
    ) -> Result<(), ()> {
        if self.work.is_empty()
            && !proposals.iter().any(|p| {
                runtime.matches_live_creature_identity(p.issuer, b"oteryn:creature.omrafir")
            })
        {
            return Ok(());
        }
        self.bind(native)?;
        self.lane(runtime)?;
        let overlay = self.overlay.as_ref().ok_or(())?.clone();
        for p in proposals {
            let Some(kind) = overlay.kind(&p.ability) else {
                continue;
            };
            if !runtime.matches_live_creature_identity(p.issuer, b"oteryn:creature.omrafir")
                || p.entry_index != 2
                || p.range_tiles != 0
                || p.magnitude.is_some()
            {
                return Err(());
            }
            oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(
                runtime, p,
            )
            .map_err(|_| ())?;
            let (list, label) = match kind {
                OmrafirSourceCallbackKind::Beam => (
                    crate::foundation::profile_schedule::ScheduleList::Attack,
                    "attack",
                ),
                OmrafirSourceCallbackKind::FireHeal => (
                    crate::foundation::profile_schedule::ScheduleList::Defence,
                    "defence",
                ),
            };
            if p.list != list {
                return Err(());
            }
            let prefix = format!(
                "ai-profile:{}:",
                oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::hex(
                    p.issuer.placement_identity()
                )
            );
            let (sequence, tail) = p
                .occurrence
                .id()
                .as_str()
                .strip_prefix(&prefix)
                .and_then(|s| s.split_once(':'))
                .ok_or(())?;
            let sequence = sequence.parse::<u64>().map_err(|_| ())?;
            if tail != format!("{label}:2") {
                return Err(());
            }
            if self
                .memos
                .get(p.issuer, &p.ability)
                .is_some_and(|m| m.sequence >= sequence)
            {
                continue;
            }
            if self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                self.memos
                    .prune(|actor| runtime.contains_live_creature(actor));
                if self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                    return Err(());
                }
            }
            match kind {
                OmrafirSourceCallbackKind::Beam => {
                    if !marked(runtime, p.issuer, now)? {
                        use crate::foundation::{
                            ActorConditionTransition, ApplicationFacts, ConditionDefinition,
                            ConditionSource, ConditionSourceKind, ConditionValues,
                        };
                        let marker = ConditionDefinition::new(
                            "source.monster.omrafir.regeneration-marker.88888",
                            1,
                            ConditionValues::SpellRegeneration {
                                duration_ms: 5000,
                                sub_id: 88888,
                                health_gain: 0,
                                health_interval_ms: 5000,
                                mana_gain: 0,
                                mana_interval_ms: 1000,
                            },
                        )
                        .ok_or(())?;
                        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
                        let digest = Sha256::digest(p.occurrence.id().as_str().as_bytes());
                        let mut id = [0; 16];
                        id.copy_from_slice(&digest[..16]);
                        let at =
                            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
                        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                        let plan = runtime
                            .prepare_actor_condition(
                                p.issuer,
                                None,
                                ActorConditionTransition::Apply {
                                    definition: &marker,
                                    source: ConditionSource {
                                        actor: p.issuer,
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
                                at,
                            )
                            .map_err(|_| ())?;
                        let token = self.token()?;
                        self.work.try_reserve(1).map_err(|_| ())?;
                        let mut timers = self.timers.as_ref().ok_or(())?.clone();
                        timers
                            .schedule(
                                runtime.owner_fence().map_err(|_| ())?,
                                stamp,
                                Family::Beam,
                                token,
                                Some(p.issuer),
                                SemanticTimeMicros::from_micros(
                                    now.checked_add(4_000_000).ok_or(())?,
                                ),
                            )
                            .map_err(|_| ())?;
                        runtime.commit_actor_condition(&plan, at).map_err(|_| ())?;
                        say(runtime, room, p.issuer, OmrafirSpeech::Inhale)?;
                        self.work.push(Work {
                            key: token,
                            family: Family::Beam,
                            kind: WorkKind::Beam {
                                parent: Box::new(p.clone()),
                                source: None,
                            },
                        });
                        self.timers = Some(timers);
                    }
                }
                OmrafirSourceCallbackKind::FireHeal => {
                    let seed = CallbackSpawnOwner::draw(
                        native.source_digest(),
                        p,
                        "omrafir-exact-item-heal",
                    )
                    .map_err(|_| ())?;
                    let heal = crate::spell::uniform_draw(seed, 7500, 9000) as u64;
                    let snapshot = runtime.companion_snapshot(p.issuer).map_err(|_| ())?;
                    if i128::from(snapshot.health) * 1_000_000
                        < i128::from(snapshot.maximum_health) * 999900
                    {
                        let at = runtime
                            .read_actor_position(p.issuer)
                            .map_err(|_| ())?
                            .position();
                        if map
                            .source_has_any_item(runtime, at, &[1487, 1492, 1500])
                            .await
                            .ok_or(())?
                        {
                            runtime
                                .commit_source_creature_heal_batch(
                                    native.source_digest(),
                                    &[(
                                        p.issuer,
                                        "oteryn:creature.omrafir".to_owned(),
                                        snapshot.maximum_health as u64,
                                        heal,
                                    )],
                                )
                                .map_err(|_| ())?;
                            effect(runtime, room, p.issuer, 13)?;
                            say(runtime, room, p.issuer, OmrafirSpeech::Heal)?;
                        }
                    }
                }
            }
            self.memos.insert(
                p.issuer,
                p.ability.clone(),
                Memo {
                    proposal: p.clone(),
                    sequence,
                },
            );
        }
        self.drain_beams(
            runtime, states, attack, room, native, map, parties, now, now,
        )
        .await?;
        // Global event materialization is driven from the actual think phase;
        // its independent timer wakes that phase even after the original dies.
        let _ = active;
        Ok(())
    }
}
impl NativeOmrafir {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive_due_through(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &super::super::physical_spell::ReferenceParties,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drain_beams(
            runtime,
            states,
            attack,
            room,
            native,
            map,
            parties,
            eligible_through_us,
            now,
        )
        .await?;
        self.drain_events(
            runtime,
            room,
            native,
            map,
            &mut states.monster_melee.creature_ai,
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn drain_beams(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &super::super::physical_spell::ReferenceParties,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        let Some(timers) = self.timers.as_ref() else {
            return Ok(());
        };
        let eligible = self
            .work
            .iter()
            .filter_map(|work| {
                timers
                    .occurrence_due(work.family, work.key)
                    .filter(|due| due.get() <= eligible_through_us)
                    .map(|_| (work.family, work.key))
            })
            .collect::<Vec<_>>();
        let mut preview = timers.clone();
        let fired = preview.drain_due_matching(
            &Now(now),
            runtime.owner_fence().map_err(|_| ())?,
            |_| true,
            |family, key| eligible.contains(&(family, key)),
        );
        for fired in fired {
            let Some(index) = self
                .work
                .iter()
                .position(|w| w.key == fired.occurrence && w.family == Family::Beam)
            else {
                continue;
            };
            let WorkKind::Beam { parent, source } = &mut self.work[index].kind else {
                return Err(());
            };
            if !runtime.contains_live_creature(parent.issuer) {
                self.timers
                    .as_mut()
                    .ok_or(())?
                    .cancel(Family::Beam, fired.occurrence);
                self.work.remove(index);
                continue;
            }
            if source.is_none() {
                let direction = match runtime
                    .read_actor_position(parent.issuer)
                    .map_err(|_| ())?
                    .facing()
                    .ok_or(())?
                {
                    MovementFacing::North => 0,
                    MovementFacing::East => 1,
                    MovementFacing::South => 2,
                    MovementFacing::West => 3,
                };
                let descriptor = self
                    .overlay
                    .as_ref()
                    .ok_or(())?
                    .beam_stage(direction)
                    .ok_or(())?;
                *source=Some(Box::new(crate::creature_damage_spell::composed::ComposedSource::from_omrafir_beam_current(native,&descriptor,runtime,parent).map_err(|_|())?));
            }
            let source = source.as_deref().ok_or(())?;
            let actor = parent.issuer;
            let mut proposal = parent.as_ref().clone();
            proposal.target = actor;
            let atom = format!(
                "actor:{}",
                oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::hex(
                    actor.placement_identity()
                )
            );
            proposal.intent =
                crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).map_err(|_| ())?;
            proposal.occurrence = crate::ability::AbilityOccurrence::new(
                &format!("callback:{}:stage:0", parent.occurrence.id().as_str()),
                parent.occurrence.revisions().clone(),
            )
            .map_err(|_| ())?;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            let Some(reader) = super::super::physical_spell::prepare_composed_reader(
                runtime, states, attack, room, native, map, source, &proposal, parties, now, stamp,
            )
            .await
            else {
                return Err(());
            };
            let Some(mut reader) = super::super::physical_spell::composed::CurrentComposed::prepare(
                runtime, attack, room, source, reader, now,
            ) else {
                return Err(());
            };
            let players = reader.base.players.clone();
            let visuals = reader.base.source_visual_positions();
            let catalog = crate::creature_area_heal::CreatureHealCatalog::from_current_native(
                runtime, native,
            )
            .map_err(|_| ())?;
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            let mut owner = std::mem::take(&mut states.monster_composed);
            let result = owner.execute_current(
                runtime,
                stamp,
                states,
                &catalog,
                source,
                &proposal,
                &mut reader,
                SemanticTimeMicros::from_micros(now),
            );
            states.monster_composed = owner;
            match result {
                Ok(outcome) => {
                    if let Some(outcome) = outcome.primary {
                        super::super::physical_spell::record_hits(
                            runtime,
                            attack,
                            Some(actor),
                            &players,
                            outcome,
                            now,
                        );
                    }
                    use crate::map::boot::BundleCollisionIndexExt;
                    if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index)=room.movement_cells().index(){
                        for at in visuals.into_iter().filter_map(visual_position){index.map().facts.emit_movement_effect(at,7);}
                    }
                    say(runtime, room, actor, OmrafirSpeech::Exhale)?;
                }
                Err(crate::creature_auto_attack::AttackError::MissingCombatFacts) => return Err(()),
                // Actual vanished caster was handled above; current other failures
                // are not a permit to redraw/rebase this source event.
                Err(_) => return Err(()),
            }
            self.timers
                .as_mut()
                .ok_or(())?
                .cancel(Family::Beam, fired.occurrence);
            self.work.remove(index);
        }
        Ok(())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "source event cutoff, real clock, current scope, map and native birth owners are independently checked"
    )]
    async fn drain_events(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        let Some(timers) = self.timers.as_ref() else {
            return Ok(());
        };
        let eligible = self
            .work
            .iter()
            .filter_map(|work| {
                timers
                    .occurrence_due(work.family, work.key)
                    .filter(|due| due.get() <= eligible_through_us)
                    .map(|_| (work.family, work.key))
            })
            .collect::<Vec<_>>();
        let mut preview = timers.clone();
        for fired in preview.drain_due_matching(
            &Now(now),
            runtime.owner_fence().map_err(|_| ())?,
            |_| true,
            |family, key| eligible.contains(&(family, key)),
        ) {
            let Some(index) = self
                .work
                .iter()
                .position(|w| w.key == fired.occurrence && w.family != Family::Beam)
            else {
                continue;
            };
            let world = self.world.as_ref().ok_or(())?;
            if !world.matches(room, runtime.content_pin()) {
                return Err(());
            }
            let family = self.work[index].family;
            match &mut self.work[index].kind {
                WorkKind::Boom {
                    actor,
                    occurrence,
                    births,
                    started,
                } => {
                    if !*started && !runtime.contains_live_creature(*actor) {
                        self.timers
                            .as_mut()
                            .ok_or(())?
                            .cancel(family, fired.occurrence);
                        self.work.remove(index);
                        continue;
                    }
                    for (ordinal, birth) in births.iter_mut().enumerate() {
                        if birth.at.is_none() {
                            let x = 33589
                                + uniform(
                                    native,
                                    occurrence,
                                    "omrafir_boom_x",
                                    ordinal as u64,
                                    -8,
                                    8,
                                )? as i32;
                            let y = 32379
                                + uniform(
                                    native,
                                    occurrence,
                                    "omrafir_boom_y",
                                    ordinal as u64,
                                    -9,
                                    9,
                                )? as i32;
                            birth.at = Some(
                                world
                                    .source_frame()
                                    .physical_position([x, y, 12])
                                    .map(|(x, y, floor)| MovementLocalPosition { x, y, floor })
                                    .ok_or(())?,
                            );
                        }
                        prepare_birth(
                            runtime,
                            native,
                            map,
                            world,
                            birth,
                            true,
                            *occurrence,
                            ordinal,
                        )
                        .await?;
                    }
                    if !map.validate_transaction(runtime).await {
                        return Err(());
                    }
                    if !*started {
                        say(runtime, room, *actor, OmrafirSpeech::Boom)?;
                        self.storage = self.storage.checked_add(1).ok_or(())?;
                        effect(runtime, room, *actor, 37)?;
                        *started = true;
                    }
                    for birth in births {
                        if birth.done {
                            continue;
                        }
                        create_birth(runtime, native, map, world, birth, ai, now)?;
                        // Lua removes the original on every iteration, after the
                        // create attempt, including a known failed creation.
                        if runtime.contains_live_creature(*actor) {
                            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                            runtime
                                .remove_current_native_encounter_subject(
                                    stamp,
                                    *actor,
                                    "oteryn:creature.omrafir",
                                    native.source_digest(),
                                )
                                .map_err(|_| ())?;
                        }
                    }
                }
                WorkKind::Reform {
                    occurrence,
                    birth,
                    known_failure,
                } => {
                    if *known_failure {
                        if self.events.as_ref().ok_or(())?.terminal_source_outcome()
                            != "unknown-monster-nil-then-lua-error"
                        {
                            return Err(());
                        }
                        if self.terminal_failures.len() >= 128 {
                            self.terminal_failures.remove(0);
                        }
                        self.terminal_failures.push(occurrence.actor);
                        // Retain the exact actor-scoped known-failure receipt;
                        // the nil lookup never grants a fabricated creature.
                    } else {
                        let birth = birth.as_mut().ok_or(())?;
                        prepare_birth(runtime, native, map, world, birth, false, *occurrence, 0)
                            .await?;
                        if !map.validate_transaction(runtime).await {
                            return Err(());
                        }
                        create_birth(runtime, native, map, world, birth, ai, now)?;
                    }
                }
                WorkKind::Beam { .. } => return Err(()),
            }
            self.timers
                .as_mut()
                .ok_or(())?
                .cancel(family, fired.occurrence);
            self.work.remove(index);
        }
        Ok(())
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "placement DATA retains its source occurrence while current Map facts refresh"
)]
async fn prepare_birth(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    world: &crate::content::QualifiedPublishedWorldContext,
    birth: &mut Birth,
    extended: bool,
    occurrence: crate::foundation::ThinkOccurrence,
    ordinal: usize,
) -> Result<(), ()> {
    if birth.done {
        return Ok(());
    }
    let at = birth.at.ok_or(())?;
    if birth.placement.is_none() {
        let mut counter = 0_u64;
        birth.placement = Some(
            SourcePlacementPlan::prepare(at, extended, true, |bound| {
                let result = uniform(
                    native,
                    &occurrence,
                    "omrafir_birth_shuffle",
                    (ordinal as u64) * 32 + counter,
                    0,
                    i64::from(bound - 1),
                )
                .map(|v| v as u32)
                .unwrap_or(bound);
                counter = counter.saturating_add(1);
                result
            })
            .map_err(|_| ())?,
        );
    }
    map.read(runtime, at).await;
    if map
        .source_tile_exists_in_world(runtime, at, world)
        .ok_or(())?
    {
        return Ok(());
    }
    for candidate in birth.placement.as_ref().ok_or(())?.candidates() {
        map.read(runtime, *candidate).await;
        map.source_placement_cell_in_world(runtime, &birth.child.key, *candidate, world)
            .ok_or(())?;
        if extended && at.floor != 0 {
            for (step, exempt) in
                crate::durability::native_world_read::source_sight_steps(at, *candidate)
                    .map_err(|_| ())?
            {
                if !exempt {
                    map.read(runtime, step).await;
                    let exists = map
                        .source_tile_exists_in_world(runtime, step, world)
                        .ok_or(())?;
                    if exists {
                        map.view().blocks_projectile(step).ok_or(())?;
                    }
                }
            }
        }
    }
    Ok(())
}
fn create_birth(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &CurrentMonsterMap<'_, '_>,
    world: &crate::content::QualifiedPublishedWorldContext,
    birth: &mut Birth,
    ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
    now: u64,
) -> Result<(), ()> {
    let at = birth.at.ok_or(())?;
    let exists = map
        .source_tile_exists_in_world(runtime, at, world)
        .ok_or(())?;
    let selected = if exists {
        Some(at)
    } else {
        birth
            .placement
            .as_ref()
            .ok_or(())?
            .select(
                |at| map.source_placement_cell_in_world(runtime, &birth.child.key, at, world),
                |from, to| {
                    if from.floor == 0 {
                        return Some(true);
                    }
                    for (step, exempt) in
                        crate::durability::native_world_read::source_sight_steps(from, to).ok()?
                    {
                        if !exempt
                            && map.source_tile_exists_in_world(runtime, step, world)?
                            && map.view().blocks_projectile(step)?
                        {
                            return Some(false);
                        }
                    }
                    Some(true)
                },
            )
            .map_err(|_| ())?
    };
    if let Some(selected) = selected {
        match runtime.admit_prepared_native_creature_event_birth(
            native,
            &mut birth.proof,
            at,
            selected,
            |runtime, selected| {
                if selected == at {
                    map.source_tile_exists_in_world(runtime, selected, world)
                } else {
                    map.source_placement_cell_in_world(runtime, &birth.child.key, selected, world)
                        .map(|c| c.query_add)
                }
            },
        ) {
            Ok(actor) => {
                ai.admit_native_creature(runtime, native, actor, &birth.child, now)
                    .map_err(|_| ())?;
            }
            Err(
                oteryn_gameplay_entities::EncounterSpawnError::MissingMap
                | oteryn_gameplay_entities::EncounterSpawnError::StaleOwner,
            ) => return Err(()),
            // A proved physical creation refusal is the Lua nil result; its
            // loop still removes the original and attempts the remaining eight.
            Err(_) => {}
        }
    }
    birth.done = true;
    Ok(())
}
