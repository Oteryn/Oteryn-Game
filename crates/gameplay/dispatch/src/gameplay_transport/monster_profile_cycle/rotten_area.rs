//! Literal Rotten minion onThink: one retained source cohort, canonical icons/KV.
use super::*;
use crate::ai_think::summon_plan::ReservedNativeThinkBatch;
use crate::content::QualifiedPublishedWorldContext;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::rotten_area::{
    CurrentRottenAreaPlayerFacts, PreparedRottenAreaRead, QualifiedRottenAreaSource,
    QualifiedRottenAreaTarget, QualifiedRottenAreaThink, prepare_current_rotten_area_read,
    read_prepared_rotten_area_players_in_transaction,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_encounters::bosses::rotten_blood::{
    RoomPosition,
    area_icons::{
        self, RottenAreaCurrentFacts, RottenAreaEffect, RottenAreaIcon, RottenAreaIconOwner,
        RottenAreaPlayer, RottenParty,
    },
};
use oteryn_gameplay_entities::carrier::rotten_blood::{RottenBloodIconKey, RottenBloodIconSource};
use oteryn_gameplay_entities::carrier::runtime_actor_source_reservation::{
    SourceActorReservation, SourceActorReservationProof, source_reservation_seal,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

pub(in crate::gameplay_transport) type CacheKey = ([u8; 16], [u8; 32], u64);
#[derive(Debug, Clone)]
struct Alias {
    login: [u8; 32],
    value: u64,
    touched: u64,
    touched_us: u64,
}
#[derive(Debug, Clone, Default)]
pub(in crate::gameplay_transport) struct CacheData {
    owner: RottenAreaIconOwner,
    aliases: Vec<Alias>,
    next_alias: u64,
}
impl CacheData {
    pub(in crate::gameplay_transport) fn empty(&self) -> bool {
        self.aliases.is_empty()
    }
    pub(in crate::gameplay_transport) fn gc(&mut self, now_us: u64) {
        self.aliases.retain(|alias| {
            now_us.saturating_sub(alias.touched_us) < area_icons::PARTY_CACHE_SECONDS * 1_000_000
        });
        let identities = self
            .aliases
            .iter()
            .map(|alias| alias.value)
            .collect::<Vec<_>>();
        self.owner.retain_cached_identities(&identities);
    }
    fn alias(&mut self, login: [u8; 32], now: u64, now_us: u64) -> Option<u64> {
        if login == [0; 32] {
            return None;
        }
        if let Some(alias) = self.aliases.iter_mut().find(|a| a.login == login) {
            alias.touched = now;
            alias.touched_us = now_us;
            return Some(alias.value);
        }
        let value = self.next_alias.checked_add(1)?;
        self.aliases.try_reserve(1).ok()?;
        self.aliases.push(Alias {
            login,
            value,
            touched: now,
            touched_us: now_us,
        });
        self.next_alias = value;
        Some(value)
    }
}
/// Pure source weak caches and one continuation claim, never icon/KV state.
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct SharedCache {
    pub(in crate::gameplay_transport) data: CacheData,
    pub(in crate::gameplay_transport) busy: Option<[u8; 16]>,
    version: u64,
}
#[derive(Debug, Clone)]
struct CohortProof {
    targets: Vec<QualifiedRottenAreaTarget>,
    affected: Vec<ExactActorRef>,
    event: [u8; 16],
    binding: [u8; 32],
}
impl CohortProof {
    fn new(targets: Vec<QualifiedRottenAreaTarget>) -> Option<Self> {
        let first = targets.first()?;
        let parent = *first.affected_actors().first()?;
        let event = first.event_identity();
        let mut affected = Vec::new();
        affected
            .try_reserve_exact(targets.len().checked_add(1)?)
            .ok()?;
        affected.push(parent);
        let mut hash = Sha256::new();
        hash.update(b"oteryn:rotten-area-native-cohort:v1");
        for target in &targets {
            if target.affected_actors() != [parent] || affected.contains(&target.actor()) {
                return None;
            }
            affected.push(target.actor());
            hash.update(target.event_identity());
            hash.update(target.source_binding_digest());
        }
        Some(Self {
            targets,
            affected,
            event,
            binding: hash.finalize().into(),
        })
    }
}
impl source_reservation_seal::Sealed for CohortProof {}
impl SourceActorReservationProof for CohortProof {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.targets[0].actor(), self.targets[0].session())
    }
    fn event_identity(&self) -> [u8; 16] {
        self.event
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.binding
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &self.affected
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.targets.iter().all(|t| t.current_for(runtime))
    }
}
#[derive(Debug, Clone)]
struct Job {
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    input: QualifiedRottenAreaThink,
}
#[derive(Debug, Clone, Copy)]
struct Effect {
    actor: ExactActorRef,
    session: GameSessionId,
    value: RottenAreaEffect,
}
#[derive(Debug)]
struct Pending {
    proof: CohortProof,
    reservation: Option<SourceActorReservation>,
    icon: Arc<RottenBloodIconSource>,
    cache: Arc<Mutex<SharedCache>>,
    version: u64,
    data: CacheData,
    jobs: Vec<Job>,
    effects: Vec<Effect>,
    job_cursor: usize,
    effect_cursor: usize,
    read: bool,
    read_plan: PreparedRottenAreaRead,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    now_us: u64,
}
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct AreaState {
    pending: Option<Pending>,
    completed: Vec<(ExactActorRef, u64)>,
}
impl AreaState {
    fn complete(&mut self, actor: ExactActorRef, sequence: u64) {
        self.completed.retain(|(old, _)| *old != actor);
        self.completed.push((actor, sequence));
    }
    pub(in crate::gameplay_transport) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(in crate::gameplay_transport) fn pending_for(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.pending.as_ref().is_some_and(|p| {
            p.proof
                .targets
                .iter()
                .any(|t| t.actor() == actor && t.session() == session)
        })
    }
}
fn position(at: [i32; 3]) -> Option<RoomPosition> {
    Some(RoomPosition {
        x: at[0],
        y: at[1],
        floor: i16::try_from(at[2]).ok()?,
    })
}
fn draw(
    native: &NativeGameplayState,
    source: ExactActorRef,
    sequence: u64,
    index: u64,
) -> Option<u32> {
    use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"oteryn:rotten-area-cross:v1")
            .chain_update(native.source_digest())
            .chain_update(source.placement_identity())
            .chain_update(source.actor_local_generation().to_be_bytes())
            .finalize()
            .into(),
    );
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&sequence.to_be_bytes());
    bytes[8..].copy_from_slice(&index.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let bound = 201;
    let limit = u64::MAX - u64::MAX % bound;
    for ordinal in 0..64 {
        let value =
            deterministic_decision_u64(&root, occurrence, "rotten_area_white_cross", ordinal)
                .ok()?;
        if value < limit {
            return Some(1300 + (value % bound) as u32);
        }
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn drive(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    batch: &ReservedNativeThinkBatch,
    world: Option<&QualifiedPublishedWorldContext>,
    now: u64,
) -> Result<bool, ()> {
    if !native.qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest()) {
        return Ok(false);
    }
    let Some(world) = world else { return Ok(false) };
    if !world.matches(room, runtime.content_pin()) {
        return Err(());
    }
    runtime.owner_fence().map_err(|_| ())?;
    if states.rotten_area.pending.is_none() {
        states
            .rotten_area
            .completed
            .retain(|(actor, _)| runtime.companion_snapshot(*actor).is_ok());
        for occurrence in batch.occurrences() {
            if states
                .rotten_area
                .completed
                .iter()
                .any(|(actor, seq)| *actor == occurrence.actor && *seq >= occurrence.sequence)
            {
                continue;
            }
            let Ok(snapshot) = runtime.companion_snapshot(occurrence.actor) else {
                continue;
            };
            if !runtime.contains_live_creature(occurrence.actor)
                || !area_icons::is_rotten_area_minion(&snapshot.state.policy.definition_key)
            {
                continue;
            }
            let source = QualifiedRottenAreaSource::from_reserved_current(
                runtime,
                native,
                room,
                world,
                *occurrence,
            )
            .ok_or(())?;
            let mut targets = Vec::new();
            for (actor, _, session) in runtime.positioned_actor_census().map_err(|_| ())? {
                let Some(session) = session else { continue };
                let event = super::super::SecureIdentifiers
                    .source_think_event_id()
                    .ok_or(())?;
                if let Some(target) = source.target(runtime, actor, session, event) {
                    targets.push(target)
                }
            }
            if targets
                .iter()
                .any(|target| states.has_pending_spell_commit(target.actor(), target.session()))
            {
                return Err(());
            }
            if targets.is_empty() {
                states
                    .rotten_area
                    .complete(occurrence.actor, occurrence.sequence);
                continue;
            }
            states
                .rotten_area
                .completed
                .try_reserve(1)
                .map_err(|_| ())?;
            let actors = targets
                .iter()
                .map(|target| (target.actor(), target.session()))
                .collect::<Vec<_>>();
            let read_plan = prepare_current_rotten_area_read(runtime, &actors).ok_or(())?;
            let proof = CohortProof::new(targets).ok_or(())?;
            let icon = Arc::new(
                runtime
                    .qualify_rotten_blood_icon_source(occurrence.actor, native)
                    .map_err(|_| ())?,
            );
            let cache = owner
                .activity_runtimes
                .rotten_area_cache((
                    *runtime.binding().world_id().as_bytes(),
                    native.source_digest(),
                    runtime.content_pin().activation_sequence(),
                ))
                .ok_or(())?;
            let (version, data) = {
                let mut shared = cache.lock().map_err(|_| ())?;
                if shared.busy.is_some() {
                    return Err(());
                }
                shared.version.checked_add(1).ok_or(())?;
                shared.busy = Some(proof.event);
                (shared.version, shared.data.clone())
            };
            let reservation =
                runtime
                    .prepare_source_actor_reservation(&proof)
                    .and_then(|mut value| {
                        runtime.reserve_source_actors(&mut value, &proof)?;
                        Ok(value)
                    });
            let reservation = match reservation {
                Ok(value) => value,
                Err(_) => {
                    let mut shared = cache.lock().map_err(|_| ())?;
                    if shared.busy == Some(proof.event) && shared.version == version {
                        shared.busy = None
                    }
                    return Err(());
                }
            };
            states.rotten_area.pending = Some(Pending {
                proof,
                reservation: Some(reservation),
                icon,
                cache,
                version,
                data,
                jobs: Vec::new(),
                effects: Vec::new(),
                job_cursor: 0,
                effect_cursor: 0,
                read: true,
                read_plan,
                scope: runtime.binding().runtime_scope(),
                generation: runtime.binding().scope_generation().get(),
                now_us: now,
            });
            break;
        }
    }
    #[cfg(test)]
    if let Some(pending) = states.rotten_area.pending.as_ref() {
        eprintln!(
            "ROTTEN_AREA_COHORT_RETAINED sequence={} targets={}",
            pending.proof.targets[0].occurrence().sequence,
            pending.proof.targets.len()
        );
    }
    Ok(states.rotten_area.pending())
}
fn prepare_effects(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    facts: &[CurrentRottenAreaPlayerFacts],
) -> Result<(), ()> {
    let dead = runtime
        .positioned_actor_census()
        .map_err(|_| ())?
        .into_iter()
        .filter_map(|(actor, _, session)| session.filter(|_| states.is_dead(actor)).map(|_| actor))
        .collect::<Vec<_>>();
    let Some(pending) = states.rotten_area.pending.as_mut() else {
        return Err(());
    };
    runtime
        .validate_source_actor_reservation(pending.reservation.as_ref().ok_or(())?, &pending.proof)
        .map_err(|_| ())?;
    if !pending.read {
        return Ok(());
    }
    if facts.len() != pending.proof.targets.len() {
        return Err(());
    }
    if !pending.read_plan.current_for(runtime)
        || facts.iter().any(|fact| !fact.current_for(runtime))
    {
        return Err(());
    }
    let mut data = pending.data.clone();
    let mut aliases = Vec::new();
    let mut players = Vec::new();
    let mut current = Vec::new();
    let mut parties = Vec::new();
    let mut jobs = Vec::new();
    for target in &pending.proof.targets {
        let fact = facts
            .iter()
            .find(|f| f.taint().actor == target.actor() && f.taint().session == target.session())
            .ok_or(())?;
        let at = position(target.player_source_position().ok_or(())?).ok_or(())?;
        let taint = fact.taint();
        let wall = u64::try_from(taint.observed_at_seconds).map_err(|_| ())?;
        data.aliases
            .retain(|alias| wall.saturating_sub(alias.touched) < area_icons::PARTY_CACHE_SECONDS);
        let alias = data
            .alias(fact.login_incarnation().ok_or(())?, wall, pending.now_us)
            .ok_or(())?;
        let leader = taint
            .party
            .map(|p| {
                data.alias(
                    p.leader_login_incarnation().ok_or(())?,
                    wall,
                    pending.now_us,
                )
                .ok_or(())
            })
            .transpose()?;
        aliases.push((alias, target.actor(), target.session(), leader));
        parties.push(taint.party);
        if let Some(input) = QualifiedRottenAreaThink::from_reserved_target(runtime, target, fact) {
            jobs.push(Job {
                actor: target.actor(),
                fence: fact.fence(),
                input,
            });
        }
        let icons = runtime
            .rotten_blood_keyed_icons(target.actor())
            .map_err(|_| ())?;
        let count = |key| {
            icons
                .iter()
                .flatten()
                .find(|entry| entry.0 == key)
                .map(|entry| u8::try_from(entry.3).map_err(|_| ()))
                .transpose()
        };
        let clocks = fact.clocks();
        current.push(RottenAreaCurrentFacts {
            actor: alias,
            last_hazard: u64::try_from(clocks[0]).map_err(|_| ())?,
            last_area: u64::try_from(clocks[1]).map_err(|_| ())?,
            last_cross: u64::try_from(clocks[2]).map_err(|_| ())?,
            area_count: count("rotten-area")?,
            cross_count: count("white-cross")?,
        });
        // Party slices refer only to this immutable facts vector through evaluation.
        players.push((
            alias,
            at,
            area_icons::donor_taint_level(&taint.state),
            !dead.contains(&target.actor()),
        ));
    }
    let spectators = players
        .iter()
        .enumerate()
        .map(|(index, (actor, at, level, alive))| RottenAreaPlayer {
            actor: *actor,
            position: *at,
            taint_level: *level,
            alive: *alive,
            party: parties[index]
                .as_ref()
                .zip(aliases[index].3)
                .map(|(party, leader)| RottenParty {
                    leader,
                    taint_levels: party.taint_levels(),
                }),
        })
        .collect::<Vec<_>>();
    let wall =
        u64::try_from(facts.first().ok_or(())?.taint().observed_at_seconds).map_err(|_| ())?;
    let occurrence = pending.proof.targets[0].occurrence();
    let mut ordinal = 0;
    let mut failed = false;
    let effects = data
        .owner
        .think_current(
            position(pending.proof.targets[0].source_position()).ok_or(())?,
            wall,
            &spectators,
            &current,
            || {
                let value = draw(native, occurrence.actor, occurrence.sequence, ordinal);
                ordinal += 1;
                value.unwrap_or_else(|| {
                    failed = true;
                    0
                })
            },
        )
        .map_err(|_| ())?;
    if failed {
        return Err(());
    }
    let effects = effects
        .into_iter()
        .map(|value| {
            let actor = match value {
                RottenAreaEffect::SetIcon { actor, .. }
                | RottenAreaEffect::RemoveIcon { actor, .. }
                | RottenAreaEffect::EnergyDamage { actor, .. } => actor,
            };
            let (_, actor, session, _) = aliases.iter().find(|entry| entry.0 == actor).ok_or(())?;
            Ok(Effect {
                actor: *actor,
                session: *session,
                value,
            })
        })
        .collect::<Result<Vec<_>, ()>>()?;
    jobs.retain(|job| effects.iter().any(|effect| effect.actor == job.actor));
    pending.data = data;
    pending.jobs = jobs;
    pending.effects = effects;
    pending.read = false;
    #[cfg(test)]
    eprintln!(
        "ROTTEN_AREA_PREPARED_QUEST jobs={} effects={}",
        pending.jobs.len(),
        pending.effects.len()
    );
    Ok(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn read_rotten_area_facts(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: u64,
        plan: &PreparedRottenAreaRead,
    ) -> Option<Vec<CurrentRottenAreaPlayerFacts>> {
        use crate::durability::spell_item_transaction::{
            assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
        };
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, permit, scope, generation, plan);
        let result = pass
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, permit, scope, generation, plan) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        *scope,
                        *generation,
                    )
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    let facts = read_prepared_rotten_area_players_in_transaction(
                        &mut tx, owner.root, &authority, plan,
                    )
                    .await?;
                    tx.commit()
                        .await
                        .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    Ok(facts)
                })
            })
            .await;
        #[cfg(test)]
        match &result {
            Ok(facts) => eprintln!(
                "ROTTEN_AREA_DETACHED_READ count={} login_digests={}",
                facts.len(),
                facts.iter().all(|fact| fact.login_incarnation().is_some())
            ),
            Err(error) => eprintln!("ROTTEN_AREA_DETACHED_READ_REFUSED {error:?}"),
        }
        result.ok()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The caller keeps the physical lane but has released runtime/states/attack.
    pub(in crate::gameplay_transport) async fn finish_rotten_area(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        use crate::durability::quest_state::QuestTransitionOutcome;
        let read = {
            let runtime = self.runtime.lock().await;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return false;
            };
            let Some(p) = states.rotten_area.pending.as_ref() else {
                return true;
            };
            if runtime
                .validate_source_actor_reservation(
                    match p.reservation.as_ref() {
                        Some(value) => value,
                        None => return false,
                    },
                    &p.proof,
                )
                .is_err()
            {
                return false;
            }
            p.read.then(|| (p.scope, p.generation, p.read_plan.clone()))
        };
        if let Some((scope, generation, plan)) = read {
            let Some(facts) = self
                .read_rotten_area_facts(permit, scope, generation, &plan)
                .await
            else {
                return false;
            };
            let runtime = self.runtime.lock().await;
            let Ok(mut states) = self.states.spell_states.try_lock() else {
                return false;
            };
            let Some(native) = self
                .active_generation
                .and_then(|active| active.native_gameplay())
            else {
                return false;
            };
            if !plan.current_for(&runtime)
                || prepare_effects(&runtime, &mut states, native, &facts).is_err()
            {
                return false;
            }
        }
        loop {
            let job = {
                let runtime = self.runtime.lock().await;
                let Ok(states) = self.states.spell_states.try_lock() else {
                    return false;
                };
                let Some(p) = states.rotten_area.pending.as_ref() else {
                    return true;
                };
                if p.read {
                    return false;
                }
                if runtime
                    .validate_source_actor_reservation(
                        match p.reservation.as_ref() {
                            Some(r) => r,
                            None => return false,
                        },
                        &p.proof,
                    )
                    .is_err()
                {
                    return false;
                }
                p.jobs.get(p.job_cursor).cloned()
            };
            let Some(job) = job else { break };
            let Some(catalogue) = self.quest_catalogue.as_ref() else {
                return false;
            };
            let mut slot = self
                .revision_sequencer
                .acquire(job.fence.character_id)
                .await;
            let outcome = slot
                .commit_rotten_area_think(
                    self.root,
                    self.character,
                    self.holder,
                    job.fence,
                    job.input,
                    Arc::clone(catalogue),
                )
                .await;
            match outcome {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => {
                    let mut states = self.states.spell_states.lock().await;
                    let Some(p) = states.rotten_area.pending.as_mut() else {
                        return false;
                    };
                    p.job_cursor += 1;
                }
                _other => {
                    #[cfg(test)]
                    eprintln!("ROTTEN_AREA_RETAINED_QUEST_OUTCOME {_other:?}");
                    return false;
                }
            }
        }
        let mut runtime = self.runtime.lock().await;
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return false;
        };
        let Some(mut p) = states.rotten_area.pending.take() else {
            return true;
        };
        let ready = (|| {
            let mut cache = p.cache.lock().map_err(|_| ())?;
            if cache.busy != Some(p.proof.event) || cache.version != p.version {
                return Err(());
            }
            let version = cache.version.checked_add(1).ok_or(())?;
            let reserved = p.reservation.take().ok_or(())?;
            runtime
                .release_source_actors_after_observed_outcome(reserved, &p.proof)
                .map_err(|_| ())?;
            while let Some(effect) = p.effects.get(p.effect_cursor).copied() {
                let key = |icon| match icon {
                    RottenAreaIcon::Hazard => RottenBloodIconKey::Hazard,
                    RottenAreaIcon::AreaArrowUp => RottenBloodIconKey::Area,
                    RottenAreaIcon::WhiteCross => RottenBloodIconKey::WhiteCross,
                };
                let applied = match effect.value {
                    RottenAreaEffect::SetIcon { icon, count, .. } => runtime
                        .set_rotten_blood_player_icon(
                            &p.icon,
                            effect.actor,
                            effect.session,
                            key(icon),
                            Some(u16::from(count)),
                        )
                        .is_ok(),
                    RottenAreaEffect::RemoveIcon { icon, .. } => runtime
                        .set_rotten_blood_player_icon(
                            &p.icon,
                            effect.actor,
                            effect.session,
                            key(icon),
                            None,
                        )
                        .is_ok(),
                    RottenAreaEffect::EnergyDamage { amount, .. } => {
                        // Lua ignores doTargetCombatHealth's result. A known HP
                        // refusal never rolls back its already-applied icon/KV.
                        let _ = states.apply_creature_damage_kind(
                            &runtime,
                            effect.actor,
                            effect.session,
                            amount,
                            crate::player_lethal::PlayerHitKind::Energy,
                            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                                p.now_us,
                            ),
                            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
                        );
                        true
                    }
                };
                if !applied {
                    return Err(());
                }
                p.effect_cursor += 1;
            }
            cache.data = std::mem::take(&mut p.data);
            cache.version = version;
            cache.busy = None;
            Ok(())
        })();
        if ready.is_err() {
            if p.reservation.is_none() {
                p.reservation = runtime
                    .prepare_source_actor_reservation(&p.proof)
                    .and_then(|mut reservation| {
                        runtime.reserve_source_actors(&mut reservation, &p.proof)?;
                        Ok(reservation)
                    })
                    .ok();
            }
            states.rotten_area.pending = Some(p);
            return false;
        }
        let occurrence = p.proof.targets[0].occurrence();
        states
            .rotten_area
            .complete(occurrence.actor, occurrence.sequence);
        true
    }
}
