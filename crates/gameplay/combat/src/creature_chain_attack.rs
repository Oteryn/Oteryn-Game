//! Source-bound sequential native chain work, using accepted picker and Foundation timers.
//! Frozen targets are exact live ActorRefs, never manufactured network creature identities.
use crate::ability::{AbilityIntent, ProposalSource};
use crate::creature_auto_attack::{AttackError, ProfileAbilityProposal};
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane,
    SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::player_lethal::PlayerLethalVitals;
use oteryn_gameplay_combat_rules::chain::{
    ChainCreature, ChainShape, ChainSpec, ChainStart, TilePosition, pick_chain,
};
use sha2::{Digest, Sha256};
/// Source validation stays with the higher-layer native spell owner.
pub trait ChainSource: Clone {
    type Outcome: Clone;

    fn chain_binding(&self) -> (&str, usize, [u8; 32], [u8; 32]);
    fn validate_chain_proposal(
        &self,
        proposal: &ProfileAbilityProposal,
    ) -> Result<u64, AttackError>;
    fn chain_details(&self) -> Option<&crate::content::ProjectV2Chain>;
    fn range_tiles(&self) -> u16;
    fn element(&self) -> Option<&str>;
}

/// Independently resolved player and combat facts needed by the chain picker.
pub trait ChainWorldReader {
    fn current_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>>;
    fn attack_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<crate::creature_auto_attack::AttackFacts>;
}

/// The higher layer consumes the sealed pulse through its existing damage owner.
pub trait ChainDamage<S: ChainSource> {
    type World<'a>: ChainWorldReader + ?Sized;

    #[allow(clippy::too_many_arguments)]
    fn consume_chain_pulse(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        pulse: ChainPulse<S>,
        world: &mut Self::World<'_>,
        now: SemanticTimeMicros,
    ) -> Result<S::Outcome, AttackError>;

    #[allow(clippy::too_many_arguments)]
    fn consume_chain_pulse_current(
        &self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        pulse: ChainPulse<S>,
        world: &mut Self::World<'_>,
        now: SemanticTimeMicros,
    ) -> Result<S::Outcome, AttackError>;
}

/// Current map-owner facts, not inferred from native geometry or precomputed source records.
pub trait ChainMapFacts {
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
pub use oteryn_gameplay_combat_rules::chain::PickWorld;
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
#[allow(dead_code, reason = "retained source chain presentation metadata")]
struct Target {
    actor: ExactActorRef,
    session: GameSessionId,
    delay: u64,
    from: TilePosition,
    effects: Vec<TilePosition>,
}
#[allow(dead_code, reason = "retained source chain occurrence metadata")]
struct Cast<S: ChainSource> {
    token: u64,
    issuer: ExactActorRef,
    entry: usize,
    sequence: u64,
    binding: [u8; 32],
    source: S,
    proposal: ProfileAbilityProposal,
    targets: Vec<Target>,
    next: usize,
    start: SemanticTimeMicros,
    receipts: Vec<Result<S::Outcome, AttackError>>,
}
struct Memo {
    issuer: ExactActorRef,
    entry: usize,
    sequence: u64,
    at: SemanticTimeMicros,
    binding: [u8; 32],
    result: Result<u64, AttackError>,
}
/// Read-only next-step metadata for independently preparing current map and effect facts.
/// This is not a pulse and cannot authorize damage execution.
pub struct PendingChainSource<S> {
    pub source: S,
    pub proposal: ProfileAbilityProposal,
    pub session: GameSessionId,
    pub due: SemanticTimeMicros,
    pub from: TilePosition,
}
/// Sealed one-use capability produced only by draining this lane. No Clone/Copy constructor.
pub struct ChainPulse<S> {
    source: S,
    proposal: ProfileAbilityProposal,
    session: GameSessionId,
}
impl<S> ChainPulse<S> {
    pub fn into_owned_parts(self) -> (S, ProfileAbilityProposal, GameSessionId) {
        (self.source, self.proposal, self.session)
    }
}
pub struct ChainOwner<S: ChainSource> {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    timers: OwnerTimerLane<Family, StepKey>,
    casts: Vec<Cast<S>>,
    memos: Vec<Memo>,
    next_token: u64,
}
impl<S: ChainSource> std::fmt::Debug for ChainOwner<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChainOwner")
            .field("scope", &self.scope)
            .field("generation", &self.generation)
            .field("casts", &self.casts.len())
            .field("memos", &self.memos.len())
            .finish()
    }
}
impl<S: ChainSource> ChainOwner<S> {
    pub fn new(runtime: &ChannelRuntimeV1, fence: &ScopeRuntimeFence) -> Result<Self, AttackError> {
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
    pub fn new_current(runtime: &ChannelRuntimeV1) -> Result<Self, AttackError> {
        let fence = runtime.owner_fence().map_err(|_| AttackError::StaleOwner)?;
        Self::new(runtime, fence)
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
    pub fn schedule(
        &mut self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &S,
        proposal: &ProfileAbilityProposal,
        world: &mut (impl ChainWorldReader + ?Sized),
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
    /// Admit the same source schedule under the runtime's independently current fence.
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_current(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        source: &S,
        proposal: &ProfileAbilityProposal,
        world: &mut (impl ChainWorldReader + ?Sized),
        map: &mut dyn ChainMapFacts,
        now: SemanticTimeMicros,
    ) -> Result<u64, AttackError> {
        let fence = runtime.owner_fence().map_err(|_| AttackError::StaleOwner)?;
        self.schedule(runtime, fence, stamp, source, proposal, world, map, now)
    }
    fn pick(
        &self,
        runtime: &ChannelRuntimeV1,
        source: &S,
        proposal: &ProfileAbilityProposal,
        world: &mut (impl ChainWorldReader + ?Sized),
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
            let f = world
                .attack_facts(
                    runtime,
                    proposal.issuer,
                    actor,
                    session,
                    source.element(),
                    stamp,
                )
                .ok_or(AttackError::MissingCombatFacts)?;
            if f.issuer != proposal.issuer
                || f.target != actor
                || f.session != session
                || f.revision == 0
            {
                return Err(AttackError::MissingCombatFacts);
            }
            // Crystal combat.cpp:1980-1984 checks combat permission and sight for
            // secondary hops, not invisibility. Initial target acquisition still
            // requires visibility; current protection gates apply to every hop.
            if (actor != proposal.target || f.visible)
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
    pub fn run_due<D: ChainDamage<S>>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &mut ScopeRuntimeFence,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &D,
        world: &mut D::World<'_>,
        map: &mut dyn ChainMapFacts,
        clock: &impl OwnerClock,
    ) -> ChainPulseResults<S::Outcome> {
        self.run_due_inner(
            runtime,
            Some(fence),
            None,
            clock,
            |runtime, fence, stamp, pulse, from, now| {
                Self::apply_due_pulse(
                    runtime, fence, stamp, vitals, damage, world, map, pulse, from, now,
                )
            },
        )
    }
    /// Use caller-issued current work so independently prepared facts bind to this exact turn.
    #[allow(clippy::too_many_arguments)]
    pub fn run_due_current<D: ChainDamage<S>>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &D,
        world: &mut D::World<'_>,
        map: &mut dyn ChainMapFacts,
        clock: &impl OwnerClock,
    ) -> ChainPulseResults<S::Outcome> {
        self.run_due_inner(
            runtime,
            None,
            Some(stamp),
            clock,
            |runtime, fence, stamp, pulse, from, now| {
                Self::apply_due_pulse(
                    runtime, fence, stamp, vitals, damage, world, map, pulse, from, now,
                )
            },
        )
    }
    /// Each sealed due pulse selects its own source/recipient/map facts from the caller's bank.
    /// The callback returns owned readers; it cannot replace or fabricate the pulse.
    #[allow(clippy::too_many_arguments)]
    pub fn run_due_current_with<'a, D, F>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &D,
        clock: &impl OwnerClock,
        mut readers: F,
    ) -> ChainPulseResults<S::Outcome>
    where
        D: ChainDamage<S>,
        F: FnMut(
            &S,
            &ProfileAbilityProposal,
            GameSessionId,
            TilePosition,
            RuntimeWorkStamp,
        ) -> Option<(Box<D::World<'a>>, Box<dyn ChainMapFacts + 'a>)>,
    {
        self.run_due_inner(
            runtime,
            None,
            Some(stamp),
            clock,
            |runtime, fence, stamp, pulse, from, now| {
                let (mut world, mut map) =
                    readers(&pulse.source, &pulse.proposal, pulse.session, from, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?;
                Self::apply_due_pulse(
                    runtime,
                    fence,
                    stamp,
                    vitals,
                    damage,
                    world.as_mut(),
                    map.as_mut(),
                    pulse,
                    from,
                    now,
                )
            },
        )
    }
    fn run_due_inner<F>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        mut fence: Option<&mut ScopeRuntimeFence>,
        supplied_stamp: Option<RuntimeWorkStamp>,
        clock: &impl OwnerClock,
        mut apply: F,
    ) -> ChainPulseResults<S::Outcome>
    where
        F: FnMut(
            &mut ChannelRuntimeV1,
            Option<&ScopeRuntimeFence>,
            RuntimeWorkStamp,
            ChainPulse<S>,
            TilePosition,
            SemanticTimeMicros,
        ) -> Result<S::Outcome, AttackError>,
    {
        let current = current_fence(runtime, fence.as_deref())?;
        if !self.current(runtime, current)
            || supplied_stamp.is_some_and(|stamp| !current.accepts_stamp(stamp))
        {
            return Err(AttackError::StaleOwner);
        }
        // Supplied current authority is checked before pruning or consuming any timer.
        let stamp = match fence.as_deref_mut() {
            Some(fence) => {
                let ordinal = fence
                    .accept_input(self.generation)
                    .map_err(|_| AttackError::StaleOwner)?;
                fence.stamp(ordinal)
            }
            None => supplied_stamp.ok_or(AttackError::StaleOwner)?,
        };
        self.prune(runtime);
        let mut results = Vec::new();
        results
            .try_reserve_exact(self.casts.len())
            .map_err(|_| AttackError::LedgerFull)?;
        let current = current_fence(runtime, fence.as_deref())?;
        let fired = self
            .timers
            .drain_due(clock, current, |a| runtime.contains_live_creature(a));
        for timer in fired {
            let Some(index) = self.casts.iter().position(|c| {
                c.token == timer.occurrence.cast && c.next == usize::from(timer.occurrence.step)
            }) else {
                continue;
            };
            let mut cast = self.casts.remove(index);
            let step = cast.next;
            cast.next += 1;
            let target = &cast.targets[step];
            let proposal = step_proposal(&cast.proposal, target.actor, step)?;
            let pulse = ChainPulse {
                source: cast.source.clone(),
                proposal,
                session: target.session,
            };
            let outcome = apply(
                runtime,
                fence.as_deref(),
                stamp,
                pulse,
                target.from,
                clock.now(),
            );
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
                        current_fence(runtime, fence.as_deref())?,
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
            }
            self.casts.push(cast);
        }
        Ok(results)
    }
    #[allow(clippy::too_many_arguments)]
    fn apply_due_pulse<D: ChainDamage<S>>(
        runtime: &mut ChannelRuntimeV1,
        fence: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        damage: &D,
        world: &mut D::World<'_>,
        map: &mut dyn ChainMapFacts,
        pulse: ChainPulse<S>,
        from: TilePosition,
        now: SemanticTimeMicros,
    ) -> Result<S::Outcome, AttackError> {
        let to = runtime
            .read_actor_position(pulse.proposal.target)
            .map_err(|_| AttackError::StaleTarget)?
            .position();
        let current_to = TilePosition {
            x: to.x,
            y: to.y,
            floor: to.floor,
        };
        match map.sight(runtime, from, current_to, stamp) {
            None => Err(AttackError::MissingCombatFacts),
            Some(false) => Err(AttackError::OutOfRange),
            Some(true) => match fence {
                Some(fence) => {
                    damage.consume_chain_pulse(runtime, fence, stamp, vitals, pulse, world, now)
                }
                None => {
                    damage.consume_chain_pulse_current(runtime, stamp, vitals, pulse, world, now)
                }
            },
        }
    }
    /// Actual next-step deadline metadata; reading it grants no callback authority.
    pub fn next_due_time(&self) -> Option<SemanticTimeMicros> {
        self.casts
            .iter()
            .filter_map(|cast| {
                let target = cast.targets.get(cast.next)?;
                if cast.next == 0 {
                    Some(cast.start)
                } else {
                    cast.start
                        .get()
                        .checked_add(target.delay)
                        .map(SemanticTimeMicros::from_micros)
                }
            })
            .min()
    }
    /// Exact frozen next recipients with source metadata for fresh owner-fact reads.
    /// Only `run_due` can produce the sealed pulse that permits damage execution.
    pub fn pending_sources(&self) -> Result<Vec<PendingChainSource<S>>, AttackError> {
        self.casts
            .iter()
            .filter(|cast| cast.next < cast.targets.len())
            .map(|cast| {
                let target = &cast.targets[cast.next];
                let due = if cast.next == 0 {
                    cast.start.get()
                } else {
                    cast.start
                        .get()
                        .checked_add(target.delay)
                        .ok_or(AttackError::NumericOverflow)?
                };
                Ok(PendingChainSource {
                    source: cast.source.clone(),
                    proposal: step_proposal(&cast.proposal, target.actor, cast.next)?,
                    session: target.session,
                    due: SemanticTimeMicros::from_micros(due),
                    from: target.from,
                })
            })
            .collect()
    }
    pub fn receipts(&self, token: u64) -> Option<&[Result<S::Outcome, AttackError>]> {
        self.casts
            .iter()
            .find(|c| c.token == token)
            .map(|c| c.receipts.as_slice())
    }
}
fn current_fence<'a>(
    runtime: &'a ChannelRuntimeV1,
    provided: Option<&'a ScopeRuntimeFence>,
) -> Result<&'a ScopeRuntimeFence, AttackError> {
    provided.map_or_else(
        || runtime.owner_fence().map_err(|_| AttackError::StaleOwner),
        Ok,
    )
}
fn step_proposal(
    parent: &ProfileAbilityProposal,
    target: ExactActorRef,
    step: usize,
) -> Result<ProfileAbilityProposal, AttackError> {
    let mut proposal = parent.clone();
    proposal.target = target;
    proposal.occurrence = crate::ability::chain_step_occurrence(&proposal.occurrence, step)
        .map_err(|_| AttackError::InvalidPlan)?;
    proposal.intent =
        AbilityIntent::normalize(ProposalSource::Ai, &atom(proposal.issuer), &[&atom(target)])
            .map_err(|_| AttackError::InvalidPlan)?;
    Ok(proposal)
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

pub type ChainPulseResults<O> = Result<Vec<(u64, usize, Result<O, AttackError>)>, AttackError>;

#[allow(clippy::expect_used)]
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
        let replacement = full
            .iter()
            .position(|key| *key == (63, 15))
            .expect("qualified fixture");
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
