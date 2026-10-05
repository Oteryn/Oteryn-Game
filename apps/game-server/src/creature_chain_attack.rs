//! Source-bound sequential native chain work, using accepted picker and Foundation timers.
//! Frozen targets are exact live ActorRefs, never manufactured network creature identities.
use crate::ability::{AbilityIntent, ProposalSource};
use crate::ai_think::profile_schedule::ProfileAbilityProposal;
use crate::creature_auto_attack::AttackError;
use crate::creature_damage_spell::{DamageSpellOwner, SpellOutcome, SpellSource, SpellWorldReader};
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane,
    SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::player_lethal::PlayerLethalVitals;
use crate::spell::chain::{
    ChainCreature, ChainShape, ChainSpec, ChainStart, ChainWorld, TilePosition, pick_chain,
};
use sha2::{Digest, Sha256};
/// Current map-owner facts, not inferred from native geometry or precomputed source records.
pub(crate) trait ChainMapFacts {
    fn sight(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: TilePosition,
        to: TilePosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn path(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: TilePosition,
        to: TilePosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<TilePosition>>;
}
struct PickWorld {
    caster: ChainCreature,
    players: Vec<ChainCreature>,
    allowed: std::collections::BTreeSet<u64>,
    sight: std::collections::BTreeMap<(TilePosition, TilePosition), bool>,
    paths: std::collections::BTreeMap<(TilePosition, TilePosition), Vec<TilePosition>>,
}
impl ChainWorld for PickWorld {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }
    fn creatures(&self) -> &[ChainCreature] {
        &self.players
    }
    fn may_hit(&self, c: &ChainCreature) -> bool {
        self.allowed.contains(&c.id)
    }
    fn sight_clear(&self, a: TilePosition, b: TilePosition) -> bool {
        self.sight.get(&(a, b)).copied().unwrap_or(false)
    }
    fn path(&self, a: TilePosition, b: TilePosition) -> Option<Vec<TilePosition>> {
        self.paths.get(&(a, b)).cloned()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Family(u8);
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        AI01_PENDING_TIMERS_PER_ACTOR
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StepKey {
    cast: u64,
    step: u8,
}
struct Target {
    actor: ExactActorRef,
    session: GameSessionId,
    delay: u64,
    from: TilePosition,
    effects: Vec<TilePosition>,
}
struct Cast {
    token: u64,
    issuer: ExactActorRef,
    entry: usize,
    sequence: u64,
    binding: [u8; 32],
    source: SpellSource,
    proposal: ProfileAbilityProposal,
    targets: Vec<Target>,
    next: usize,
    start: SemanticTimeMicros,
    receipts: Vec<Result<SpellOutcome, AttackError>>,
}
struct Memo {
    issuer: ExactActorRef,
    entry: usize,
    sequence: u64,
    at: SemanticTimeMicros,
    binding: [u8; 32],
    result: Result<u64, AttackError>,
}
/// Sealed one-use capability produced only by draining this lane. No Clone/Copy constructor.
pub(crate) struct ChainPulse {
    source: SpellSource,
    proposal: ProfileAbilityProposal,
    session: GameSessionId,
}
impl ChainPulse {
    pub(crate) fn into_owned_parts(self) -> (SpellSource, ProfileAbilityProposal, GameSessionId) {
        (self.source, self.proposal, self.session)
    }
}
pub(crate) struct ChainOwner {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Family, StepKey>,
    casts: Vec<Cast>,
    memos: Vec<Memo>,
    next_token: u64,
}
impl ChainOwner {
    pub(crate) fn new(
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
    ) -> Result<Self, AttackError> {
        let b = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        let generation = b.scope_generation();
        if !fence.is_current_for_scope(scope, generation) {
            return Err(AttackError::StaleOwner);
        }
        let timers = OwnerTimerLane::for_generation(
            scope,
            generation,
            (0..16).map(|i| {
                (
                    Family(i),
                    FamilyPolicy {
                        max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )
            }),
        )
        .map_err(|_| AttackError::InvalidPlan)?;
        Ok(Self {
            scope,
            generation,
            timers,
            casts: Vec::new(),
            memos: Vec::new(),
            next_token: 1,
        })
    }
    fn current(&self, runtime: &ChannelRuntimeV1, fence: &ScopeRuntimeFence) -> bool {
        let b = runtime.binding();
        self.scope == RuntimeScopeRefV1::channel(b.world_id(), b.channel_id())
            && self.generation == b.scope_generation()
            && fence.is_current_for_scope(self.scope, self.generation)
    }
    fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        let dead: Vec<_> = self
            .casts
            .iter()
            .filter(|c| {
                !runtime.contains_live_creature(c.issuer)
                    || runtime.content_pin().server_artifact_digest() != c.source.chain_binding().3
            })
            .map(|c| (c.entry, c.token, c.next))
            .collect();
        for (e, token, step) in dead {
            self.timers.cancel(
                Family(e as u8),
                StepKey {
                    cast: token,
                    step: step as u8,
                },
            );
        }
        self.casts.retain(|c| {
            runtime.contains_live_creature(c.issuer)
                && runtime.content_pin().server_artifact_digest() == c.source.chain_binding().3
        });
        self.memos
            .retain(|m| runtime.contains_live_creature(m.issuer));
    }
    /// Existing source schedule owns interval/chance. Retrying the same prepared proposal never
    /// adds timers. A source entry has one active cast, at most one next-step timer.
    // Keep schedule ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn schedule(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        world: &mut dyn SpellWorldReader,
        map: &mut dyn ChainMapFacts,
        now: SemanticTimeMicros,
    ) -> Result<u64, AttackError> {
        if !self.current(runtime, fence) || !fence.accepts_stamp(stamp) {
            return Err(AttackError::StaleOwner);
        }
        self.prune(runtime);
        let (key, entry, source_binding, digest) = source.chain_binding();
        if digest != runtime.content_pin().server_artifact_digest() {
            return Err(AttackError::ContentChanged);
        }
        if !runtime.matches_live_creature_identity(proposal.issuer, key.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        let sequence = source.validate_chain_proposal(proposal)?;
        source
            .chain_details()
            .ok_or(AttackError::UnsupportedShape)?;
        let binding = Sha256::digest(
            serde_json::to_vec(&(source_binding, format!("{proposal:?}")))
                .map_err(|_| AttackError::InvalidSource)?,
        )
        .into();
        let memo = self
            .memos
            .iter()
            .position(|m| m.issuer == proposal.issuer && m.entry == entry);
        if let Some(i) = memo {
            let m = &self.memos[i];
            if sequence == m.sequence {
                return if binding == m.binding {
                    m.result
                } else {
                    Err(AttackError::OccurrenceConflict)
                };
            }
            if sequence < m.sequence {
                return Err(AttackError::OccurrenceSuperseded);
            }
            if now <= m.at {
                return Err(AttackError::NotDue);
            }
        }
        if self
            .casts
            .iter()
            .any(|c| c.issuer == proposal.issuer && c.entry == entry && c.next < c.targets.len())
        {
            return Err(AttackError::NotDue);
        }
        // A completed same-key cast is a replacement, not an additional quota slot.
        // Active-cast and occurrence replay gates above remain authoritative.
        let replaces_completed = self
            .casts
            .iter()
            .any(|c| c.issuer == proposal.issuer && c.entry == entry && c.next == c.targets.len());
        admit_cast_capacity(
            self.casts.len(),
            replaces_completed,
            self.memos.len(),
            memo.is_some(),
        )?;
        self.casts.retain(|c| {
            !(c.issuer == proposal.issuer && c.entry == entry && c.next == c.targets.len())
        });
        self.casts
            .try_reserve(1)
            .map_err(|_| AttackError::LedgerFull)?;
        if memo.is_none() {
            self.memos
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
        }
        let token = self.next_token;
        self.next_token = token.checked_add(1).ok_or(AttackError::NumericOverflow)?;
        let result = self
            .pick(runtime, source, proposal, world, map, stamp)
            .and_then(|targets| {
                if targets.is_empty() {
                    return Err(AttackError::StaleTarget);
                }
                for t in &targets {
                    now.get()
                        .checked_add(t.delay)
                        .ok_or(AttackError::NumericOverflow)?;
                }
                let mut receipts = Vec::new();
                receipts
                    .try_reserve_exact(targets.len())
                    .map_err(|_| AttackError::LedgerFull)?;
                self.timers
                    .schedule(
                        fence,
                        stamp,
                        Family(entry as u8),
                        StepKey {
                            cast: token,
                            step: 0,
                        },
                        Some(proposal.issuer),
                        now,
                    )
                    .map_err(|_| AttackError::InvalidPlan)?;
                self.casts.push(Cast {
                    token,
                    issuer: proposal.issuer,
                    entry,
                    sequence,
                    binding,
                    source: source.clone(),
                    proposal: proposal.clone(),
                    targets,
                    next: 0,
                    start: now,
                    receipts,
                });
                Ok(token)
            });
        let value = Memo {
            issuer: proposal.issuer,
            entry,
            sequence,
            at: now,
            binding,
            result,
        };
        match memo {
            Some(i) => self.memos[i] = value,
            None => self.memos.push(value),
        }
        result
    }
    fn pick(
        &self,
        runtime: &ChannelRuntimeV1,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        world: &mut dyn SpellWorldReader,
        map: &mut dyn ChainMapFacts,
        stamp: RuntimeWorkStamp,
    ) -> Result<Vec<Target>, AttackError> {
        let at = runtime
            .read_actor_position(proposal.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let a = at.position();
        let caster = ChainCreature {
            id: proposal.issuer.chain_local_actor_id(),
            actor: atom(proposal.issuer),
            position: TilePosition {
                x: a.x,
                y: a.y,
                floor: a.floor,
            },
        };
        let players = world
            .current_players(runtime, stamp)
            .ok_or(AttackError::MissingCombatFacts)?;
        if players.len() > 64 {
            return Err(AttackError::LedgerFull);
        }
        let mut lookup = std::collections::BTreeMap::new();
        let mut core = PickWorld {
            caster,
            players: Vec::new(),
            allowed: Default::default(),
            sight: Default::default(),
            paths: Default::default(),
        };
        for (actor, session) in players {
            let control = runtime
                .player_control_facts(actor, session)
                .map_err(|_| AttackError::StaleTarget)?;
            if control.control_loss.is_some() {
                continue;
            }
            let pos = runtime
                .read_actor_position(actor)
                .map_err(|_| AttackError::StaleTarget)?;
            if pos.context() != at.context() {
                continue;
            }
            let p = pos.position();
            let id = actor.chain_local_actor_id();
            if lookup.insert(id, (actor, session)).is_some() {
                return Err(AttackError::InvalidPlan);
            }
            let facts = world
                .combat(
                    runtime,
                    proposal.issuer,
                    actor,
                    session,
                    source.element(),
                    stamp,
                )
                .ok_or(AttackError::MissingCombatFacts)?;
            let f = facts.attack;
            if f.issuer != proposal.issuer
                || f.target != actor
                || f.session != session
                || f.revision == 0
            {
                return Err(AttackError::MissingCombatFacts);
            }
            if f.visible
                && !f.issuer_pz
                && !f.target_pz
                && !f.issuer_protected
                && !f.target_protected
            {
                core.allowed.insert(id);
            }
            core.players.push(ChainCreature {
                id,
                actor: atom(actor),
                position: TilePosition {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                },
            });
        }
        let mut positions = vec![core.caster.position];
        positions.extend(core.players.iter().map(|c| c.position));
        let chain = source
            .chain_details()
            .ok_or(AttackError::UnsupportedShape)?;
        for from in &positions {
            for to in &positions {
                let sight = map
                    .sight(runtime, *from, *to, stamp)
                    .ok_or(AttackError::MissingCombatFacts)?;
                core.sight.insert((*from, *to), sight);
                if chain.chain_asset_binding.is_some() {
                    let path = map
                        .path(runtime, *from, *to, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?;
                    core.paths.insert((*from, *to), path);
                }
            }
        }
        let range = if source.range_tiles() == 0 {
            u32::MAX
        } else {
            u32::from(source.range_tiles())
        };
        let spec = ChainSpec {
            max_targets: u32::from(chain.max_targets),
            range_tiles: u32::from(chain.range_tiles),
            initial_range_tiles: range,
            shape: ChainShape::Sequential,
            damage_step_percent: 0,
            asset_binding: chain.chain_asset_binding.clone(),
        };
        let hits = pick_chain(
            &spec,
            &core,
            ChainStart {
                target: Some(proposal.target.chain_local_actor_id()),
                attacked: None,
            },
            range,
        );
        hits.into_iter()
            .map(|h| {
                let (actor, session) = lookup
                    .get(&h.creature)
                    .copied()
                    .ok_or(AttackError::StaleTarget)?;
                Ok(Target {
                    actor,
                    session,
                    delay: h.delay_micros,
                    from: h.from,
                    effects: h.effect_tiles,
                })
            })
            .collect()
    }
    /// Drain delivers only opaque native timer keys. This owner removes the pending step first,
    /// creates a non-Clone pulse, validates current world facts and commits actual HP/MP once.
    // Keep run_due ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_due(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &mut ScopeRuntimeFence,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &DamageSpellOwner,
        world: &mut dyn SpellWorldReader,
        map: &mut dyn ChainMapFacts,
        clock: &impl OwnerClock,
    ) -> ChainPulseResults {
        if !self.current(runtime, fence) {
            return Err(AttackError::StaleOwner);
        }
        self.prune(runtime);
        let fired = self
            .timers
            .drain_due(clock, fence, |a| runtime.contains_live_creature(a));
        let mut results = Vec::new();
        results
            .try_reserve_exact(fired.len())
            .map_err(|_| AttackError::LedgerFull)?;
        for timer in fired {
            let Some(index) = self.casts.iter().position(|c| {
                c.token == timer.occurrence.cast && c.next == usize::from(timer.occurrence.step)
            }) else {
                continue;
            };
            let mut cast = self.casts.remove(index);
            let step = cast.next;
            cast.next += 1;
            let ordinal = fence
                .accept_input(self.generation)
                .map_err(|_| AttackError::StaleOwner)?;
            let stamp = fence.stamp(ordinal);
            let target = &cast.targets[step];
            let mut proposal = cast.proposal.clone();
            proposal.target = target.actor;
            proposal.occurrence =
                crate::spell::plan::chain_step_occurrence(&proposal.occurrence, step)
                    .map_err(|_| AttackError::InvalidPlan)?;
            proposal.intent = AbilityIntent::normalize(
                ProposalSource::Ai,
                &atom(proposal.issuer),
                &[&atom(target.actor)],
            )
            .map_err(|_| AttackError::InvalidPlan)?;
            let outcome = match runtime.read_actor_position(target.actor) {
                Err(_) => Err(AttackError::StaleTarget),
                Ok(position) => {
                    let to = position.position();
                    let current_to = TilePosition {
                        x: to.x,
                        y: to.y,
                        floor: to.floor,
                    };
                    match map.sight(runtime, target.from, current_to, stamp) {
                        None => Err(AttackError::MissingCombatFacts),
                        Some(false) => Err(AttackError::OutOfRange),
                        Some(true) => {
                            let pulse = ChainPulse {
                                source: cast.source.clone(),
                                proposal,
                                session: target.session,
                            };
                            damage.consume_chain_pulse(
                                runtime,
                                fence,
                                stamp,
                                vitals,
                                pulse,
                                world,
                                clock.now(),
                            )
                        }
                    }
                }
            };
            cast.receipts.push(outcome.clone());
            results.push((cast.token, step, outcome));
            if cast.next < cast.targets.len() {
                let due = cast
                    .start
                    .get()
                    .checked_add(cast.targets[cast.next].delay)
                    .ok_or(AttackError::NumericOverflow)?;
                if self
                    .timers
                    .schedule(
                        fence,
                        stamp,
                        Family(cast.entry as u8),
                        StepKey {
                            cast: cast.token,
                            step: cast.next as u8,
                        },
                        Some(cast.issuer),
                        SemanticTimeMicros::from_micros(due),
                    )
                    .is_err()
                {
                    cast.receipts.push(Err(AttackError::InvalidPlan));
                    cast.next = cast.targets.len();
                }
                self.casts.push(cast);
            } else {
                self.casts.push(cast);
            }
        }
        Ok(results)
    }
    pub(crate) fn receipts(&self, token: u64) -> Option<&[Result<SpellOutcome, AttackError>]> {
        self.casts
            .iter()
            .find(|c| c.token == token)
            .map(|c| c.receipts.as_slice())
    }
}
fn atom(a: ExactActorRef) -> String {
    format!(
        "actor:{}",
        a.placement_identity()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

fn admit_cast_capacity(
    casts: usize,
    replaces_completed: bool,
    memos: usize,
    existing_memo: bool,
) -> Result<(), AttackError> {
    let projected = casts.saturating_sub(usize::from(replaces_completed));
    if projected >= 64 * 16 || (!existing_memo && memos >= 64 * 16) {
        Err(AttackError::LedgerFull)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod capacity_tests {
    use super::*;
    #[test]
    fn full_retained_chain_ledger_accepts_existing_completed_replacement_only() {
        // Private quota-boundary state fixture, NOT1024 source-qualified gameplay executions.
        // The production schedule calls the same gate after replay and active-cast checks.
        let full = (0..64)
            .flat_map(|actor| (0..16).map(move |entry| (actor, entry)))
            .collect::<Vec<_>>();
        assert_eq!(full.len(), 1024);
        let replacement = full.iter().position(|key| *key == (63, 15)).unwrap();
        assert_eq!(
            admit_cast_capacity(full.len(), replacement < full.len(), full.len(), true),
            Ok(())
        );
        assert_eq!(
            admit_cast_capacity(full.len(), false, full.len(), true),
            Err(AttackError::LedgerFull)
        );
        assert_eq!(
            admit_cast_capacity(full.len(), true, full.len(), false),
            Err(AttackError::LedgerFull)
        );
        assert_eq!(
            admit_cast_capacity(full.len() - 1, false, full.len() - 1, false),
            Ok(())
        );
    }
}

pub(crate) type ChainPulseResults =
    Result<Vec<(u64, usize, Result<SpellOutcome, AttackError>)>, AttackError>;
