//! One private current-caster DATA snapshot in an existing real command read.
use super::*;
use crate::durability::spell_item_transaction::SpellItemAuthority;
use crate::foundation::{CommandRef, GameSessionId};
use oteryn_gameplay_entities::carrier::soul_war_player_source::{
    CurrentSoulWarPlayerSourceProof, SoulWarPlayerSourceFacts, SoulWarPlayerSourceOrigin,
    SoulWarRealmMonster as Realm, source_proof_seal,
};
use sqlx::{Postgres, Transaction};

struct PlayerSourceSnapshot {
    binding: taints::Binding,
    client: [u8; 32],
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    command: Option<CommandRef>,
    now: u64,
    position: MovementLocalPosition,
    facts: SoulWarPlayerSourceFacts,
}
impl source_proof_seal::Sealed for PlayerSourceSnapshot {}
impl CurrentSoulWarPlayerSourceProof for PlayerSourceSnapshot {
    fn current_facts(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now: u64,
    ) -> Option<SoulWarPlayerSourceFacts> {
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        (self.binding == taints::binding(runtime)
            && self.client == runtime.content_pin().client_artifact_digest()
            && self.actor == actor
            && self.session == session
            && self.command == command
            && self.now == now
            && self.character == *lease.character_id().as_bytes()
            && self.lease == lease.generation()
            && runtime.read_actor_position(actor).ok()?.position() == self.position
            && (self.facts.origin != SoulWarPlayerSourceOrigin::Combat
                || runtime
                    .player_control_facts(actor, session)
                    .ok()?
                    .control_loss
                    .is_none()))
        .then_some(self.facts)
    }
}

/// Reuse the same current Quest read that prepared this native monster pass.
/// The proof grants only caster DATA; reflection retains its independent stream
/// identity and never borrows a player command or changes the physical caster.
pub(super) fn publish_reflection_sources(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    geometry: taints::Geometry,
    players: &[crate::durability::quest_state::soul_war::CurrentSoulWarTaintFacts],
    now: u64,
) -> Result<(), ()> {
    let mut by_actor = std::collections::BTreeMap::new();
    for fact in players {
        if by_actor
            .insert(fact.actor.placement_identity(), fact)
            .is_some()
        {
            return Err(());
        }
    }
    for (actor, session) in taints::source_player_census(runtime, states)? {
        let position = runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position();
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| ())?;
        let realm = realm(Some(geometry), position);
        let taints = if realm.is_some() {
            let Some(fact) = by_actor.get(&actor.placement_identity()).filter(|fact| {
                fact.actor == actor
                    && fact.session == session
                    && &fact.character == lease.character_id().as_bytes()
            }) else {
                continue;
            };
            fact.count
        } else {
            0
        };
        let state = states.get(runtime, actor, session).ok_or(())?;
        let proof = PlayerSourceSnapshot {
            binding: taints::binding(runtime),
            client: runtime.content_pin().client_artifact_digest(),
            actor,
            session,
            character: *lease.character_id().as_bytes(),
            lease: lease.generation(),
            command: None,
            now,
            position,
            facts: SoulWarPlayerSourceFacts {
                vocation: state.facts.vocation,
                taints,
                realm,
                before_revision: state.revision(),
                origin: SoulWarPlayerSourceOrigin::Reflection,
            },
        };
        runtime
            .install_current_soul_war_player_source(actor, session, None, now, &proof)
            .map_err(|_| ())?;
    }
    Ok(())
}
fn realm(geometry: Option<taints::Geometry>, position: MovementLocalPosition) -> Option<Realm> {
    let geometry = geometry?;
    if !geometry.contains(position) {
        return None;
    }
    if matches!(geometry, taints::Geometry::Boss(_)) {
        return Some(Realm::DreadfulHarvester);
    }
    let p = geometry.source_position(position)?;
    let inside = |(x0, y0, z0, x1, y1, z1): (i32, i32, i16, i32, i32, i16)| {
        (x0..=x1).contains(&p.x) && (y0..=y1).contains(&p.y) && (z0..=z1).contains(&p.floor)
    };
    [
        ((33982, 30981, 9, 34051, 31110, 11), Realm::Brachiodemon),
        ((33873, 30994, 8, 33968, 31150, 9), Realm::BonySeaDevil),
        ((33814, 31819, 3, 33907, 31920, 7), Realm::CloakOfTerror),
        // Accepted Q2a repairs the donor's reversed Rotten Wasteland x corners.
        ((33901, 30986, 11, 33980, 31105, 12), Realm::BranchyCrawler),
        ((33877, 31164, 9, 33991, 31241, 13), Realm::ManyFaces),
    ]
    .into_iter()
    .find_map(|(bounds, kind)| inside(bounds).then_some(kind))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn publish_command_source(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    active: &crate::content::ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    now: u64,
) -> Result<(), crate::durability::DurabilityError> {
    let refuse = || crate::durability::DurabilityError::Unavailable;
    let native = active.native_gameplay().ok_or_else(refuse)?;
    if !qualifies_source(runtime, native) {
        return Ok(());
    }
    let session = authority.game_session_id();
    let command = authority.command();
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| refuse())?
        .position();
    let geometry = if let Some(role) = room
        .pinned_activity_source_room()
        .filter(|role| taints::boss_room(*role))
    {
        if runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
        {
            return Err(refuse());
        }
        Some(taints::Geometry::Boss(role))
    } else {
        active
            .qualify_published_world_context(room, runtime.content_pin())
            .map(|proof| taints::Geometry::World(proof.source_frame()))
    };
    let realm = realm(geometry, position);
    let taints = if realm.is_some() {
        crate::durability::spell_item_transaction::read_current_soul_war_caster_taints_in_transaction(
            tx,authority,runtime,actor).await.map_err(|_|refuse())?.count
    } else {
        0
    };
    let state = states.get(runtime, actor, session).ok_or_else(refuse)?;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(|_| refuse())?;
    if lease.character_id().as_bytes() != &authority.character_id_bytes() {
        return Err(refuse());
    }
    let proof = PlayerSourceSnapshot {
        binding: taints::binding(runtime),
        client: runtime.content_pin().client_artifact_digest(),
        actor,
        session,
        character: *lease.character_id().as_bytes(),
        lease: lease.generation(),
        command: Some(command),
        now,
        position,
        facts: SoulWarPlayerSourceFacts {
            vocation: state.facts.vocation,
            taints,
            realm,
            before_revision: state.revision(),
            origin: SoulWarPlayerSourceOrigin::Combat,
        },
    };
    runtime
        .install_current_soul_war_player_source(actor, session, Some(command), now, &proof)
        .map_err(|_| refuse())
}

/// Refresh only actual periodic lineage sources, once per current field/condition pass.
/// The caller gathers DATA before any HP publication; the canonical reader proves
/// each visible session independently in this same scope transaction.
#[allow(clippy::too_many_arguments)]
async fn publish_sources(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    active: &crate::content::ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    sources: &[ExactActorRef],
    combat: Option<&[(ExactActorRef, GameSessionId, CommandRef)]>,
    now: u64,
) -> Result<(), crate::durability::DurabilityError> {
    let refuse = || crate::durability::DurabilityError::Unavailable;
    let Some(native) = active.native_gameplay() else {
        return Ok(());
    };
    if !qualifies_source(runtime, native) || sources.is_empty() {
        return Ok(());
    }
    let geometry = room
        .pinned_activity_source_room()
        .filter(|role| taints::boss_room(*role))
        .map(taints::Geometry::Boss)
        .or_else(|| {
            active
                .qualify_published_world_context(room, runtime.content_pin())
                .map(|proof| taints::Geometry::World(proof.source_frame()))
        });
    // A transient index of this held state's actual identities avoids an
    // O(source-count * present-player-count) scan in a dense World pass.
    let wanted: std::collections::BTreeSet<_> = sources
        .iter()
        .map(|actor| actor.placement_identity())
        .collect();
    let mut present = std::collections::BTreeMap::new();
    for (actor, session, state) in &states.actors {
        let key = actor.placement_identity();
        if !wanted.contains(&key) || runtime.player_control_facts(*actor, *session).is_err() {
            continue;
        }
        if present
            .insert(
                key,
                (*actor, *session, state.facts.vocation, state.revision()),
            )
            .is_some()
        {
            return Err(refuse());
        }
    }
    let mut actors = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for &actor in sources {
        let key = actor.placement_identity();
        let Some(&(actual, session, _, _)) = present.get(&key) else {
            continue;
        };
        if actual != actor {
            return Err(refuse());
        }
        if seen.insert(key) {
            actors.push((actor, session));
        }
    }
    for page in actors.chunks(64) {
        let facts =
            crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                tx, root, authority, runtime, page,
            )
            .await?;
        for fact in facts {
            let &(actual, session, vocation, before_revision) = present
                .get(&fact.actor.placement_identity())
                .ok_or_else(refuse)?;
            if actual != fact.actor || session != fact.session {
                return Err(refuse());
            }
            let position = runtime
                .read_actor_position(fact.actor)
                .map_err(|_| refuse())?
                .position();
            let lease = runtime
                .bound_attacker_lease(fact.actor, fact.session)
                .map_err(|_| refuse())?;
            if lease.character_id().as_bytes() != &fact.character {
                return Err(refuse());
            }
            let command = combat.and_then(|sources| {
                sources
                    .iter()
                    .find(|(actor, session, _)| *actor == fact.actor && *session == fact.session)
                    .map(|(_, _, command)| *command)
            });
            if combat.is_some() && command.is_none() {
                return Err(refuse());
            }
            let proof = PlayerSourceSnapshot {
                binding: taints::binding(runtime),
                client: runtime.content_pin().client_artifact_digest(),
                actor: fact.actor,
                session: fact.session,
                character: fact.character,
                lease: lease.generation(),
                command,
                now,
                position,
                facts: SoulWarPlayerSourceFacts {
                    vocation,
                    taints: fact.count,
                    realm: realm(geometry, position),
                    before_revision,
                    origin: if combat.is_some() {
                        SoulWarPlayerSourceOrigin::Combat
                    } else {
                        SoulWarPlayerSourceOrigin::Condition
                    },
                },
            };
            runtime
                .install_current_soul_war_player_source(
                    fact.actor,
                    fact.session,
                    command,
                    now,
                    &proof,
                )
                .map_err(|_| refuse())?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn publish_condition_sources(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    active: &crate::content::ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    sources: &[ExactActorRef],
    now: u64,
) -> Result<(), crate::durability::DurabilityError> {
    publish_sources(
        tx, root, authority, active, room, runtime, states, sources, None, now,
    )
    .await
}

/// Snapshot DATA for the real held attack lineages; selecting/paying a swing
/// remains exclusively the existing attack owner below this read.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn publish_auto_sources(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    active: &crate::content::ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    sources: &[(ExactActorRef, GameSessionId, CommandRef)],
    now: u64,
) -> Result<(), crate::durability::DurabilityError> {
    let actors: Vec<_> = sources.iter().map(|(actor, _, _)| *actor).collect();
    publish_sources(
        tx,
        root,
        authority,
        active,
        room,
        runtime,
        states,
        &actors,
        Some(sources),
        now,
    )
    .await
}
