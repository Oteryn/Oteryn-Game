//! The source spell party view joins a same-transaction strong World read with current
//! colocated physical actors/player states. It never authorizes membership from Slot cache.
use super::actor_spell::ChannelSpellStates;
use crate::durability::{
    admission_journal::party_target_binding,
    spell_item_transaction::{SpellItemAuthority, SpellItemError, SpellItemScopeAuthority},
    world_party,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, PartyPresence,
    PartySnapshot,
};
use crate::spell::{
    cast::PlayerSpellState,
    chain::{ChainCreature, TilePosition},
    harmony::SereneWorld,
    native_actor_states::PartyMemberFacts,
    party::PartyWorld,
};
use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub(crate) struct SourcePartyWorld {
    source: PartySnapshot,
    caster: ChainCreature,
    party: Option<Vec<ChainCreature>>,
    physical: Vec<(
        ExactActorRef,
        GameSessionId,
        MovementPositionSnapshot,
        PlayerSpellState,
    )>,
    caster_facts: PartyMemberFacts,
    member_facts: Vec<PartyMemberFacts>,
    adjacent_masterless_monsters: u32,
}
impl SourcePartyWorld {
    pub(crate) fn caster_facts(&self) -> PartyMemberFacts {
        self.caster_facts
    }
    pub(crate) fn member_facts(&self) -> &[PartyMemberFacts] {
        &self.member_facts
    }
    pub(crate) fn source(&self) -> &PartySnapshot {
        &self.source
    }
    pub(crate) fn matches_current_world(&self, current: &PartySnapshot) -> bool {
        self.source.world == current.world
            && self.source.character == current.character
            && self.source.party_id == current.party_id
            && self.source.revision == current.revision
            && self.source.leader == current.leader
            && self.source.members == current.members
    }

    pub(crate) fn player_predecessors(
        &self,
    ) -> Vec<(ExactActorRef, GameSessionId, PlayerSpellState)> {
        self.physical
            .iter()
            .map(|(a, s, _, state)| (*a, *s, state.clone()))
            .collect()
    }
    pub(crate) fn actor_for_id(&self, id: u64) -> Option<(ExactActorRef, GameSessionId)> {
        self.physical
            .iter()
            .find(|(actor, _, _, _)| u64::from(actor.actor_local_id()) == id)
            .map(|(a, s, _, _)| (*a, *s))
    }
    pub(crate) fn validate_physical(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
    ) -> Result<(), SpellItemError> {
        for (actor, session, position, state) in &self.physical {
            if runtime.read_actor_position(*actor) != Ok(*position)
                || states.get(runtime, *actor, *session) != Some(state)
            {
                return Err(SpellItemError::Rejected(
                    "source party physical predecessor changed",
                ));
            }
        }
        Ok(())
    }
}
impl PartyWorld for SourcePartyWorld {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }
    fn party(&self) -> Option<&[ChainCreature]> {
        self.party.as_deref()
    }
}
impl SereneWorld for SourcePartyWorld {
    fn monk(&self) -> &ChainCreature {
        &self.caster
    }
    fn party(&self) -> Option<&[ChainCreature]> {
        self.party.as_deref()
    }
    fn adjacent_creatures(&self) -> u32 {
        self.adjacent_masterless_monsters
    }
}
fn creature(actor: ExactActorRef, position: MovementPositionSnapshot) -> ChainCreature {
    let p = position.position();
    ChainCreature {
        id: u64::from(actor.actor_local_id()),
        actor: crate::spell::combat_execution::actor_atom(actor),
        position: TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
    }
}
fn facts(
    actor: ExactActorRef,
    state: &PlayerSpellState,
    position: MovementPositionSnapshot,
    party: Option<u64>,
) -> PartyMemberFacts {
    let p = position.position();
    let vitals = state.vitals();
    PartyMemberFacts {
        id: u64::from(actor.actor_local_id()),
        party_id: party,
        vocation: state.source_party_vocation(),
        x: p.x,
        y: p.y,
        floor: p.floor,
        alive: vitals.health > 0,
        removed: false,
        health: u64::from(vitals.health),
    }
}
/// Call with actual runtime -> player locks and source transaction held throughout use.
/// Foreign-Channel members remain authoritative World membership but have no local spell tile.
pub(crate) async fn read_source_party_world_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    caster: ExactActorRef,
    session: GameSessionId,
) -> Result<SourcePartyWorld, SpellItemError> {
    read_source_party_world(
        tx,
        root,
        SourcePartyAuthority::Cast(authority),
        runtime,
        states,
        caster,
        session,
    )
    .await
}

/// Periodic owner turn uses a real scoped SQL authority, without manufacturing a command.
pub(crate) async fn read_source_party_world_for_scope_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    caster: ExactActorRef,
    session: GameSessionId,
) -> Result<SourcePartyWorld, SpellItemError> {
    read_source_party_world(
        tx,
        root,
        SourcePartyAuthority::Scope(authority),
        runtime,
        states,
        caster,
        session,
    )
    .await
}
#[derive(Clone, Copy)]
enum SourcePartyAuthority<'a> {
    Cast(&'a SpellItemAuthority),
    Scope(&'a SpellItemScopeAuthority),
}
#[allow(
    clippy::expect_used,
    reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
)]
async fn read_source_party_world(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: SourcePartyAuthority<'_>,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    caster: ExactActorRef,
    session: GameSessionId,
) -> Result<SourcePartyWorld, SpellItemError> {
    if !runtime
        .player_control_facts(caster, session)
        .is_ok_and(|f| f.control_loss.is_none())
        || matches!(authority, SourcePartyAuthority::Cast(a) if a.game_session_id() != session)
    {
        return Err(SpellItemError::Rejected("party caster owner"));
    }
    let source = match authority {
        SourcePartyAuthority::Cast(a) => {
            world_party::read_world_party_in_transaction(tx, a).await?
        }
        SourcePartyAuthority::Scope(a) => {
            world_party::read_world_party_for_scope_actor_in_transaction(
                tx, root, a, runtime, caster, session,
            )
            .await?
        }
    };
    let caster_state = states
        .get(runtime, caster, session)
        .ok_or(SpellItemError::Rejected("party caster vitals unavailable"))?;
    let caster_position = runtime
        .read_actor_position(caster)
        .map_err(|_| SpellItemError::Rejected("party caster position"))?;
    let mut physical = vec![(caster, session, caster_position, caster_state.clone())];
    let mut party = source.party_id.map(|_| Vec::new());
    // A local occurrence ordinal identifies equality inside this already-qualified group.
    // It is never a public PartyId nor an authorization token.
    let local_party = source.party_id.map(|_| 1);
    let caster_facts = facts(caster, caster_state, caster_position, local_party);
    let mut member_facts = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut source_order = std::collections::BTreeMap::new();
    if source.party_id.is_some() {
        for member in &source.members {
            if member.character == source.character {
                seen.insert(member.character);
                source_order.insert(caster_facts.id, member.invitation_order);
                member_facts.push(caster_facts);
                party
                    .as_mut()
                    .expect("qualified party")
                    .push(creature(caster, caster_position));
            }
        }
    }
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| SpellItemError::Rejected("party current bounded census"))?;
    let mut adjacent = 0_u32;
    for (actor, position, actor_session) in census {
        if actor == caster {
            continue;
        }
        if let Some(member_session) = actor_session {
            if source.party_id.is_none() {
                continue;
            }
            let character = match authority {
                SourcePartyAuthority::Cast(a) => party_target_binding::current_colocated_party_character_in_transaction(tx, root, a, runtime, actor, member_session).await?,
                SourcePartyAuthority::Scope(a) => party_target_binding::current_colocated_party_character_for_scope_in_transaction(tx, root, a, runtime, actor, member_session).await?,
            }.ok_or(SpellItemError::Rejected("party member current journal unavailable"))?;
            let Some(member) = source.members.iter().find(|m| m.character == character) else {
                continue;
            };
            if member.presence == PartyPresence::Offline
                || (member.presence == PartyPresence::Channel
                    && member.channel != Some(*runtime.binding().channel_id().as_bytes()))
            {
                return Err(SpellItemError::Rejected(
                    "party presence projection changed",
                ));
            }
            let state =
                states
                    .get(runtime, actor, member_session)
                    .ok_or(SpellItemError::Rejected(
                        "party member current vitals unavailable",
                    ))?;
            if state.vitals().health == 0 {
                seen.insert(character);
                continue;
            }
            seen.insert(character);
            source_order.insert(u64::from(actor.actor_local_id()), member.invitation_order);
            physical.push((actor, member_session, position, state.clone()));
            member_facts.push(facts(actor, state, position, local_party));
            party
                .as_mut()
                .expect("qualified party")
                .push(creature(actor, position));
        } else if source.party_id.is_some() {
            let p = position.position();
            let c = caster_position.position();
            if p.floor != c.floor || p.x.abs_diff(c.x) > 1 || p.y.abs_diff(c.y) > 1 {
                continue;
            }
            // Pinned Canary counts only monsters without a master. Official8944 supersedes
            // the old source threshold6 with8; the existing Serene owner applies8.
            let snapshot = runtime
                .companion_snapshot(actor)
                .map_err(|_| SpellItemError::Rejected("adjacent monster source policy unknown"))?;
            if snapshot.state.master.is_none() {
                adjacent = adjacent
                    .checked_add(1)
                    .ok_or(SpellItemError::Rejected("Serene count overflow"))?;
            }
        }
    }
    for member in &source.members {
        if member.channel == Some(*runtime.binding().channel_id().as_bytes())
            && !seen.contains(&member.character)
        {
            return Err(SpellItemError::Rejected(
                "colocated party actor unavailable",
            ));
        }
    }
    if source.party_id.is_some() && !seen.contains(&source.character) {
        return Err(SpellItemError::Rejected("caster absent from strong party"));
    }
    // Source selection ties keep invitation order, not slot order or arbitrary roster ordering.
    member_facts.sort_by_key(|f| source_order.get(&f.id).copied().unwrap_or(u64::MAX));
    Ok(SourcePartyWorld {
        source,
        caster: creature(caster, caster_position),
        party,
        physical,
        caster_facts,
        member_facts,
        adjacent_masterless_monsters: adjacent,
    })
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Actual admission/current owner producer, committed before the projection is cached.
    pub(crate) async fn refresh_source_party_presence(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let mut runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        if runtime.assert_actor_spell_unreserved(actor).is_err()
            || states.has_pending_spell_commit(actor, session)
        {
            return false;
        }
        let Ok(view) = self
            .root
            .refresh_current_world_party_presence(
                self.character,
                self.holder,
                &runtime,
                actor,
                session,
            )
            .await
        else {
            return false;
        };
        runtime.project_world_party(actor, session, view).is_ok()
    }
    /// Idempotent source World deadline pass; no player command or invented caster identity.
    pub(crate) async fn drain_source_party_expiry(&self) -> Option<usize> {
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        runtime.owner_fence().ok()?;
        self.root
            .drain_world_party_expiry(
                self.character,
                self.holder,
                crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                ),
                binding.scope_generation().get(),
            )
            .await
            .ok()
    }

    /// Source absence hides presence, never removes an actor or infers a clear combat lock.
    pub(crate) async fn drain_source_party_offline_presence(&self) -> Option<usize> {
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        runtime.owner_fence().ok()?;
        self.root
            .drain_world_party_offline_presence(
                self.character,
                self.holder,
                crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                ),
                binding.scope_generation().get(),
            )
            .await
            .ok()
    }
    /// Evaluate real World party membership on the actual owner cycle. Unknown SQL or
    /// member presence skips evaluation; it never substitutes a solo-party decision.
    pub(crate) async fn refresh_source_party_serene(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, oteryn_protocol_oteryn::actor_spell::ActorVitals)> {
        use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        runtime.assert_actor_spell_unreserved(actor).ok()?;
        if states.has_pending_spell_commit(actor, session) {
            return None;
        }
        let state = states.get(&runtime, actor, session)?;
        if !matches!(
            state.source_party_vocation(),
            crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
        ) {
            return None;
        }
        // Legacy baseline has its own explicitly scoped solo behavior. This is the native source path.
        self.active_generation?.native_gameplay()?;
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, &*runtime, &mut *states);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, states) = ctx;
                let binding = runtime.binding();
                let scope = crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                );
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    owner.root,
                    owner.character,
                    owner.holder,
                    scope,
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let world = read_source_party_world_for_scope_in_transaction(
                    &mut tx, owner.root, &authority, runtime, states, actor, session,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                world
                    .validate_physical(runtime, states)
                    .map_err(|_| DurabilityError::Unavailable)?;
                let state = states
                    .get_mut(runtime, actor, session)
                    .ok_or(DurabilityError::Unavailable)?;
                let changed = state
                    .tick_with_world(owner.owner_now(), &world)
                    .map_err(|_| DurabilityError::Unavailable)?;
                let outcome = changed.then(|| (state.revision(), state.vitals()));
                // Read-only SQL transaction owns all strong membership locks until this actual
                // uninterrupted physical update completes. No durable cast/payment is invented.
                tx.rollback().await?;
                Ok(outcome)
            })
        })
        .await
        .ok()
        .flatten()
    }
}
